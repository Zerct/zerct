use super::super::{
    args::CliOptions,
    errors::{Result, print_json},
};
use super::{common::page_query, http::api_request};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn pricing(_cli: &CliOptions) -> Result<()> {
    print_json(&pricing_payload())
}

fn pricing_payload() -> Value {
    json!({
        "plans": [
            {
                "plan": "plus",
                "monthlyPriceUsdCents": 2000,
                "includedBalanceUsdCents": 2000,
                "bonusBalanceUsdCents": 0
            },
            {
                "plan": "pro",
                "monthlyPriceUsdCents": 10000,
                "includedBalanceUsdCents": 12000,
                "bonusBalanceUsdCents": 2000
            },
            {
                "plan": "max",
                "monthlyPriceUsdCents": 20000,
                "includedBalanceUsdCents": 30000,
                "bonusBalanceUsdCents": 10000
            }
        ],
        "scraperPrices": [
            {"scraper": "reddit", "unit": "record", "usdMicrosPerResult": 700},
            {"scraper": "github", "unit": "record", "usdMicrosPerResult": 600},
            {"scraper": "linkedin", "unit": "record", "usdMicrosPerResult": 900},
            {"scraper": "tiktok", "unit": "record", "usdMicrosPerResult": 1700},
            {"scraper": "instagram", "unit": "record", "usdMicrosPerResult": 800},
            {"scraper": "x", "unit": "post", "usdMicrosPerResult": 300}
        ],
        "topUp": {
            "minimumUsdCents": 2000,
            "expiresAfterInactiveDays": 365
        },
        "nextActions": [
            "Use `tovuk scraper list --json` and `tovuk scraper show <scraper> --json` to choose a public-data scraper.",
            "Use `priceEvents[].usdMicros`, request limits, and `tovuk usage --json` to estimate account balance impact before high-count requests.",
            "Choose a plan, then use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json` when an upgrade is required."
        ]
    })
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
