use super::super::{
    args::CliOptions,
    errors::{Result, agent_error, print_json},
};
use super::{common::page_query, http::api_request};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn pricing(cli: &CliOptions) -> Result<()> {
    let response = api_request(cli, Method::GET, "/v1/capabilities", None, None)?;
    let payload = pricing_payload(&response, cli.output.json)?;
    print_json(&payload)
}

fn pricing_payload(response: &Value, json_output: bool) -> Result<Value> {
    let plans = required_capability_array(response, "plans", json_output)?;
    let products = required_capability_array(response, "products", json_output)?;
    Ok(json!({
        "plans": plans,
        "products": products,
        "nextActions": [
            "Use `tovuk scraper list --json` and `tovuk scraper show <scraper> --json` to choose a public-data scraper.",
            "Use `priceEvents[].usdMicros`, request limits, and `tovuk usage --json` to estimate account balance impact before high-count requests.",
            "Choose a plan, then use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json` when an upgrade is required."
        ]
    }))
}

fn required_capability_array(response: &Value, field: &str, json_output: bool) -> Result<Value> {
    let Some(value) = response.get(field) else {
        return Err(capabilities_contract_error(field, json_output));
    };
    if value.as_array().is_none() {
        return Err(capabilities_contract_error(field, json_output));
    }
    Ok(value.clone())
}

fn capabilities_contract_error(field: &str, json_output: bool) -> super::super::errors::CliError {
    agent_error(
        "capabilities_invalid",
        format!("Tovuk capabilities response is missing `{field}`."),
        "Retry `tovuk pricing --json`. If it keeps failing, create a Tovuk support ticket with command output.",
        json_output,
    )
}

pub(crate) fn print_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    let token = super::super::auth::read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, route, Some(&token), None)?;
    print_json(&response)
}

pub(crate) fn print_paged_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    print_authenticated(cli, &format!("{route}{}", page_query(cli)))
}

pub(crate) fn print_authenticated_mutation(
    cli: &CliOptions,
    method: Method,
    route: &str,
    body: Option<Value>,
) -> Result<()> {
    let token = super::super::auth::read_or_login_token(cli)?;
    let response = api_request(cli, method, route, Some(&token), body)?;
    print_json(&response)
}

#[cfg(test)]
#[path = "generic_tests.rs"]
mod tests;
