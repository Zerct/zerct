use super::{
    api_commands::api_request,
    args::CliOptions,
    constants::{DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS},
    errors::{Result, agent_error, internal_error, print_json},
    project::{encode_component, number_alias, open_url, progress, string_alias, string_field},
};
use reqwest::Method;
use serde_json::{Value, json};
use std::{
    thread,
    time::{Duration, Instant},
};

mod keychain;
mod token_store;

use token_store::{read_stored_token, write_session_token};

pub(crate) fn read_or_login_token(cli: &CliOptions) -> Result<String> {
    let token = read_stored_token(cli);
    if !token.is_empty() {
        return Ok(token);
    }
    login_and_store(cli).map(|session| session.token)
}

pub(crate) fn login(cli: &CliOptions) -> Result<()> {
    if !cli.token.trim().is_empty() {
        write_session_token(cli.token.trim())?;
        print_login_success(cli, "saved", "")?;
        return Ok(());
    }
    let session = login_and_store(cli)?;
    print_login_success(cli, "logged_in", &session.email)?;
    Ok(())
}

struct StoredSession {
    token: String,
    email: String,
}

fn login_and_store(cli: &CliOptions) -> Result<StoredSession> {
    let start = api_request(cli, Method::POST, "/v1/login/device", None, None)?;
    let login_url = string_alias(&start, &["loginUrl", "login_url"]);
    let user_code = string_alias(&start, &["userCode", "user_code"]);
    let device_code = string_alias(&start, &["deviceCode", "device_code"]);
    if login_url.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a browser URL.",
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.output.json,
        ));
    }
    open_url(&login_url);
    print_login_started(cli, &start, &login_url, &user_code)?;

    let session = poll_login(cli, &device_code, &start)?;
    let token = string_field(&session, "token");
    if token.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a session token.",
            "Run `tovuk login` again and complete the browser login.",
            cli.output.json,
        ));
    }
    let email = string_field(&session, "email");
    write_session_token(&token)?;
    progress(cli, &logged_in_message(&session));
    Ok(StoredSession { token, email })
}

fn login_wait_message(user_code: &str) -> String {
    format!(
        "waiting for browser login code {}",
        if user_code.is_empty() {
            "TOVUK"
        } else {
            user_code
        }
    )
}

fn logged_in_message(session: &Value) -> String {
    let email = string_field(session, "email");
    format!("logged in as {}", user_or_fallback(&email))
}

fn print_login_started(
    cli: &CliOptions,
    start: &Value,
    login_url: &str,
    user_code: &str,
) -> Result<()> {
    if cli.output.json {
        print_json_event(&login_started_payload(start, login_url, user_code))?;
        return Ok(());
    }
    progress(cli, "opened browser login");
    progress(cli, &login_wait_message(user_code));
    Ok(())
}

fn print_login_success(cli: &CliOptions, status: &str, email: &str) -> Result<()> {
    if cli.output.json {
        return print_json(&login_success_payload(status, email));
    }
    if status == "saved" {
        println!("saved Tovuk session token");
        return Ok(());
    }
    Ok(())
}

fn login_started_payload(start: &Value, login_url: &str, user_code: &str) -> Value {
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

fn login_success_payload(status: &str, email: &str) -> Value {
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

fn user_or_fallback(email: &str) -> &str {
    if email.is_empty() {
        "Tovuk user"
    } else {
        email
    }
}

fn print_json_event(value: &Value) -> Result<()> {
    let source = serde_json::to_string(value).map_err(|error| internal_error(error.to_string()))?;
    eprintln!("{source}");
    Ok(())
}

fn poll_login(cli: &CliOptions, device_code: &str, start: &Value) -> Result<Value> {
    if device_code.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a device code.",
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.output.json,
        ));
    }

    let expires_seconds = number_alias(start, &["expiresInSeconds", "expires_in_seconds"])
        .unwrap_or(DEFAULT_LOGIN_EXPIRES_SECONDS);
    let mut interval_seconds = number_alias(start, &["intervalSeconds", "interval_seconds"])
        .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS);
    let deadline = Instant::now() + Duration::from_secs(expires_seconds);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(interval_seconds));
        let response = api_request(
            cli,
            Method::GET,
            &format!("/v1/login/device/{}", encode_component(device_code)),
            None,
            None,
        )?;
        let status = string_field(&response, "status");
        if status == "complete" {
            return Ok(response);
        }
        if status == "expired" {
            return login_expired(cli);
        }
        interval_seconds = number_alias(&response, &["intervalSeconds", "interval_seconds"])
            .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS)
            .max(DEFAULT_LOGIN_INTERVAL_SECONDS);
    }
    login_expired(cli)
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
