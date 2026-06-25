use serde_json::{Value, json};

use crate::cli::{
    constants::{DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS},
    project::{number_alias, string_alias},
};

pub(super) fn login_started_payload(start: &Value, login_url: &str, user_code: &str) -> Value {
    let verification_uri = string_alias(start, &["verificationUri", "verification_uri"]);
    json!({
        "event": "login_started",
        "ok": true,
        "status": "waiting_for_browser_login",
        "login_url": login_url,
        "verification_uri": optional_string(verification_uri),
        "user_code": optional_string(user_code.to_owned()),
        "expires_in_seconds": number_alias(start, &["expiresInSeconds", "expires_in_seconds"])
            .unwrap_or(DEFAULT_LOGIN_EXPIRES_SECONDS),
        "poll_interval_seconds": number_alias(start, &["intervalSeconds", "interval_seconds"])
            .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS),
        "agent_instruction": "Open login_url, complete Tovuk browser login, then keep waiting for this command to finish. Stdout remains reserved for the final command JSON.",
    })
}

pub(super) fn login_success_payload(status: &str, email: &str) -> Value {
    json!({
        "ok": true,
        "status": status,
        "email": optional_string(email.to_owned()),
        "agent_instruction": "Tovuk session is saved. Continue with the original command.",
    })
}

fn optional_string(value: String) -> Value {
    if value.is_empty() {
        Value::Null
    } else {
        Value::String(value)
    }
}

#[cfg(test)]
#[path = "payload_tests.rs"]
mod tests;
