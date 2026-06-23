use super::super::{
    args::CliOptions,
    errors::{Result, print_json},
};
use super::{common::page_query, http::api_request};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn pricing(cli: &CliOptions) -> Result<()> {
    let response = api_request(cli, Method::GET, "/v1/capabilities", None, None)?;
    let plans = response.get("plans").cloned().unwrap_or(Value::Null);
    let products = response.get("products").cloned().unwrap_or(Value::Null);
    print_json(&json!({
        "plans": plans,
        "products": products,
        "nextActions": [
            "Use `tovuk scraper list --json` and `tovuk scraper show <scraper> --json` to choose a public-data scraper.",
            "Use `priceEvents[].usdMicros`, request limits, and `tovuk usage --json` to estimate account balance impact before high-count requests.",
            "Use `tovuk billing checkout --json` when an upgrade is required."
        ]
    }))
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
