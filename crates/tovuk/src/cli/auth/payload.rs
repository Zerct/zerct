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
mod tests {
    use super::{login_started_payload, login_success_payload};
    use crate::cli::constants::{DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS};
    use serde_json::{Value, json};

    #[test]
    fn login_started_payload_is_agent_readable_without_standalone_device_code() {
        let start = json!({
            "loginUrl": "https://tovuk.com/login?device_code=secret",
            "verificationUri": "https://tovuk.com/login",
            "userCode": "TOVUK-123",
            "deviceCode": "secret-device-code",
            "expiresInSeconds": 900,
            "intervalSeconds": 2
        });

        let payload = login_started_payload(
            &start,
            "https://tovuk.com/login?device_code=secret",
            "TOVUK-123",
        );

        assert_eq!(payload["event"], "login_started");
        assert_eq!(payload["status"], "waiting_for_browser_login");
        assert_eq!(
            payload["login_url"],
            "https://tovuk.com/login?device_code=secret"
        );
        assert_eq!(payload["verification_uri"], "https://tovuk.com/login");
        assert_eq!(payload["user_code"], "TOVUK-123");
        assert_eq!(payload["expires_in_seconds"], 900);
        assert_eq!(payload["poll_interval_seconds"], 2);
        assert!(payload.get("agent_instruction").is_some());
        assert!(payload.get("deviceCode").is_none());
        assert!(payload.get("device_code").is_none());
    }

    #[test]
    fn login_started_payload_defaults_missing_optional_fields() {
        let payload = login_started_payload(&json!({}), "https://tovuk.com/login", "");

        assert_eq!(payload["verification_uri"], Value::Null);
        assert_eq!(payload["user_code"], Value::Null);
        assert_eq!(payload["expires_in_seconds"], DEFAULT_LOGIN_EXPIRES_SECONDS);
        assert_eq!(
            payload["poll_interval_seconds"],
            DEFAULT_LOGIN_INTERVAL_SECONDS
        );
    }

    #[test]
    fn login_success_payload_excludes_session_token() {
        let payload = login_success_payload("logged_in", "ada@example.com");

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["status"], "logged_in");
        assert_eq!(payload["email"], "ada@example.com");
        assert!(payload.get("token").is_none());
    }
}
