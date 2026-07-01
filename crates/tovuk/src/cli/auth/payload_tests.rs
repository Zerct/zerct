use serde_json::{Value, json};

use super::{login_started_payload, login_success_payload};

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
        Some("TOVUK-123"),
        900,
        2,
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
fn login_started_payload_uses_required_timing_values() {
    let payload = login_started_payload(&json!({}), "https://tovuk.com/login", None, 900, 2);

    assert_eq!(payload["verification_uri"], Value::Null);
    assert_eq!(payload["user_code"], Value::Null);
    assert_eq!(payload["expires_in_seconds"], 900);
    assert_eq!(payload["poll_interval_seconds"], 2);
}

#[test]
fn login_success_payload_excludes_session_token() {
    let payload = login_success_payload("logged_in", Some("ada@example.com"));

    assert_eq!(payload["ok"], true);
    assert_eq!(payload["status"], "logged_in");
    assert_eq!(payload["email"], "ada@example.com");
    assert!(payload.get("token").is_none());
}
