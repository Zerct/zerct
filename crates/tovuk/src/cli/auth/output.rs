use serde_json::Value;

use crate::cli::{
    args::CliOptions,
    errors::{Result, internal_error, print_json},
    project::{progress, string_field},
};

use super::payload::{login_started_payload, login_success_payload};

pub(super) fn logged_in_message(session: &Value) -> String {
    let email = string_field(session, "email");
    format!("logged in as {}", user_or_fallback(&email))
}

pub(super) fn print_login_started(
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

pub(super) fn print_login_success(cli: &CliOptions, status: &str, email: &str) -> Result<()> {
    if cli.output.json {
        return print_json(&login_success_payload(status, email));
    }
    if status == "saved" {
        println!("saved Tovuk session token");
        return Ok(());
    }
    Ok(())
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
