//! Fail-closed public package registry release availability check.

/// Propagate a failed availability check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

#[cfg(test)]
#[path = "check_release_availability_tests/verification.rs"]
mod tests;

use core::time::Duration;

use flate2 as _;

use http::{StatusCode, header::ACCEPT};

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use serde as _;

use serde_json as _;

use sha2 as _;

use std::{
    env,
    io::{Result as InputOutputResult, Write as _, stderr, stdout},
    process::ExitCode,
    thread::sleep,
};

use tar as _;

use tokio as _;

use tovuk_public_checks::{check_support::CheckResult, http_transport::Client as TransportClient};

use url as _;

/// Maximum time allowed to establish one registry connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(0x0a);

/// Largest accepted package-registry metadata response.
const MAXIMUM_REGISTRY_BODY_BYTES: usize = 0x0002_0000;

/// Public package registries that must not contain the candidate version.
const REGISTRIES: &[Registry] = &[
    Registry {
        endpoint_prefix: "https://crates.io/api/v1/crates/tovuk/",
        endpoint_suffix: "",
        name: "crates.io",
    },
    Registry {
        endpoint_prefix: "https://registry.npmjs.org/tovuk/",
        endpoint_suffix: "",
        name: "npm",
    },
    Registry {
        endpoint_prefix: "https://pypi.org/pypi/tovuk/",
        endpoint_suffix: "/json",
        name: "PyPI",
    },
];

/// Delays before the four allowed retries of a transient registry failure.
const REGISTRY_RETRY_DELAYS: [Duration; 0x4] = [
    Duration::from_secs(0x1),
    Duration::from_secs(0x2),
    Duration::from_secs(0x4),
    Duration::from_secs(0x8),
];

/// Maximum total duration of one registry request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(0x1e);

/// Public identifier sent with every package registry request.
const USER_AGENT: &str = "Tovuk public release availability check (https://github.com/tovuk/tovuk)";

/// Maximum accepted candidate version length.
const VERSION_LENGTH_LIMIT: usize = 0x40;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x9] = [
    size_of_val(&build_client),
    size_of_val(&check_registry),
    size_of_val(&classify_status),
    size_of_val(&is_retryable_status),
    size_of_val(&parse_version_argument),
    size_of_val(&query_registry_with_retries),
    size_of_val(&registry_endpoint),
    size_of_val(&run),
    size_of_val(&validate_version),
];

/// One immutable public package registry endpoint.
#[derive(Clone, Copy, Debug)]
struct Registry {
    /// Endpoint content before the candidate version.
    endpoint_prefix: &'static str,
    /// Endpoint content after the candidate version.
    endpoint_suffix: &'static str,
    /// Human-readable public registry name.
    name: &'static str,
}

/// Build the bounded HTTPS-only registry client.
///
/// # Errors
///
/// Returns an error when the bounded client cannot be constructed.
fn build_client() -> CheckResult<TransportClient> {
    return TransportClient::build(CONNECT_TIMEOUT, REQUEST_TIMEOUT, 0x00, USER_AGENT)
        .map_err(|error| return format!("build release availability HTTP client: {error}"));
}

/// Check that one public registry does not contain the candidate version.
///
/// # Errors
///
/// Returns an error on network failure, a published version, or any response
/// other than HTTP 404.
fn check_registry(client: &TransportClient, registry: Registry, version: &str) -> CheckResult {
    let endpoint = registry_endpoint(registry, version);
    let status = check_try!(query_registry_with_retries(
        client,
        registry,
        endpoint.as_str(),
    ));
    return classify_status(registry, version, status);
}

/// Classify the only two accepted registry responses.
///
/// # Errors
///
/// Returns an error for a published version or any response other than HTTP
/// 404.
fn classify_status(registry: Registry, version: &str, status: StatusCode) -> CheckResult {
    if status == StatusCode::NOT_FOUND {
        return Ok(());
    }
    if status == StatusCode::OK {
        return Err(format!(
            "{} already publishes tovuk version {version}",
            registry.name
        ));
    }
    return Err(format!(
        "{} returned unexpected HTTP status {status} for tovuk version {version}",
        registry.name
    ));
}

