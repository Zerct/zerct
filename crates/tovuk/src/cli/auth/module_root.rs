#[cfg(test)]
#[path = "auth_tests.rs"]
/// Authentication protocol tests.
mod tests;

/// Operating-system credential store integration.
mod keychain;
/// Authentication progress and result output.
mod output;
/// Authentication JSON payload construction.
mod payload;
/// Session token discovery and persistence.
mod token_store;

use super::{
    ExecuteCommand,
    api_commands::{ApiRequestContent, api_request},
    args::CliOptions,
    errors::{CliError, Result, agent_error, internal_error},
    utils::{encode_component, open_url, optional_string_field, progress},
};
use core::time::Duration;
use reqwest::Method;
use serde_json::Value;
use std::{thread, time::Instant};

use output::{LoggedInMessage, LoginStartedOutput, PrintLoginStarted, print_login_success};
use token_store::{StoredToken, write_session_token};

#[derive(Clone, Copy, Debug)]
/// Explicit login command action.
pub(in crate::cli) struct LoginCommand;

impl ExecuteCommand for LoginCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        if !cli.token().trim().is_empty() {
            result_or_return!(write_session_token(cli.token().trim()));
            result_or_return!(print_login_success(cli, "saved", None));
            return Ok(());
        }
        let session = result_or_return!(login_and_store(cli));
        result_or_return!(print_login_success(
            cli,
            "logged_in",
            session.email.as_deref(),
        ));
        return Ok(());
    }
}

#[derive(Clone, Copy, Debug)]
/// Inputs required to poll a device login.
struct LoginPoll<'input> {
    /// Validated CLI options.
    cli: &'input CliOptions,
    /// Device code returned when login starts.
    device_code: &'input str,
    /// Expiry and polling interval configuration.
    timing: LoginTiming,
}

#[derive(Debug)]
/// Completed device-login response.
struct LoginSession(Value);

impl<'input> TryFrom<LoginPoll<'input>> for LoginSession {
    type Error = CliError;

    fn try_from(value: LoginPoll<'input>) -> Result<Self> {
        let mut poll_interval = value.timing.poll_interval;
        let deadline = result_or_return!(
            Instant::now()
                .checked_add(value.timing.expiry)
                .ok_or_else(|| {
                    return internal_error("Tovuk login expiry exceeded the supported time range.");
                })
        );
        while Instant::now() < deadline {
            thread::sleep(poll_interval);
            let outcome = result_or_return!(PollOutcome::try_from(PollRequest {
                cli: value.cli,
                device_code: value.device_code,
            }));
            let next_interval = match outcome {
                PollOutcome::Complete(response) => return Ok(Self(response)),
                PollOutcome::Continue(interval) => interval,
                PollOutcome::Expired => return login_expired(value.cli).map(Self),
            };
            poll_interval = next_interval.unwrap_or(poll_interval);
        }
        return login_expired(value.cli).map(Self);
    }
}

#[derive(Debug)]
/// Data needed to report a newly started login.
struct LoginStarted {
    /// Browser URL for completing login.
    login_url: String,
    /// Raw public login-start response.
    start: Value,
    /// Validated login timing.
    timing: LoginTiming,
    /// Optional user-facing verification code.
    user_code: Option<String>,
}

#[derive(Clone, Copy, Debug)]
/// Device-login expiry and polling intervals.
struct LoginTiming {
    /// Maximum login lifetime.
    expiry: Duration,
    /// Delay between poll requests.
    poll_interval: Duration,
}

impl TryFrom<(&CliOptions, &Value)> for LoginTiming {
    type Error = CliError;

    fn try_from(value: (&CliOptions, &Value)) -> Result<Self> {
        let (cli, start) = value;
        return Ok(Self {
            expiry: Duration::from_secs(result_or_return!(required_positive_number_field(
                cli,
                start,
                "expiresInSeconds",
                "login expiry seconds",
            ))),
            poll_interval: Duration::from_secs(result_or_return!(required_positive_number_field(
                cli,
                start,
                "intervalSeconds",
                "login poll interval seconds",
            ))),
        });
    }
}

#[derive(Debug)]
/// Result of one device-login poll request.
enum PollOutcome {
    /// Login completed with a session response.
    Complete(Value),
    /// Login remains pending with an optional updated interval.
    Continue(Option<Duration>),
    /// Login expired remotely.
    Expired,
}

impl<'input> TryFrom<PollRequest<'input>> for PollOutcome {
    type Error = CliError;

