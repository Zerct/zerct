use super::{
    api_commands::api_request,
    args::CliOptions,
    errors::{Result, agent_error},
    utils::{encode_component, open_url, optional_string_field, progress},
};
use reqwest::Method;
use serde_json::Value;
use std::{
    thread,
    time::{Duration, Instant},
};

mod keychain;
mod output;
mod payload;
mod token_store;

use output::{logged_in_message, print_login_started, print_login_success};
use token_store::{read_stored_token, write_session_token};

pub(crate) fn read_or_login_token(cli: &CliOptions) -> Result<String> {
    if let Some(token) = read_stored_token(cli)? {
        return Ok(token);
    }
    login_and_store(cli).map(|session| session.token)
}

pub(crate) fn login(cli: &CliOptions) -> Result<()> {
    if !cli.token.trim().is_empty() {
        write_session_token(cli.token.trim())?;
        print_login_success(cli, "saved", None)?;
        return Ok(());
    }
    let session = login_and_store(cli)?;
    print_login_success(cli, "logged_in", session.email.as_deref())?;
    Ok(())
}

struct StoredSession {
    token: String,
    email: Option<String>,
}

#[derive(Clone, Copy)]
struct LoginTiming {
    expires_seconds: u64,
    interval_seconds: u64,
}

fn login_and_store(cli: &CliOptions) -> Result<StoredSession> {
    let start = api_request(cli, Method::POST, "/v1/login/device", None, None)?;
    let Some(login_url) = optional_string_field(&start, "loginUrl") else {
        return missing_login_field(cli, "browser URL");
    };
    let user_code = optional_string_field(&start, "userCode");
    let Some(device_code) = optional_string_field(&start, "deviceCode") else {
        return missing_login_field(cli, "device code");
    };
    let timing = login_timing(cli, &start)?;
    open_url(&login_url);
    print_login_started(
        cli,
        &start,
        &login_url,
        user_code.as_deref(),
        timing.expires_seconds,
        timing.interval_seconds,
    )?;

    let session = poll_login(cli, device_code.as_str(), timing)?;
    let Some(token) = optional_string_field(&session, "token") else {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a session token.",
            "Run `tovuk login` again and complete the browser login.",
            cli.output.json,
        ));
    };
    let email = optional_string_field(&session, "email");
    write_session_token(&token)?;
    progress(cli, &logged_in_message(&session));
    Ok(StoredSession { token, email })
}

fn login_timing(cli: &CliOptions, start: &Value) -> Result<LoginTiming> {
    Ok(LoginTiming {
        expires_seconds: required_positive_number_field(
            cli,
            start,
            "expiresInSeconds",
            "login expiry seconds",
        )?,
        interval_seconds: required_positive_number_field(
            cli,
            start,
            "intervalSeconds",
            "login poll interval seconds",
        )?,
    })
}

fn required_positive_number_field(
    cli: &CliOptions,
    value: &Value,
    key: &str,
    field: &str,
) -> Result<u64> {
    match value
        .get(key)
        .and_then(Value::as_u64)
        .filter(|value| *value > 0)
    {
        Some(value) => Ok(value),
        None => Err(agent_error(
            "login_failed",
            format!("Tovuk login did not return valid {field}."),
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.output.json,
        )),
    }
}

fn poll_login(cli: &CliOptions, device_code: &str, timing: LoginTiming) -> Result<Value> {
    let mut interval_seconds = timing.interval_seconds;
    let deadline = Instant::now() + Duration::from_secs(timing.expires_seconds);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(interval_seconds));
        let response = api_request(
            cli,
            Method::GET,
            &format!("/v1/login/device/{}", encode_component(device_code)),
            None,
            None,
        )?;
        match optional_string_field(&response, "status").as_deref() {
            Some("complete") => return Ok(response),
            Some("expired") => return login_expired(cli),
            _ => {}
        }
        if let Some(next_interval) = response
            .get("intervalSeconds")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
        {
            interval_seconds = next_interval;
        }
    }
    login_expired(cli)
}

fn missing_login_field(cli: &CliOptions, field: &str) -> Result<StoredSession> {
    Err(agent_error(
        "login_failed",
        format!("Tovuk login did not return a {field}."),
        "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
        cli.output.json,
    ))
}

fn login_expired(cli: &CliOptions) -> Result<Value> {
    Err(agent_error(
        "login_expired",
        "Tovuk login expired before it completed.",
        "Run `tovuk login` again and finish the browser login in the newly opened tab.",
        cli.output.json,
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{CliOptions, required_positive_number_field};

    #[test]
    fn login_timing_requires_positive_number() {
        let cli = CliOptions::default();
        let message = required_positive_number_field(
            &cli,
            &json!({"intervalSeconds": 0}),
            "intervalSeconds",
            "login poll interval seconds",
        )
        .err()
        .map(|error| error.to_string());

        assert_eq!(
            message.as_deref(),
            Some("Tovuk login did not return valid login poll interval seconds.")
        );
    }

    #[test]
    fn login_timing_accepts_camel_case_protocol_field() {
        let value = required_positive_number_field(
            &CliOptions::default(),
            &json!({"expiresInSeconds": 900}),
            "expiresInSeconds",
            "login expiry seconds",
        );

        assert_eq!(value.ok(), Some(900));
    }
}
