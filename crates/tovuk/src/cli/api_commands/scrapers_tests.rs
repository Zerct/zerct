use serde_json::json;

use crate::cli::args::{CliOptions, options_for_test};

use super::{request_input, required_arg};

#[test]
/// Verifies request creation requires a JSON input argument.
///
/// # Panics
///
/// Panics when missing input is accepted or reports a different error.
fn request_create_requires_input_json() {
    let cli = options_for_test(&["request", "create", "github"]);

    let message = required_arg(
        &cli,
        0b10,
        (
            "request_input_required",
            "Request input JSON is required.",
            "Provide request input as a JSON object.",
        ),
    )
    .err()
    .map(|error| return error.message().to_owned());
    assert_eq!(message.as_deref(), Some("Request input JSON is required."));
}

#[test]
/// Verifies request input accepts objects and applies the explicit limit.
///
/// # Panics
///
/// Panics when valid input is rejected or the limit is not merged.
fn request_input_accepts_json_object_and_applies_limit_flag() {
    let cli = options_for_test(&["request", "create", "github", "{}", "--limit", "25"]);
    let expected_limit: u64 = 0x0019;

    assert_eq!(
        request_input(&cli, r#"{"query":"coffee shops"}"#).ok(),
        Some(json!({
            "query": "coffee shops",
            "limit": expected_limit
        }))
    );
}

#[test]
/// Verifies request input rejects non-object JSON.
///
/// # Panics
///
/// Panics when a non-object input is accepted or reports a different error.
fn request_input_rejects_non_object_json() {
    let message = request_input(&CliOptions::default(), "[]")
        .err()
        .map(|error| return error.message().to_owned());
    assert_eq!(
        message.as_deref(),
        Some("Request input must be a JSON object.")
    );
}
