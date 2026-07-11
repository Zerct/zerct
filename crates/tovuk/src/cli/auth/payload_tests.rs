use core::time::Duration;
use serde_json::{Value, json};

use super::super::{LoginStarted, LoginTiming};
use super::{LoginStartedPayload, LoginSuccessPayload};

/// Valid login lifetime used by payload tests.
const LOGIN_EXPIRY_SECONDS: u64 = 0x0384;
/// Valid login polling interval used by payload tests.
const LOGIN_POLL_INTERVAL_SECONDS: u64 = 0x0002;

/// Builds a device-login start record for payload tests.
fn login_started(start: &Value, login_url: &str, user_code: Option<&str>) -> LoginStarted {
    return LoginStarted {
        login_url: login_url.to_owned(),
        start: start.clone(),
        timing: LoginTiming {
            expiry: Duration::from_mins(0x000f),
            poll_interval: Duration::from_secs(0b10),
        },
        user_code: user_code.map(str::to_owned),
    };
}

#[test]
/// Verifies login-start output is complete without exposing a standalone device code.
///
/// # Panics
///
/// Panics when the machine-readable login-start contract changes.
fn login_started_payload_is_agent_readable_without_standalone_device_code() {
    let start = json!({
        "loginUrl": "https://tovuk.com/login?device_code=secret",
        "verificationUri": "https://tovuk.com/login",
        "userCode": "TOVUK-123",
        "deviceCode": "secret-device-code",
        "expiresInSeconds": LOGIN_EXPIRY_SECONDS,
        "intervalSeconds": LOGIN_POLL_INTERVAL_SECONDS
    });

    let payload = Value::from(LoginStartedPayload::from(&login_started(
        &start,
        "https://tovuk.com/login?device_code=secret",
        Some("TOVUK-123"),
    )));

    assert_eq!(
        payload,
        json!({
            "event": "login_started",
            "ok": true,
            "status": "waiting_for_browser_login",
            "login_url": "https://tovuk.com/login?device_code=secret",
            "verification_uri": "https://tovuk.com/login",
            "user_code": "TOVUK-123",
            "expires_in_seconds": LOGIN_EXPIRY_SECONDS,
            "poll_interval_seconds": LOGIN_POLL_INTERVAL_SECONDS,
            "agent_instruction": "Open login_url, complete Tovuk browser login, then keep waiting for this command to finish. Stdout remains reserved for the final command JSON."
        })
    );
}

#[test]
/// Verifies login-start output uses validated timing and explicit null optionals.
///
/// # Panics
///
/// Panics when required timing values or optional-field encoding changes.
fn login_started_payload_uses_required_timing_values() {
    let start = json!({});
    let payload = Value::from(LoginStartedPayload::from(&login_started(
        &start,
        "https://tovuk.com/login",
        None,
    )));

    assert_eq!(
        payload,
        json!({
            "event": "login_started",
            "ok": true,
            "status": "waiting_for_browser_login",
            "login_url": "https://tovuk.com/login",
            "verification_uri": null,
            "user_code": null,
            "expires_in_seconds": LOGIN_EXPIRY_SECONDS,
            "poll_interval_seconds": LOGIN_POLL_INTERVAL_SECONDS,
            "agent_instruction": "Open login_url, complete Tovuk browser login, then keep waiting for this command to finish. Stdout remains reserved for the final command JSON."
        })
    );
}

#[test]
/// Verifies successful login output never exposes the session token.
///
/// # Panics
///
/// Panics when the machine-readable login-success contract changes.
fn login_success_payload_excludes_session_token() {
    let payload = Value::from(LoginSuccessPayload::from((
        "logged_in",
        Some("ada@example.com"),
    )));

    assert_eq!(
        payload,
        json!({
            "ok": true,
            "status": "logged_in",
            "email": "ada@example.com",
            "agent_instruction": "Tovuk session is saved. Continue with the original command."
        })
    );
}
