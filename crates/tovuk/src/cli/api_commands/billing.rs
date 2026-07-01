use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    constants::BILLING_CHECKOUT_ROUTE,
    errors::{Result, agent_error, print_json},
    utils::open_url,
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
                "Use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, `tovuk billing checkout max --json`, or `tovuk billing portal`.",
                cli.output.json,
            ));
        }
    };
    let body = if route == BILLING_CHECKOUT_ROUTE {
        Some(billing_checkout_body(cli)?)
    } else {
        None
    };
    let response = api_request(cli, Method::POST, route, Some(&token), body)?;
    if cli.output.json {
        return print_json(&response);
    }
    let url = billing_session_url(&response, cli.output.json)?;
    println!("{url}");
    open_url(url);
    Ok(())
}

fn billing_checkout_body(cli: &CliOptions) -> Result<Value> {
    let target_plan = billing_checkout_plan(cli)?;
    let reason = joined_args(cli, 2);
    Ok(json!({
        "target_plan": target_plan,
        "reason": if reason.is_empty() {
            format!("Open Tovuk {target_plan} checkout.")
        } else {
            reason
        },
    }))
}

fn billing_checkout_plan(cli: &CliOptions) -> Result<&str> {
    match cli.args.get(1).map(String::as_str) {
        Some(plan @ ("plus" | "pro" | "max")) => Ok(plan),
        Some(_) => Err(agent_error(
            "billing_plan_invalid",
            "Billing plan must be plus, pro, or max.",
            "Use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json`.",
            cli.output.json,
        )),
        None => Err(agent_error(
            "billing_plan_required",
            "Billing plan is required.",
            "Use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json`.",
            cli.output.json,
        )),
    }
}

fn billing_session_url(response: &Value, json_output: bool) -> Result<&str> {
    if let Some(url) = response
        .get("checkout")
        .and_then(|checkout| checkout.get("url"))
        .and_then(Value::as_str)
        .filter(|url| !url.trim().is_empty())
    {
        return Ok(url);
    }

    Err(agent_error(
        "billing_url_missing",
        "Tovuk billing did not return a URL.",
        "Retry `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, `tovuk billing checkout max --json`, or `tovuk billing portal --json`. If it keeps failing, create a Tovuk support ticket with command output.",
        json_output,
    ))
}

#[cfg(test)]
#[path = "billing_tests.rs"]
mod tests;
