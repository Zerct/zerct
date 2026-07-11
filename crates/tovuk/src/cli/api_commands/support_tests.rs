use serde_json::json;

use crate::cli::args::{CliOptions, parse_args};

use super::support_create_body;

/// Parses CLI values for support contract tests.
fn parsed_cli(values: &[&str]) -> Option<CliOptions> {
    let arguments = values.iter().map(ToString::to_string).collect::<Vec<_>>();
    return parse_args(&arguments).ok();
}

#[test]
/// Verifies support payloads include normalized optional context.
///
/// # Panics
///
/// Panics when valid arguments do not parse or context fields are serialized incorrectly.
fn support_create_body_includes_trimmed_context_flags() {
    let parsed = parsed_cli(&[
        "support",
        "create",
        "Request failed",
        "Request id request_123 failed.",
        "--failing-command",
        " tovuk request show request_123 --json ",
        "--first-log-line",
        " upstream timeout ",
        "--request-id",
        " request_123 ",
        "--scraper-id",
        " tiktok ",
        "--severity",
        " urgent ",
    ]);
    assert!(parsed.is_some(), "support arguments should parse");
    let Some(cli) = parsed else {
        return;
    };

    assert_eq!(
        support_create_body(&cli).ok(),
        Some(json!({
            "subject": "Request failed",
            "details": "Request id request_123 failed.",
            "severity": "urgent",
            "request_id": "request_123",
            "scraper_id": "tiktok",
            "failing_command": "tovuk request show request_123 --json",
            "first_log_line": "upstream timeout"
        }))
    );
}

#[test]
/// Verifies support payloads reject unsupported severities.
///
/// # Panics
///
/// Panics when valid arguments do not parse or invalid severity handling changes.
fn support_create_body_rejects_invalid_severity() {
    let parsed = parsed_cli(&[
        "support",
        "create",
        "Request failed",
        "Request id request_123 failed.",
        "--severity",
        "critical",
    ]);
    assert!(parsed.is_some(), "support arguments should parse");
    let Some(cli) = parsed else {
        return;
    };

    let payload = support_create_body(&cli)
        .err()
        .map(|error| return error.payload().clone());
    assert!(payload.is_some(), "invalid severity should return an error");
    let Some(error_payload) = payload.as_ref() else {
        return;
    };
    assert_eq!(error_payload.code(), "invalid_support_ticket");
    assert_eq!(
        error_payload.message(),
        "Support ticket severity must be low, normal, or urgent."
    );
}

#[test]
/// Verifies support payloads require both subject and details.
///
/// # Panics
///
/// Panics when valid arguments do not parse or incomplete payload handling changes.
fn support_create_body_requires_subject_and_details() {
    let parsed = parsed_cli(&["support", "create", " ", "details"]);
    assert!(parsed.is_some(), "support arguments should parse");
    let Some(cli) = parsed else {
        return;
    };

    let message = support_create_body(&cli)
        .err()
        .map(|error| return error.message().to_owned());
    assert_eq!(
        message.as_deref(),
        Some("Support ticket subject and details are required.")
    );
}

#[test]
/// Verifies support payloads normalize required fields and omit empty context.
///
/// # Panics
///
/// Panics when valid arguments do not parse or required fields serialize incorrectly.
fn support_create_body_uses_typed_payload_and_omits_empty_context() {
    let parsed = parsed_cli(&[
        "support",
        "create",
        " Request failed ",
        " request_123 ",
        "timed",
        "out ",
    ]);
    assert!(parsed.is_some(), "support arguments should parse");
    let Some(cli) = parsed else {
        return;
    };

    assert_eq!(
        support_create_body(&cli).ok(),
        Some(json!({
            "subject": "Request failed",
            "details": "request_123 timed out"
        }))
    );
}
