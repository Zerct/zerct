use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
};
use super::common::joined_args;
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
    let positional_name = joined_args(cli, 2);
    if positional_name.is_empty() {
        handle.to_owned()
    } else {
        positional_name
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
