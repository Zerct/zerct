use serde_json::{Value, json};

use crate::cli::utils::optional_string_alias;

pub(super) fn login_started_payload(
    start: &Value,
    login_url: &str,
    user_code: Option<&str>,
    expires_seconds: u64,
    interval_seconds: u64,
) -> Value {
    let verification_uri = optional_string_alias(start, &["verificationUri", "verification_uri"]);
    json!({
        "event": "login_started",
        "ok": true,
        "status": "waiting_for_browser_login",
        "login_url": login_url,
        "verification_uri": optional_string(verification_uri),
        "user_code": optional_string(user_code.map(str::to_owned)),
        "expires_in_seconds": expires_seconds,
        "poll_interval_seconds": interval_seconds,
        "agent_instruction": "Open login_url, complete Tovuk browser login, then keep waiting for this command to finish. Stdout remains reserved for the final command JSON.",
    })
}

pub(super) fn login_success_payload(status: &str, email: Option<&str>) -> Value {
    json!({
        "ok": true,
        "status": status,
        "email": optional_string(email.map(str::to_owned)),
        "agent_instruction": "Tovuk session is saved. Continue with the original command.",
    })
}

fn optional_string(value: Option<String>) -> Value {
    value.map_or(Value::Null, Value::String)
}

#[cfg(test)]
#[path = "payload_tests.rs"]
mod tests;
