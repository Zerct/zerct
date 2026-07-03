use super::super::{
    args::CliOptions,
    errors::{Result, print_json},
};
use super::{common::page_query, http::api_request};
use reqwest::Method;
use serde_json::Value;

pub(crate) fn pricing(_cli: &CliOptions) -> Result<()> {
    let payload = super::pricing_catalog::pricing_payload()?;
    print_json(&payload)
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