    fn try_from(value: PollRequest<'input>) -> Result<Self> {
        let response = result_or_return!(api_request(
            value.cli,
            Method::GET,
            &format!("/v1/login/device/{}", encode_component(value.device_code)),
            ApiRequestContent::Anonymous,
        ));
        let next_interval = response
            .get("intervalSeconds")
            .and_then(Value::as_u64)
            .filter(|interval| return *interval > 0)
            .map(Duration::from_secs);
        return match optional_string_field(&response, "status").as_deref() {
            Some("complete") => Ok(Self::Complete(response)),
            Some("expired") => Ok(Self::Expired),
            Some(_) | None => Ok(Self::Continue(next_interval)),
        };
    }
}

#[derive(Clone, Copy, Debug)]
/// Inputs required for one device-login poll request.
struct PollRequest<'input> {
    /// Validated CLI options.
    cli: &'input CliOptions,
    /// Device code identifying the pending login.
    device_code: &'input str,
}

/// Authenticated session persisted by the login flow.
struct StoredSession {
    /// Account email returned by the public login API.
    email: Option<String>,
    /// Session token returned by the public login API.
    token: String,
}

impl TryFrom<(&CliOptions, Value)> for StoredSession {
    type Error = CliError;

    fn try_from(value: (&CliOptions, Value)) -> Result<Self> {
        let (cli, session) = value;
        let Some(token) = optional_string_field(&session, "token") else {
            return Err(agent_error(
                "login_failed",
                "Tovuk login did not return a session token.",
                "Run `tovuk login` again and complete the browser login.",
                cli.output_format(),
            ));
        };
        let email = optional_string_field(&session, "email");
        result_or_return!(write_session_token(&token));
        let logged_in_message = String::from(LoggedInMessage::from(&session));
        result_or_return!(progress(cli, logged_in_message.as_str()));
        return Ok(Self { email, token });
    }
}

/// Starts interactive login, polls it to completion, and stores the session.
///
/// # Errors
///
/// Returns an error when the login protocol, transport, output, or storage fails.
fn login_and_store(cli: &CliOptions) -> Result<StoredSession> {
    let start = result_or_return!(api_request(
        cli,
        Method::POST,
        "/v1/login/device",
        ApiRequestContent::Anonymous,
    ));
    let Some(login_url) = optional_string_field(&start, "loginUrl") else {
        return missing_login_field(cli, "browser URL");
    };
    let user_code = optional_string_field(&start, "userCode");
    let Some(device_code) = optional_string_field(&start, "deviceCode") else {
        return missing_login_field(cli, "device code");
    };
    let timing = result_or_return!(LoginTiming::try_from((cli, &start)));
    open_url(&login_url);
    result_or_return!(PrintLoginStarted::print(
        LoginStartedOutput,
        cli,
        &LoginStarted {
            login_url,
            start,
            timing,
            user_code,
        },
    ));
    let session = result_or_return!(LoginSession::try_from(LoginPoll {
        cli,
        device_code: device_code.as_str(),
        timing,
    }))
    .0;
    return StoredSession::try_from((cli, session));
}

/// Creates the error returned after device login expires.
///
/// # Errors
///
/// Always returns a login-expired error.
fn login_expired(cli: &CliOptions) -> Result<Value> {
    return Err(agent_error(
        "login_expired",
        "Tovuk login expired before it completed.",
        "Run `tovuk login` again and finish the browser login in the newly opened tab.",
        cli.output_format(),
    ));
}

/// Creates the protocol error for a missing required login field.
///
/// # Errors
///
/// Always returns a login protocol error.
fn missing_login_field(cli: &CliOptions, field: &str) -> Result<StoredSession> {
    return Err(agent_error(
        "login_failed",
        format!("Tovuk login did not return a {field}."),
        "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
        cli.output_format(),
    ));
}

/// Returns a stored token or completes interactive login when needed.
///
/// # Errors
///
/// Returns an error when token discovery, login, or persistence fails.
pub(super) fn read_or_login_token(cli: &CliOptions) -> Result<String> {
    let stored_token = result_or_return!(StoredToken::try_from(cli));
    if let Some(token) = Option::<String>::from(stored_token) {
        return Ok(token);
    }
    return login_and_store(cli).map(|session| return session.token);
}

/// Reads a required positive integer from a login response.
///
/// # Errors
///
/// Returns an error when the field is missing, zero, or not an unsigned integer.
fn required_positive_number_field(
    cli: &CliOptions,
    value: &Value,
    key: &str,
    field: &str,
) -> Result<u64> {
    match value
        .get(key)
        .and_then(Value::as_u64)
        .filter(|numeric_value| return *numeric_value > 0)
    {
        Some(field_value) => return Ok(field_value),
        None => {
            return Err(agent_error(
                "login_failed",
                format!("Tovuk login did not return valid {field}."),
                "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
                cli.output_format(),
            ));
        }
    }
}
