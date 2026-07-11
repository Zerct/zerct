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

use reqwest::{StatusCode, blocking::Client, header::ACCEPT, redirect::Policy as RedirectPolicy};

use serde as _;

use serde_json as _;

use sha2 as _;

use std::{
    env,
    io::{Result as InputOutputResult, Write as _, stderr, stdout},
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::check_support::CheckResult;

/// Maximum time allowed to establish one registry connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(0x0a);

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

/// Maximum total duration of one registry request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(0x1e);

/// Public identifier sent with every package registry request.
const USER_AGENT: &str = "Tovuk public release availability check (https://github.com/tovuk/tovuk)";

/// Maximum accepted candidate version length.
const VERSION_LENGTH_LIMIT: usize = 0x40;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x7] = [
    size_of_val(&build_client),
    size_of_val(&check_registry),
    size_of_val(&classify_status),
    size_of_val(&parse_version_argument),
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
fn build_client() -> CheckResult<Client> {
    return Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .https_only(true)
        .redirect(RedirectPolicy::none())
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| return format!("build release availability HTTP client: {error}"));
}

/// Check that one public registry does not contain the candidate version.
///
/// # Errors
///
/// Returns an error on network failure, a published version, or any response
/// other than HTTP 404.
fn check_registry(client: &Client, registry: Registry, version: &str) -> CheckResult {
    let endpoint = registry_endpoint(registry, version);
    let response = check_try!(
        client
            .get(endpoint.as_str())
            .header(ACCEPT, "application/json")
            .send()
            .map_err(|error| return format!(
                "query {} release availability: {error}",
                registry.name
            ))
    );
    return classify_status(registry, version, response.status());
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
