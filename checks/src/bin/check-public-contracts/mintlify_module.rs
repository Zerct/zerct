/// Public contract checks for copy.
#[path = "mintlify_module/copy.rs"]
pub mod copy;

/// Public contract checks for mcp.
#[path = "mintlify_module/mcp.rs"]
pub mod mcp;

/// Public contract checks for pages.
#[path = "mintlify_module/pages.rs"]
pub mod pages;

use core::time::Duration;

use crate::helpers::{CheckResult, OutputChannel, env_int, number_field, read_json, write_line};

use crate::mintlify_fetch::{FetchContext, normalize_target_url, retry_delay};

use reqwest::blocking::Client;

use serde_json::Value;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&check_agent_readiness),
    size_of_val(&check_score),
];

/// Contract implementation for `check_agent_readiness`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_agent_readiness(target: &str) -> CheckResult {
    let base_url = normalize_target_url(target);
    let retries = check_try!(env_int("TOVUK_DOCS_CHECK_RETRIES", 0x0008));
    let context = FetchContext::new(
        base_url,
        check_try!(
            Client::builder()
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(|error| format!("build HTTP client: {error}"))
        ),
        retries,
        check_try!(retry_delay()),
    );

    check_try!(pages::check_required_agent_paths(&context));
    check_try!(pages::check_html_paths(&context));
    check_try!(pages::check_llms_skill_and_robots(&context));
    check_try!(pages::check_content_negotiation(&context));
    check_try!(mcp::check_mcp_discovery(&context));

    check_try!(write_line(
        OutputChannel::Regular,
        format!(
            "Mintlify agent readiness checks passed for {}",
            context.base_url()
        )
        .as_str(),
    ));
    return Ok(());
}

/// Contract implementation for `check_score`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_score(path: &str) -> CheckResult {
    let score: Value = check_try!(read_json(path));
    let mut value = number_field(&score, "score");
    if value.abs() < f64::EPSILON {
        value = number_field(&score, "overallScore");
    }
    let minimum = f64::from(check_try!(
        i32::try_from(check_try!(env_int("MINTLIFY_SCORE_MIN", 90)))
            .map_err(|error| return format!("MINTLIFY_SCORE_MIN must fit in an i32: {error}"))
    ));
    if value < minimum {
        return Err(format!(
            "Mintlify score is {value:.0}/100; expected at least {minimum:.0}/100"
        ));
    }
    check_try!(write_line(
        OutputChannel::Regular,
        format!("Mintlify score is {value:.0}/100").as_str(),
    ));
    return Ok(());
}
