use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    constants::BILLING_CHECKOUT_ROUTE,
    errors::{Result, agent_error, print_json},
    project::open_url,
};
use super::common::joined_args;
use super::http::api_request;
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn billing_command(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let action = cli.args.first().map_or("checkout", String::as_str);
    let route = match action {
        "" | "checkout" => BILLING_CHECKOUT_ROUTE,
        "portal" => "/v1/billing/portal",
        _ => {
            return Err(agent_error(
                "unknown_billing_command",
                "Unknown billing command.",
                "Use `tovuk billing checkout --json` or `tovuk billing portal`.",
                cli.output.json,
            ));
        }
    };
    let reason = joined_args(cli, 1);
    let body = if route == BILLING_CHECKOUT_ROUTE {
        Some(json!({
            "target_plan": "pro",
            "reason": if reason.is_empty() { "Upgrade to Tovuk Pro." } else { reason.as_str() },
        }))
    } else {
        None
    };
    let response = api_request(cli, Method::POST, route, Some(&token), body)?;
    if cli.output.json {
        return print_json(&response);
    }
    let url = response
        .get("checkout")
        .and_then(|checkout| checkout.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    println!("{url}");
    if !url.is_empty() {
        open_url(url);
    }
    Ok(())
}
