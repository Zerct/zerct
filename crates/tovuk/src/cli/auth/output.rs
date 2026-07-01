use serde_json::Value;

use crate::cli::{
    args::CliOptions,
    errors::{Result, internal_error, print_json},
    utils::{optional_string_field, progress},
};

use super::payload::{login_started_payload, login_success_payload};

pub(super) fn logged_in_message(session: &Value) -> String {
    optional_string_field(session, "email").map_or_else(
        || "logged in".to_owned(),
        |email| format!("logged in as {email}"),
    )
}

pub(super) fn print_login_started(
    cli: &CliOptions,
    start: &Value,
    login_url: &str,
    user_code: Option<&str>,
    expires_seconds: u64,
    interval_seconds: u64,
) -> Result<()> {
    if cli.output.json {
        print_json_event(&login_started_payload(
            start,
            login_url,
            user_code,
            expires_seconds,
            interval_seconds,
        ))?;
        return Ok(());
    }
    progress(cli, "opened browser login");
    progress(cli, &login_wait_message(user_code));
    Ok(())
}

pub(super) fn print_login_success(
    cli: &CliOptions,
    status: &str,
    email: Option<&str>,
) -> Result<()> {
    if cli.output.json {
        return print_json(&login_success_payload(status, email));
    }
    if status == "saved" {
        println!("saved Tovuk session token");
        return Ok(());
    }
    Ok(())
}

fn login_wait_message(user_code: Option<&str>) -> String {
    user_code.map_or_else(
        || "waiting for browser login".to_owned(),
        |code| format!("waiting for browser login code {code}"),
    )
}

fn print_json_event(value: &Value) -> Result<()> {
    let source = serde_json::to_string(value).map_err(|error| internal_error(error.to_string()))?;
    eprintln!("{source}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::logged_in_message;

    #[test]
    fn logged_in_message_uses_email_when_present() {
        assert_eq!(
            logged_in_message(&json!({"email": "ada@example.com"})),
            "logged in as ada@example.com"
        );
    }

    #[test]
    fn logged_in_message_does_not_invent_user_when_email_is_missing() {
        assert_eq!(logged_in_message(&json!({})), "logged in");
    }
}
