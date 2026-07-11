use super::super::{
    ExecuteCommand,
    args::CliOptions,
    errors::{CliError, Result, agent_error},
    utils::encode_component,
};
use super::{
    common::{command_arg, joined_args},
    generic::{print_authenticated, print_authenticated_mutation},
};
use hyper::Method;
#[cfg(test)]
use serde_json::Value;
use serde_json::json;

/// Validation error used when an API key identifier is absent.
const API_KEY_ID_ERROR: super::common::ArgumentError = (
    "invalid_api_key",
    "API key id is required.",
    "Use `tovuk api-key revoke <api_key_id> --json` with an id from `tovuk api-key list --json`.",
);

#[derive(Clone, Copy, Debug)]
/// Top-level API key command action.
pub(in crate::cli) struct ApiKeyCommand;

impl ExecuteCommand for ApiKeyCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        match cli.args().first().map_or("list", String::as_str) {
            "list" => return print_authenticated(cli, "/v1/account/api-keys"),
            "create" => {
                let name = result_or_return!(ApiKeyName::try_from(cli));
                return print_authenticated_mutation(
                    cli,
                    Method::POST,
                    "/v1/account/api-keys",
                    Some(json!({ "name": name.0 })),
                );
            }
            "revoke" => {
                let key_id = result_or_return!(command_arg(cli, API_KEY_ID_ERROR));
                return print_authenticated_mutation(
                    cli,
                    Method::DELETE,
                    &format!("/v1/account/api-keys/{}", encode_component(&key_id)),
                    None,
                );
            }
            _ => {
                return Err(agent_error(
                    "unknown_command",
                    "Unknown API key command.",
                    "Use `tovuk api-key list --json`, `tovuk api-key create \"Production scraper\" --json`, or `tovuk api-key revoke <api_key_id> --json`.",
                    cli.output_format(),
                ));
            }
        }
    }
}

#[derive(Debug)]
/// Validated non-empty API key name.
struct ApiKeyName(String);

impl TryFrom<&CliOptions> for ApiKeyName {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        let name = joined_args(value, 0b1);
        if name.is_empty() {
            return Err(agent_error(
                "invalid_api_key_name",
                "API key name is required.",
                "Use `tovuk api-key create \"Production scraper\" --json` with a short name for the script or environment.",
                value.output_format(),
            ));
        }
        return Ok(Self(name));
    }
}

#[cfg(test)]
/// Builds the API-key creation body used by contract tests.
///
/// # Errors
///
/// Returns an error when the test options do not contain a valid API-key name.
fn api_key_create_body(cli: &CliOptions) -> Result<Value> {
    return ApiKeyName::try_from(cli).map(|name| return json!({ "name": name.0 }));
}

#[cfg(test)]
#[path = "api_keys_tests.rs"]
mod tests;
