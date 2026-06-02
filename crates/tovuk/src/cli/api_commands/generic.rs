use super::super::{
    args::CliOptions,
    errors::{Result, print_json},
};
use super::{
    common::{page_query, service_route},
    http::api_request,
};
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
            "Use the `products` entries to choose Worker, Static Frontend, SQLite, Object Storage, State, KV, Queue, Cron, Service Bindings, Secrets, Custom Domains, Logs, Builds, or Usage Caps before changing code.",
            "Use each product's `features`, `meters`, `meter_details`, `pricing_fields`, and `limit_fields` to verify supported behavior, price work, and choose hard caps.",
            "Use `tovuk usage --json` after login to compare current usage against these limits.",
            "Use `tovuk limits set <metric> --period month --value <n> --json` to set a hard cap before paid overages.",
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

pub(crate) fn service_get_command(cli: &CliOptions, suffix: &str) -> Result<()> {
    let route = service_route(cli, suffix)?;
    let token = super::super::auth::read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    print_json(&response)
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
