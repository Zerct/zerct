#[cfg(test)]
#[path = "billing_tests.rs"]
/// Billing command tests.
mod tests;

use super::super::{
    ExecuteCommand,
    args::CliOptions,
    auth::read_or_login_token,
    constants::BILLING_CHECKOUT_ROUTE,
    errors::{CliError, OutputFormat, Result, agent_error, print_json, write_stdout_line},
    utils::open_url,
};
use super::common::joined_args;
use super::http::{ApiRequestContent, api_request};
use hyper::Method;
use serde_json::{Value, json};

#[derive(Debug)]
/// Validated billing checkout request.
enum BillingCheckout {
    /// Subscription plan checkout.
    Plan(PlanCheckout),
    /// Account balance top-up checkout.
    TopUp(TopUpCheckout),
}

impl TryFrom<&CliOptions> for BillingCheckout {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        if value.top_up_usd_cents().is_empty() {
            return PlanCheckout::try_from(value).map(Self::Plan);
        }
        return TopUpCheckout::try_from(value).map(Self::TopUp);
    }
}

#[derive(Clone, Copy, Debug)]
/// Top-level billing command action.
pub(in crate::cli) struct BillingCommand;

impl ExecuteCommand for BillingCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        let token = result_or_return!(read_or_login_token(cli));
        let action = cli.args().first().map_or("checkout", String::as_str);
        let route = match action {
            "" | "checkout" => BILLING_CHECKOUT_ROUTE,
            "portal" => "/v1/billing/portal",
            _ => {
                return Err(agent_error(
                    "unknown_billing_command",
                    "Unknown billing command.",
                    "Use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, `tovuk billing checkout max --json`, or `tovuk billing portal`.",
                    cli.output_format(),
                ));
            }
        };
        let body = if route == BILLING_CHECKOUT_ROUTE {
            Some(Value::from(result_or_return!(BillingCheckout::try_from(
                cli
            ))))
        } else {
            None
        };
        let response = result_or_return!(api_request(
            cli,
            Method::POST,
            route,
            ApiRequestContent::Authenticated { body, token },
        ));
        if cli.is_json() {
            return print_json(&response);
        }
        let url = result_or_return!(BillingSessionUrl::try_from((
            &response,
            cli.output_format(),
        )));
        result_or_return!(write_stdout_line(url.0));
        open_url(url.0);
        return Ok(());
    }
}

#[derive(Clone, Copy, Debug)]
/// Validated checkout or billing-portal URL.
struct BillingSessionUrl<'response>(&'response str);

impl<'response> TryFrom<(&'response Value, OutputFormat)> for BillingSessionUrl<'response> {
    type Error = CliError;

    fn try_from(value: (&'response Value, OutputFormat)) -> Result<Self> {
        let (response, output_format) = value;
        if let Some(url) = response
            .get("checkout")
            .and_then(|checkout| return checkout.get("url"))
            .and_then(Value::as_str)
            .filter(|url| return !url.trim().is_empty())
        {
            return Ok(Self(url));
        }
        return Err(agent_error(
            "billing_url_missing",
            "Tovuk billing did not return a URL.",
            "Retry `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, `tovuk billing checkout max --json`, or `tovuk billing portal --json`. If it keeps failing, create a Tovuk support ticket with command output.",
            output_format,
        ));
    }
}

#[derive(Debug)]
/// Subscription plan checkout body.
struct PlanCheckout {
    /// User-provided or generated checkout reason.
    reason: String,
    /// Public subscription plan identifier.
    target_plan: String,
}

impl TryFrom<&CliOptions> for PlanCheckout {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        let target_plan = match value.args().get(0b1).map(String::as_str) {
            Some(plan @ ("plus" | "pro" | "max")) => plan,
            Some(_) => {
                return Err(agent_error(
                    "billing_plan_invalid",
                    "Billing plan must be plus, pro, or max.",
                    "Use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json`.",
                    value.output_format(),
                ));
            }
            None => {
                return Err(agent_error(
                    "billing_plan_required",
                    "Billing plan is required.",
                    "Use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json`.",
                    value.output_format(),
                ));
            }
        };
        let reason = joined_args(value, 0b10);
        return Ok(Self {
            reason: if reason.is_empty() {
                format!("Open Tovuk {target_plan} checkout.")
            } else {
                reason
            },
            target_plan: target_plan.to_owned(),
        });
    }
}

#[derive(Debug)]
/// Account balance top-up checkout body.
struct TopUpCheckout {
    /// User-provided or generated checkout reason.
    reason: String,
    /// Top-up amount in United States dollar cents.
    top_up_usd_cents: u32,
}

impl TryFrom<&CliOptions> for TopUpCheckout {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        if matches!(
            value.args().get(0b1).map(String::as_str),
            Some("plus" | "pro" | "max")
        ) {
            return Err(agent_error(
                "billing_checkout_target_conflict",
                "Billing checkout cannot include both a plan and top-up amount.",
                "Use `tovuk billing checkout plus --json` for a plan or `tovuk billing checkout --top-up-usd-cents 2000 --json` for balance.",
                value.output_format(),
            ));
        }
        let top_up_usd_cents = result_or_return!(
            value
                .top_up_usd_cents()
                .parse::<u32>()
                .map_err(|_error| {
                    return agent_error(
                        "billing_top_up_invalid",
                        "Billing top-up amount must be an integer number of USD cents.",
                        "Use `tovuk billing checkout --top-up-usd-cents 2000 --json` for the minimum $20 top-up.",
                        value.output_format(),
                    );
                })
        );
        let reason = joined_args(value, 0b1);
        let whole_dollars = top_up_usd_cents.checked_div(100).unwrap_or_default();
        let remaining_cents = top_up_usd_cents.checked_rem(100).unwrap_or_default();
        return Ok(Self {
            reason: if reason.is_empty() {
                format!("Open Tovuk ${whole_dollars}.{remaining_cents:02} balance top-up checkout.")
            } else {
                reason
            },
            top_up_usd_cents,
        });
    }
}

impl From<BillingCheckout> for Value {
    #[inline]
    fn from(value: BillingCheckout) -> Self {
        match value {
            BillingCheckout::Plan(checkout) => {
                return json!({
                    "target_plan": checkout.target_plan,
                    "reason": checkout.reason,
                });
            }
            BillingCheckout::TopUp(checkout) => {
                return json!({
                    "top_up_usd_cents": checkout.top_up_usd_cents,
                    "reason": checkout.reason,
                });
            }
        }
    }
}

#[cfg(test)]
/// Builds the billing checkout body used by contract tests.
///
/// # Errors
///
/// Returns an error when the checkout target or amount is invalid.
fn billing_checkout_body(cli: &CliOptions) -> Result<Value> {
    return BillingCheckout::try_from(cli).map(Value::from);
}

#[cfg(test)]
/// Reads the checkout URL from a public billing response used by contract tests.
///
/// # Errors
///
/// Returns an error when the response omits a valid checkout URL.
fn billing_session_url(response: &Value, output_format: OutputFormat) -> Result<&str> {
    return BillingSessionUrl::try_from((response, output_format)).map(|url| return url.0);
}
