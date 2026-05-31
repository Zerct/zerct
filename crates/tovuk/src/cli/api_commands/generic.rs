use super::super::{
    args::CliOptions,
    errors::{Result, print_json},
};
use super::{
    common::{page_query, service_route},
    http::api_request,
};
use reqwest::Method;
use serde_json::Value;

pub(crate) fn capabilities(cli: &CliOptions) -> Result<()> {
    let response = api_request(cli, Method::GET, "/v1/capabilities", None, None)?;
    print_json(&response)
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
    let token = super::super::auth::read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::GET,
        &service_route(cli, suffix)?,
        Some(&token),
        None,
    )?;
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
