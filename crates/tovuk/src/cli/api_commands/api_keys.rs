use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{command_arg, joined_args},
    generic::{print_authenticated, print_authenticated_mutation},
};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn api_key_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_authenticated(cli, "/v1/account/api-keys"),
        "create" => api_key_create(cli),
        "revoke" => api_key_revoke(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown API key command.",
            "Use `tovuk api-key list --json`, `tovuk api-key create \"Production scraper\" --json`, or `tovuk api-key revoke <api_key_id> --json`.",
            cli.output.json,
        )),
    }
}

fn api_key_create(cli: &CliOptions) -> Result<()> {
    print_authenticated_mutation(
        cli,
        Method::POST,
        "/v1/account/api-keys",
        Some(api_key_create_body(cli)?),
    )
}

fn api_key_create_body(cli: &CliOptions) -> Result<Value> {
    let name = joined_args(cli, 1);
    if name.is_empty() {
        return Err(agent_error(
            "invalid_api_key_name",
            "API key name is required.",
            "Use `tovuk api-key create \"Production scraper\" --json` with a short name for the script or environment.",
            cli.output.json,
        ));
    }
    Ok(json!({ "name": name }))
}

fn api_key_revoke(cli: &CliOptions) -> Result<()> {
    let key_id = command_arg(
        cli,
        "invalid_api_key",
        "API key id is required.",
        "Use `tovuk api-key revoke <api_key_id> --json` with an id from `tovuk api-key list --json`.",
    )?;
    print_authenticated_mutation(
        cli,
        Method::DELETE,
        &format!("/v1/account/api-keys/{}", encode_component(&key_id)),
        None,
    )
}

#[cfg(test)]
#[path = "api_keys_tests.rs"]
mod tests;
