use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
};
use super::generic::{print_authenticated_mutation, print_paged_authenticated};
use reqwest::Method;
use serde_json::{Value, json};

const ACCOUNT_ACTIVITY_PATH: &str = "/v1/account/activity";

pub(crate) fn account_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("show", String::as_str) {
        "show" => print_paged_authenticated(cli, "/v1/account"),
        "activity" => print_paged_authenticated(cli, ACCOUNT_ACTIVITY_PATH),
        "update" => account_update(cli),
        _ => Err(agent_error(
            "unknown_account_command",
            "Unknown account command.",
            "Use `tovuk account show --json`, `tovuk account activity --json`, or `tovuk account update --handle <handle> --display-name <name> --json`.",
            cli.output.json,
        )),
    }
}

fn account_update(cli: &CliOptions) -> Result<()> {
    print_authenticated_mutation(
        cli,
        Method::PUT,
        "/v1/account",
        Some(account_update_body(cli)?),
    )
}

fn account_update_body(cli: &CliOptions) -> Result<Value> {
    let handle = account_update_handle(cli)?;
    let display_name = account_update_display_name(cli, &handle);
    Ok(json!({
        "handle": handle,
        "displayName": display_name,
    }))
}

fn account_update_handle(cli: &CliOptions) -> Result<String> {
    if !cli.account.handle.is_empty() {
        return Ok(cli.account.handle.clone());
    }
    cli.args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            agent_error(
                "account_handle_required",
                "Account handle is required.",
                "Use `tovuk account update --handle <handle> --display-name <name> --json`.",
                cli.output.json,
            )
        })
}

fn account_update_display_name(cli: &CliOptions, handle: &str) -> String {
    if !cli.account.display_name.is_empty() {
        return cli.account.display_name.clone();
    }
    let positional_name = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if positional_name.trim().is_empty() {
        handle.to_owned()
    } else {
        positional_name.trim().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{ACCOUNT_ACTIVITY_PATH, account_update_body};
    use crate::cli::args::CliOptions;
    use serde_json::json;

    #[test]
    fn account_update_prefers_explicit_flags() {
        let mut cli = CliOptions {
            command: "account".to_owned(),
            args: vec!["update".to_owned(), "ignored".to_owned()],
            ..CliOptions::default()
        };
        cli.account.handle = "tovuk-team".to_owned();
        cli.account.display_name = "Tovuk Team".to_owned();

        assert_eq!(
            account_update_body(&cli).ok(),
            Some(json!({ "handle": "tovuk-team", "displayName": "Tovuk Team" }))
        );
    }

    #[test]
    fn account_update_accepts_positional_handle_and_name() {
        let cli = CliOptions {
            command: "account".to_owned(),
            args: vec![
                "update".to_owned(),
                "tovuk-team".to_owned(),
                "Tovuk".to_owned(),
                "Team".to_owned(),
            ],
            ..CliOptions::default()
        };

        assert_eq!(
            account_update_body(&cli).ok(),
            Some(json!({ "handle": "tovuk-team", "displayName": "Tovuk Team" }))
        );
    }

    #[test]
    fn account_update_requires_handle() {
        let cli = CliOptions {
            command: "account".to_owned(),
            args: vec!["update".to_owned()],
            ..CliOptions::default()
        };

        let message = account_update_body(&cli)
            .err()
            .map(|error| error.to_string());
        assert_eq!(message.as_deref(), Some("Account handle is required."));
    }

    #[test]
    fn account_activity_uses_consolidated_account_route() {
        assert_eq!(ACCOUNT_ACTIVITY_PATH, "/v1/account/activity");
        let retired_path = format!("/v1/{}", "activity");
        assert_ne!(ACCOUNT_ACTIVITY_PATH, retired_path);
    }
}
