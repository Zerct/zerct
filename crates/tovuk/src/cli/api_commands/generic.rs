use super::super::{
    ExecuteCommand,
    args::CliOptions,
    errors::{Result, print_json},
};
use super::{
    common::page_query,
    http::{ApiRequestContent, api_request},
};
use reqwest::Method;
use serde_json::Value;

#[derive(Clone, Copy, Debug)]
/// Top-level public pricing command action.
pub(in crate::cli) struct PricingCommand;

impl ExecuteCommand for PricingCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let payload = result_or_return!(api_request(
            cli,
            Method::GET,
            "/v1/pricing",
            ApiRequestContent::Anonymous,
        ));
        return print_json(&payload);
    }
}

/// Fetches and prints one authenticated public API resource.
///
/// # Errors
///
/// Returns an error when authentication, transport, decoding, or output fails.
pub(in crate::cli) fn print_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    let token = result_or_return!(super::super::auth::read_or_login_token(cli));
    let response = result_or_return!(api_request(
        cli,
        Method::GET,
        route,
        ApiRequestContent::Authenticated { body: None, token },
    ));
    return print_json(&response);
}

/// Sends and prints one authenticated public API mutation.
///
/// # Errors
///
/// Returns an error when authentication, transport, decoding, or output fails.
pub(super) fn print_authenticated_mutation(
    cli: &CliOptions,
    method: Method,
    route: &str,
    body: Option<Value>,
) -> Result<()> {
    let token = result_or_return!(super::super::auth::read_or_login_token(cli));
    let response = result_or_return!(api_request(
        cli,
        method,
        route,
        ApiRequestContent::Authenticated { body, token },
    ));
    return print_json(&response);
}

/// Fetches and prints one authenticated paginated API resource.
///
/// # Errors
///
/// Returns an error when authentication, transport, decoding, or output fails.
pub(super) fn print_paged_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    return print_authenticated(cli, &format!("{route}{}", page_query(cli)));
}
