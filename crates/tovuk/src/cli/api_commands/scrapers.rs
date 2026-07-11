#[cfg(test)]
#[path = "scrapers_tests.rs"]
/// Public data-source and request command tests.
mod tests;

use super::super::{
    ExecuteCommand,
    args::CliOptions,
    errors::{CliError, Result, agent_error},
    utils::encode_component,
};
use super::common::{page_query, required_arg};
use super::generic::{
    print_authenticated, print_authenticated_mutation, print_paged_authenticated,
};
use reqwest::Method;
use serde_json::{Value, from_str, json};

#[derive(Clone, Copy, Debug)]
/// Cancels one public data request.
struct CancelRequest;

impl ExecuteCommand for CancelRequest {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let request_id = result_or_return!(required_arg(
            cli,
            0b1,
            (
                "request_required",
                "Request id is required.",
                "Use `tovuk request cancel request_123 --json` with an id from `tovuk request list --json`.",
            ),
        ));
        return print_authenticated_mutation(
            cli,
            Method::POST,
            &format!("/v1/requests/{}/cancel", encode_component(&request_id)),
            None,
        );
    }
}

#[derive(Clone, Copy, Debug)]
/// Creates one public data request.
struct CreateRequest;

impl ExecuteCommand for CreateRequest {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let scraper = result_or_return!(required_arg(
            cli,
            0b1,
            (
                "scraper_required",
                "Scraper id is required.",
                "Use `tovuk request create github '{\"query\":\"mcp server\",\"language\":\"Rust\",\"limit\":100}' --json`.",
            ),
        ));
        let input_source = result_or_return!(required_arg(
            cli,
            0b10,
            (
                "request_input_required",
                "Request input JSON is required.",
                "Use `tovuk request create github '{\"query\":\"mcp server\",\"language\":\"Rust\",\"limit\":100}' --json`.",
            ),
        ));
        let input = result_or_return!(RequestInput::try_from(RequestInputContext {
            cli,
            source: input_source.as_str(),
        }));
        return print_authenticated_mutation(
            cli,
            Method::POST,
            "/v1/requests",
            Some(json!({ "scraper": scraper, "input": input.0 })),
        );
    }
}

#[derive(Clone, Copy, Debug)]
/// Top-level request command action.
pub(in crate::cli) struct RequestCommand;

impl ExecuteCommand for RequestCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        match cli.args().first().map_or("list", String::as_str) {
            "list" => return print_paged_authenticated(cli, "/v1/requests"),
            "create" => return CreateRequest.execute(cli),
            "show" => return RequestShow.execute(cli),
            "results" => return RequestResults.execute(cli),
            "cancel" => return CancelRequest.execute(cli),
            _ => {
                return Err(agent_error(
                    "unknown_command",
                    "Unknown request command.",
                    "Use `tovuk request create <scraper> '<json>' --json`, `tovuk request show <request_id> --json`, `tovuk request results <request_id> --json`, or `tovuk request cancel <request_id> --json`.",
                    cli.output_format(),
                ));
            }
        }
    }
}

#[derive(Debug)]
/// Validated JSON object used as request input.
struct RequestInput(Value);

impl<'input> TryFrom<RequestInputContext<'input>> for RequestInput {
    type Error = CliError;

    fn try_from(value: RequestInputContext<'input>) -> Result<Self> {
        let mut input = match from_str::<Value>(value.source) {
            Ok(parsed) => parsed,
            Err(error) => {
                return Err(agent_error(
                    "invalid_request_input",
                    format!("Request input is not valid JSON: {error}"),
                    "Pass scraper input as a JSON object, for example `'{\"query\":\"mcp server\",\"language\":\"Rust\",\"limit\":100}'`.",
                    value.cli.output_format(),
                ));
            }
        };
        let Some(object) = input.as_object_mut() else {
            return Err(agent_error(
                "invalid_request_input",
                "Request input must be a JSON object.",
                "Pass scraper input as a JSON object, for example `'{\"query\":\"mcp server\",\"language\":\"Rust\",\"limit\":100}'`.",
                value.cli.output_format(),
            ));
        };
        let RequestLimit(request_limit) = result_or_return!(RequestLimit::try_from(value.cli));
        if let Some(limit) = request_limit {
            drop(object.insert("limit".to_owned(), json!(limit)));
        }
        return Ok(Self(input));
    }
}

