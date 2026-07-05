use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

use crate::helpers::{CheckResult, env_int, number_field, read_json};
use crate::mintlify_fetch::{normalize_target_url, retry_delay};

mod copy;
mod mcp;
mod pages;

pub(crate) fn check_agent_readiness(target: &str) -> CheckResult {
    let base_url = normalize_target_url(target);
    let retries = env_int("TOVUK_DOCS_CHECK_RETRIES", 8)?;
    let retry_delay = retry_delay()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("build HTTP client: {error}"))?;

    pages::check_required_agent_paths(&client, base_url.as_str(), retries, retry_delay)?;
    pages::check_html_paths(&client, base_url.as_str(), retries, retry_delay)?;
    pages::check_llms_skill_and_robots(&client, base_url.as_str(), retries, retry_delay)?;
    pages::check_content_negotiation(&client, base_url.as_str(), retries, retry_delay)?;
    mcp::check_mcp_discovery(&client, base_url.as_str(), retries, retry_delay)?;

    println!("Mintlify agent readiness checks passed for {base_url}");
    Ok(())
}

pub(crate) fn check_score(path: &str) -> CheckResult {
    let score: Value = read_json(path)?;
    let mut value = number_field(&score, "score");
    if value == 0.0 {
        value = number_field(&score, "overallScore");
    }
    let minimum = f64::from(
        i32::try_from(env_int("MINTLIFY_SCORE_MIN", 90)?)
            .map_err(|_| "MINTLIFY_SCORE_MIN must fit in an i32".to_owned())?,
    );
    if value < minimum {
        return Err(format!(
            "Mintlify score is {value:.0}/100; expected at least {minimum:.0}/100"
        ));
    }
    println!("Mintlify score is {value:.0}/100");
    Ok(())
}
