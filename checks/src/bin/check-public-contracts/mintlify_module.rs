/// Public contract checks for copy.
#[path = "mintlify_module/copy.rs"]
pub mod copy;

/// Public contract checks for deployment.
#[path = "mintlify_module/deployment.rs"]
pub mod deployment;

/// Public contract checks for mcp.
#[path = "mintlify_module/mcp.rs"]
pub mod mcp;

/// Public contract checks for pages.
#[path = "mintlify_module/pages.rs"]
pub mod pages;

use core::time::Duration;

use crate::helpers::{CheckResult, OutputChannel, env_int, number_field, read_json, write_line};

use crate::mintlify_fetch::{
    FetchContext, FetchPolicy, docs_cache_identity, normalize_target_url, retry_delay,
};

use serde_json::Value;

use std::{thread::sleep, time::Instant};

use tovuk_public_checks::http_transport::Client;

/// Maximum duration allowed to establish a docs connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(0x0a);

/// Largest accepted number of complete public readiness retry attempts.
const MAX_READINESS_RETRIES: i64 = 0x10;

/// Shared public readiness deadline, excluding compilation and local preflight checks.
const READINESS_DEADLINE: Duration = Duration::from_mins(0x0a);

/// Safe redirect ceiling for public documentation requests.
const REDIRECT_LIMIT: u8 = 0x05;

/// Total duration allowed for one docs request and its redirects.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(0x14);

/// Public identifier sent with agent-readiness requests.
const USER_AGENT: &str =
    "Tovuk public documentation readiness check (https://github.com/tovuk/tovuk)";

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&check_agent_readiness),
    size_of_val(&check_agent_readiness_once),
    size_of_val(&check_score),
    size_of_val(&readiness_retries),
];

/// Contract implementation for `check_agent_readiness`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_agent_readiness(target: &str) -> CheckResult {
    let base_url = normalize_target_url(target);
    let retries = check_try!(readiness_retries());
    let policy = FetchPolicy::new(
        retries,
        check_try!(retry_delay()),
        check_try!(docs_cache_identity()),
        check_try!(
            Instant::now()
                .checked_add(READINESS_DEADLINE)
                .ok_or_else(|| return "calculate public docs readiness deadline".to_owned())
        ),
    );
    let mut context = FetchContext::new(
        base_url,
        check_try!(
            Client::build(CONNECT_TIMEOUT, REQUEST_TIMEOUT, REDIRECT_LIMIT, USER_AGENT)
                .map_err(|error| format!("build HTTP client: {error}"))
        ),
        policy,
    );

    let mut last_error = "public docs readiness was not attempted".to_owned();
    for attempt in 0..=context.retries() {
        context.set_readiness_attempt(attempt);
        match check_agent_readiness_once(&context) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = error,
        }
        if attempt >= context.retries() || !context.can_retry_after_delay() {
            break;
        }
        sleep(context.retry_delay());
    }
    return Err(last_error);
}

/// Check every public documentation contract once.
///
/// # Errors
///
/// Returns an error when the current deployment has not satisfied every contract.
fn check_agent_readiness_once(context: &FetchContext) -> CheckResult {
    check_try!(deployment::check_exact_deployment(context));
    check_try!(pages::check_required_agent_paths(context));
    check_try!(pages::check_html_paths(context));
    check_try!(pages::check_llms_skill_and_robots(context));
    check_try!(pages::check_content_negotiation(context));
    check_try!(mcp::check_mcp_discovery(context));

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

/// Read a bounded number of complete readiness retry attempts.
///
/// # Errors
///
/// Returns an error when the configured retry count is negative or excessive.
fn readiness_retries() -> CheckResult<i64> {
    let retries = check_try!(env_int("TOVUK_DOCS_CHECK_RETRIES", 0x0008));
    if (0..=MAX_READINESS_RETRIES).contains(&retries) {
        return Ok(retries);
    }
    return Err(format!(
        "TOVUK_DOCS_CHECK_RETRIES must be between 0 and {MAX_READINESS_RETRIES}."
    ));
}