#[derive(Clone, Copy, Debug)]
/// CLI and source text used to validate request input.
struct RequestInputContext<'input> {
    /// Validated CLI options.
    cli: &'input CliOptions,
    /// Raw JSON input text.
    source: &'input str,
}

#[derive(Clone, Copy, Debug)]
/// Optional validated request result limit.
struct RequestLimit(Option<u64>);

impl TryFrom<&CliOptions> for RequestLimit {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        if value.limit().is_empty() {
            return Ok(Self(None));
        }
        return match value.limit().parse::<u64>() {
            Ok(parsed) => Ok(Self(Some(parsed))),
            Err(_error) => Err(agent_error(
                "invalid_request_limit",
                "Request limit must be an integer.",
                "Pass `--limit 100` or include `\"limit\": 100` in the input JSON.",
                value.output_format(),
            )),
        };
    }
}

#[derive(Clone, Copy, Debug)]
/// Fetches stored results for a data request.
struct RequestResults;

impl ExecuteCommand for RequestResults {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let request_id = result_or_return!(required_arg(
            cli,
            0b1,
            (
                "request_required",
                "Request id is required.",
                "Use `tovuk request results request_123 --json` with an id from `tovuk request list --json`.",
            ),
        ));
        let route = format!(
            "/v1/requests/{}/results{}",
            encode_component(&request_id),
            page_query(cli)
        );
        return print_authenticated(cli, &route);
    }
}

#[derive(Clone, Copy, Debug)]
/// Fetches one data request by identifier.
struct RequestShow;

impl ExecuteCommand for RequestShow {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let request_id = result_or_return!(required_arg(
            cli,
            0b1,
            (
                "request_required",
                "Request id is required.",
                "Use `tovuk request show request_123 --json` with an id from `tovuk request list --json`.",
            ),
        ));
        return print_authenticated(
            cli,
            &format!("/v1/requests/{}", encode_component(&request_id)),
        );
    }
}

#[derive(Clone, Copy, Debug)]
/// Top-level public data-source command action.
pub(in crate::cli) struct ScraperCommand;

impl ExecuteCommand for ScraperCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        match cli.args().first().map_or("list", String::as_str) {
            "list" => return print_authenticated(cli, "/v1/data-sources"),
            "health" => return print_authenticated(cli, "/v1/data-sources/health"),
            "show" => return ScraperShow.execute(cli),
            _ => {
                return Err(agent_error(
                    "unknown_command",
                    "Unknown scraper command.",
                    "Use `tovuk scraper list --json`, `tovuk scraper health --json`, or `tovuk scraper show <scraper> --json`.",
                    cli.output_format(),
                ));
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// Fetches one public data source by identifier.
struct ScraperShow;

impl ExecuteCommand for ScraperShow {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let scraper = result_or_return!(required_arg(
            cli,
            0b1,
            (
                "scraper_required",
                "Scraper id is required.",
                "Use `tovuk scraper show github --json` with an id from `tovuk scraper list --json`.",
            ),
        ));
        return print_authenticated(
            cli,
            &format!("/v1/data-sources/{}", encode_component(&scraper)),
        );
    }
}

#[cfg(test)]
/// Parses a request input body used by contract tests.
///
/// # Errors
///
/// Returns an error when `source` is not a valid JSON object.
fn request_input(cli: &CliOptions, source: &str) -> Result<Value> {
    return RequestInput::try_from(RequestInputContext { cli, source }).map(|input| return input.0);
}
