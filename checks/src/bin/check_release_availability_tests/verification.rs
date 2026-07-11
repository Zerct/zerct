//! Release availability request and response verification.

use http::StatusCode;

use super::{REGISTRIES, classify_status, parse_version_argument, registry_endpoint};

/// Verify that only HTTP 404 represents an unpublished version.
///
/// # Errors
///
/// Returns an error when a status is classified incorrectly.
#[test]
fn classification_is_fail_closed() -> Result<(), String> {
    let Some(registry) = REGISTRIES.first().copied() else {
        return Err("the registry list must not be empty".to_owned());
    };
    check_try!(classify_status(registry, "0.1.116", StatusCode::NOT_FOUND,));
    check_try!(require_error(
        classify_status(registry, "0.1.116", StatusCode::OK),
        "already publishes",
    ));
    return require_error(
        classify_status(registry, "0.1.116", StatusCode::INTERNAL_SERVER_ERROR),
        "unexpected HTTP status",
    );
}

/// Verify exact HTTPS metadata endpoints for all public registries.
///
/// # Errors
///
/// Returns an error when any endpoint differs from the public contract.
#[test]
fn endpoints_are_exact_and_https() -> Result<(), String> {
    let expected_endpoints = [
        "https://crates.io/api/v1/crates/tovuk/0.1.116",
        "https://registry.npmjs.org/tovuk/0.1.116",
        "https://pypi.org/pypi/tovuk/0.1.116/json",
    ];
    for (registry, expected) in REGISTRIES.iter().copied().zip(expected_endpoints) {
        let actual = registry_endpoint(registry, "0.1.116");
        if actual != expected {
            return Err(format!("unexpected {} endpoint: {actual}", registry.name));
        }
    }
    return Ok(());
}

/// Verify that the checker accepts exactly one valid version argument.
///
/// # Errors
///
/// Returns an error when argument count validation is incorrect.
#[test]
fn parser_requires_exactly_one_argument() -> Result<(), String> {
    let missing: [String; 0x0] = [];
    check_try!(require_error(
        parse_version_argument(missing.as_slice()),
        "usage:",
    ));
    let extra = [String::from("0.1.116"), String::from("0.1.117")];
    check_try!(require_error(
        parse_version_argument(extra.as_slice()),
        "usage:",
    ));
    let valid = [String::from("0.1.116")];
    let parsed = check_try!(parse_version_argument(valid.as_slice()));
    if parsed != "0.1.116" {
        return Err(format!("unexpected parsed version: {parsed}"));
    }
    return Ok(());
}

/// Require an operation to fail with a diagnostic fragment.
///
/// # Errors
///
/// Returns an error when the operation succeeds or its diagnostic omits the
/// expected fragment.
fn require_error<Value>(result: Result<Value, String>, expected: &str) -> Result<(), String> {
    let Err(message) = result else {
        return Err(format!(
            "availability check unexpectedly succeeded; expected {expected}"
        ));
    };
    if !message.contains(expected) {
        return Err(format!("unexpected availability error: {message}"));
    }
    return Ok(());
}

/// Verify conservative numeric release version validation.
///
/// # Errors
///
/// Returns an error when an invalid version is accepted.
#[test]
fn version_validation_rejects_ambiguous_inputs() -> Result<(), String> {
    for invalid in [
        "",
        "1.2",
        "1.2.3.4",
        "1..3",
        "01.2.3",
        "1.02.3",
        "1.2.03",
        "1.2.beta",
        "1.2.3?published=false",
    ] {
        let arguments = [String::from(invalid)];
        check_try!(require_error(
            parse_version_argument(arguments.as_slice()),
            "candidate version",
        ));
    }
    return Ok(());
}
