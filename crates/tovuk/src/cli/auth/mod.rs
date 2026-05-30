use super::{
    api_commands::api_request,
    args::CliOptions,
    constants::{DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS},
    errors::{Result, agent_error},
    project::{encode_component, number_alias, open_url, progress, string_alias, string_field},
};
use reqwest::Method;
use serde_json::Value;
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
    login_and_store(cli)
}

pub(crate) fn login(cli: &CliOptions) -> Result<()> {
    if !cli.token.trim().is_empty() {
        write_session_token(cli.token.trim())?;
        println!("saved Tovuk session token");
        return Ok(());
    }
    login_and_store(cli)?;
    Ok(())
}

fn login_and_store(cli: &CliOptions) -> Result<String> {
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
    progress(cli, "opened browser login");
    progress(cli, &login_wait_message(&user_code));

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
    write_session_token(&token)?;
    progress(cli, &logged_in_message(&session));
    Ok(token)
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
    format!(
        "logged in as {}",
        if email.is_empty() {
            "Tovuk user"
        } else {
            &email
        }
    )
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
