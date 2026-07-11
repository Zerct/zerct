#[cfg(test)]
#[path = "output_tests.rs"]
/// Authentication output tests.
mod tests;

use serde_json::{Value, to_string};

use crate::cli::{
    args::CliOptions,
    errors::{Result, internal_error, print_json, write_stderr_line, write_stdout_line},
    utils::progress,
};

use super::{
    LoginStarted,
    payload::{LoginStartedPayload, LoginSuccessPayload},
};

#[derive(Debug)]
/// Serialized login event destined for standard error.
struct JsonEvent(Value);

impl PrintAuthOutput for JsonEvent {
    fn print(self) -> Result<()> {
        let source = result_or_return!(
            to_string(&self.0).map_err(|error| return internal_error(error.to_string()))
        );
        result_or_return!(
            write_stderr_line(&source).map_err(|error| return internal_error(error.to_string()))
        );
        return Ok(());
    }
}

#[derive(Debug)]
/// Human-readable successful login message.
pub(super) struct LoggedInMessage(String);

impl From<&Value> for LoggedInMessage {
    fn from(value: &Value) -> Self {
        return Self(value.get("email").and_then(Value::as_str).map_or_else(
            || return "logged in".to_owned(),
            |email| format!("logged in as {email}"),
        ));
    }
}

#[derive(Clone, Copy, Debug)]
/// Login-start output action.
pub(super) struct LoginStartedOutput;

impl PrintLoginStarted for LoginStartedOutput {
    fn print(self, cli: &CliOptions, login: &LoginStarted) -> Result<()> {
        if cli.is_json() {
            let payload = Value::from(LoginStartedPayload::from(login));
            return PrintAuthOutput::print(JsonEvent(payload));
        }
        result_or_return!(progress(cli, "opened browser login"));
        let message = LoginWaitMessage::from(login.user_code.as_deref());
        result_or_return!(progress(cli, message.0.as_str()));
        return Ok(());
    }
}

#[derive(Debug)]
/// Human-readable pending login message.
struct LoginWaitMessage(String);

impl From<Option<&str>> for LoginWaitMessage {
    fn from(value: Option<&str>) -> Self {
        return Self(value.map_or_else(
            || return "waiting for browser login".to_owned(),
            |code| format!("waiting for browser login code {code}"),
        ));
    }
}

/// Prints an authentication event on its designated stream.
pub(super) trait PrintAuthOutput {
    /// Prints the event.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or output writing fails.
    fn print(self) -> Result<()>;
}

/// Prints the appropriate login-start output for the selected format.
pub(super) trait PrintLoginStarted {
    /// Prints login-start state for a text or JSON client.
    ///
    /// # Errors
    ///
    /// Returns an error when progress or JSON event output fails.
    fn print(self, cli: &CliOptions, login: &LoginStarted) -> Result<()>;
}

impl From<LoggedInMessage> for String {
    #[inline]
    fn from(value: LoggedInMessage) -> Self {
        return value.0;
    }
}

/// Prints successful login or token-save output.
///
/// # Errors
///
/// Returns an error when JSON serialization or output writing fails.
pub(super) fn print_login_success(
    cli: &CliOptions,
    status: &str,
    email: Option<&str>,
) -> Result<()> {
    if cli.is_json() {
        return print_json(&Value::from(LoginSuccessPayload::from((status, email))));
    }
    if status == "saved" {
        return write_stdout_line("saved Tovuk session token");
    }
    return Ok(());
}
