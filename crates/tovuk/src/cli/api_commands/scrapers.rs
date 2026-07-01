use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    utils::encode_component,
};
use super::common::{page_query, required_arg};
use super::generic::{
    print_authenticated, print_authenticated_mutation, print_paged_authenticated,
};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn scraper_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_authenticated(cli, "/v1/scrapers"),
        "health" => print_authenticated(cli, "/v1/scrapers/health"),
        "show" => scraper_show(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown scraper command.",
            "Use `tovuk scraper list --json`, `tovuk scraper health --json`, or `tovuk scraper show <scraper> --json`.",
            cli.output.json,
        )),
    }
}

pub(crate) fn request_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_paged_authenticated(cli, "/v1/requests"),
        "create" => request_create(cli),
        "show" => request_show(cli),
        "results" => request_results(cli),
        "cancel" => request_cancel(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown request command.",
            "Use `tovuk request create <scraper> '<json>' --json`, `tovuk request show <request_id> --json`, `tovuk request results <request_id> --json`, or `tovuk request cancel <request_id> --json`.",
            cli.output.json,
        )),
    }
}

fn scraper_show(cli: &CliOptions) -> Result<()> {
    let scraper = required_arg(
        cli,
        1,
        "scraper_required",
        "Scraper id is required.",
        "Use `tovuk scraper show google-maps --json` with an id from `tovuk scraper list --json`.",
    )?;
    print_authenticated(cli, &format!("/v1/scrapers/{}", encode_component(&scraper)))
}

fn request_create(cli: &CliOptions) -> Result<()> {
    let scraper = required_arg(
        cli,
        1,
        "scraper_required",
        "Scraper id is required.",
        "Use `tovuk request create google-maps '{\"query\":\"coffee shops\",\"limit\":100}' --json`.",
    )?;
    let input_source = request_input_source(cli)?;
    let input = request_input(cli, input_source.as_str())?;

    print_authenticated_mutation(
        cli,
        Method::POST,
        "/v1/requests",
        Some(json!({
            "scraper": scraper,
            "input": input,
        })),
    )
}

fn request_show(cli: &CliOptions) -> Result<()> {
    let request_id = required_arg(
        cli,
        1,
        "request_required",
        "Request id is required.",
        "Use `tovuk request show request_123 --json` with an id from `tovuk request list --json`.",
    )?;
    print_authenticated(
        cli,
        &format!("/v1/requests/{}", encode_component(&request_id)),
    )
}

fn request_results(cli: &CliOptions) -> Result<()> {
    let request_id = required_arg(
        cli,
        1,
        "request_required",
        "Request id is required.",
        "Use `tovuk request results request_123 --json` with an id from `tovuk request list --json`.",
    )?;
    let route = format!(
        "/v1/requests/{}/results{}",
        encode_component(&request_id),
        page_query(cli)
    );
    print_authenticated(cli, &route)
}

fn request_cancel(cli: &CliOptions) -> Result<()> {
    let request_id = required_arg(
        cli,
        1,
        "request_required",
        "Request id is required.",
        "Use `tovuk request cancel request_123 --json` with an id from `tovuk request list --json`.",
    )?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &format!("/v1/requests/{}/cancel", encode_component(&request_id)),
        None,
    )
}

fn request_input_source(cli: &CliOptions) -> Result<String> {
    required_arg(
        cli,
        2,
        "request_input_required",
        "Request input JSON is required.",
        "Use `tovuk request create google-maps '{\"query\":\"coffee shops\",\"limit\":100}' --json`.",
    )
}

fn request_input(cli: &CliOptions, input_source: &str) -> Result<Value> {
    let mut input = serde_json::from_str::<Value>(input_source).map_err(|error| {
        agent_error(
            "invalid_request_input",
            format!("Request input is not valid JSON: {error}"),
            "Pass scraper input as a JSON object, for example `'{\"query\":\"coffee shops\",\"limit\":100}'`.",
            cli.output.json,
        )
    })?;
    if !input.is_object() {
        return Err(agent_error(
            "invalid_request_input",
            "Request input must be a JSON object.",
            "Pass scraper input as a JSON object, for example `'{\"query\":\"coffee shops\",\"limit\":100}'`.",
            cli.output.json,
        ));
    }
    if !cli.limit.is_empty() {
        let limit = cli.limit.parse::<u64>().map_err(|_error| {
            agent_error(
                "invalid_request_limit",
                "Request limit must be an integer.",
                "Pass `--limit 100` or include `\"limit\": 100` in the input JSON.",
                cli.output.json,
            )
        })?;
        if let Some(object) = input.as_object_mut() {
            object.insert("limit".to_owned(), json!(limit));
        }
    }
    Ok(input)
}

#[cfg(test)]
#[path = "scrapers_tests.rs"]
mod tests;