/// Return whether one response may represent a transient registry failure.
fn is_retryable_status(status: StatusCode) -> bool {
    return status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
}

/// Execute the availability check and report failures on standard error.
///
/// # Errors
///
/// Returns an error when a command failure cannot be written to standard error.
fn main() -> InputOutputResult<ExitCode> {
    match run() {
        Ok(()) => return Ok(ExitCode::SUCCESS),
        Err(error) => {
            return writeln!(stderr().lock(), "{error}").map(|()| return ExitCode::FAILURE);
        }
    }
}

/// Parse the one required synchronized public version argument.
///
/// # Errors
///
/// Returns an error unless exactly one valid version argument is present.
fn parse_version_argument(arguments: &[String]) -> CheckResult<String> {
    let mut argument_iterator = arguments.iter();
    let Some(version) = argument_iterator.next() else {
        return Err("usage: check-release-availability <synchronized-version>".to_owned());
    };
    if argument_iterator.next().is_some() {
        return Err("usage: check-release-availability <synchronized-version>".to_owned());
    }
    check_try!(validate_version(version));
    return Ok(version.clone());
}

/// Query one registry with a finite exponential retry schedule.
///
/// # Errors
///
/// Returns an error after transport retries are exhausted. HTTP status
/// classification remains the caller's fail-closed responsibility.
fn query_registry_with_retries(
    client: &TransportClient,
    registry: Registry,
    endpoint: &str,
) -> CheckResult<StatusCode> {
    for retry_delay in REGISTRY_RETRY_DELAYS {
        match client.get(
            endpoint,
            &[(ACCEPT.as_str(), "application/json")],
            MAXIMUM_REGISTRY_BODY_BYTES,
        ) {
            Ok(response) if !is_retryable_status(response.status()) => {
                return Ok(response.status());
            }
            Ok(response) => drop(response),
            Err(error) => drop(error),
        }
        sleep(retry_delay);
    }
    return client
        .get(
            endpoint,
            &[(ACCEPT.as_str(), "application/json")],
            MAXIMUM_REGISTRY_BODY_BYTES,
        )
        .map(|response| return response.status())
        .map_err(|error| {
            return format!(
                "query {} release availability after bounded retries: {error}",
                registry.name
            );
        });
}

/// Build one exact public package metadata endpoint.
fn registry_endpoint(registry: Registry, version: &str) -> String {
    return format!(
        "{}{version}{}",
        registry.endpoint_prefix, registry.endpoint_suffix
    );
}

/// Check every registry and report one successful preflight.
///
/// # Errors
///
/// Returns an error when arguments, client construction, registry checks, or
/// output fail.
fn run() -> CheckResult {
    let arguments = env::args().skip(0x1).collect::<Vec<_>>();
    let version = check_try!(parse_version_argument(arguments.as_slice()));
    let client = check_try!(build_client());
    for registry in REGISTRIES.iter().copied() {
        check_try!(check_registry(&client, registry, version.as_str()));
    }
    return writeln!(
        stdout().lock(),
        "Tovuk version {version} is unpublished on crates.io, npm, and PyPI."
    )
    .map_err(|error| return format!("write release availability result: {error}"));
}

/// Require a bounded numeric `major.minor.patch` candidate version.
///
/// # Errors
///
/// Returns an error when the candidate is empty, oversized, nonnumeric, or not
/// canonical `major.minor.patch` form.
fn validate_version(version: &str) -> CheckResult {
    if version.is_empty() || version.len() > VERSION_LENGTH_LIMIT {
        return Err(format!(
            "candidate version must contain 1 to {VERSION_LENGTH_LIMIT} characters"
        ));
    }
    if version.split('.').count() != 0x3 {
        return Err("candidate version must use numeric major.minor.patch form".to_owned());
    }
    for component in version.split('.') {
        if component.is_empty()
            || !component
                .chars()
                .all(|character| return character.is_ascii_digit())
        {
            return Err("candidate version must use numeric major.minor.patch form".to_owned());
        }
        if component.len() > 0x1 && component.starts_with('0') {
            return Err("candidate version components must not have leading zeroes".to_owned());
        }
    }
    return Ok(());
}
