use serde_json::{Value, json};

use super::LoginStarted;
use crate::cli::utils::optional_string_field;

#[derive(Debug)]
/// Machine-readable event emitted when browser login starts.
pub(super) struct LoginStartedPayload(Value);

impl From<&LoginStarted> for LoginStartedPayload {
    fn from(value: &LoginStarted) -> Self {
        let verification_uri = optional_string_field(&value.start, "verificationUri");
        return Self(json!({
            "event": "login_started",
            "ok": true,
            "status": "waiting_for_browser_login",
            "login_url": value.login_url,
            "verification_uri": optional_string(verification_uri),
            "user_code": optional_string(value.user_code.clone()),
            "expires_in_seconds": value.timing.expiry.as_secs(),
            "poll_interval_seconds": value.timing.poll_interval.as_secs(),
            "agent_instruction": "Open login_url, complete Tovuk browser login, then keep waiting for this command to finish. Stdout remains reserved for the final command JSON.",
        }));
    }
}

#[derive(Debug)]
/// Machine-readable successful login payload.
pub(super) struct LoginSuccessPayload(Value);

impl From<(&str, Option<&str>)> for LoginSuccessPayload {
    fn from(value: (&str, Option<&str>)) -> Self {
        let (status, email) = value;
        return Self(json!({
            "ok": true,
            "status": status,
            "email": optional_string(email.map(str::to_owned)),
            "agent_instruction": "Tovuk session is saved. Continue with the original command.",
        }));
    }
}

impl From<LoginStartedPayload> for Value {
    #[inline]
    fn from(value: LoginStartedPayload) -> Self {
        return value.0;
    }
}

impl From<LoginSuccessPayload> for Value {
    #[inline]
    fn from(value: LoginSuccessPayload) -> Self {
        return value.0;
    }
}

/// Converts an optional string into a JSON string or null.
fn optional_string(value: Option<String>) -> Value {
    return value.map_or(Value::Null, Value::String);
}

#[cfg(test)]
#[path = "payload_tests.rs"]
mod tests;
