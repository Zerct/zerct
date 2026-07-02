use serde_json::json;

use crate::cli::args::CliOptions;

use super::support_create_body;

#[test]
fn support_create_body_uses_typed_payload_and_omits_empty_context() {
    let cli = CliOptions {
        command: "support".to_owned(),
        args: vec![
            "create".to_owned(),
            " Request failed ".to_owned(),
            " request_123 ".to_owned(),
            "timed".to_owned(),
            "out ".to_owned(),
        ],
        ..CliOptions::default()
    };

    assert_eq!(
        support_create_body(&cli).ok(),
        Some(json!({
            "subject": "Request failed",
            "details": "request_123 timed out"
        }))
    );
}

#[test]
fn support_create_body_includes_trimmed_context_flags() {
    let mut cli = CliOptions {
        command: "support".to_owned(),
        args: vec![
            "create".to_owned(),
            "Request failed".to_owned(),
            "Request id request_123 failed.".to_owned(),
        ],
        ..CliOptions::default()
    };
    cli.failing_command = " tovuk request show request_123 --json ".to_owned();
    cli.first_log_line = " upstream timeout ".to_owned();
    cli.request_id = " request_123 ".to_owned();
    cli.scraper_id = " tiktok ".to_owned();
    cli.severity = " urgent ".to_owned();

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
fn support_create_body_requires_subject_and_details() {
    let cli = CliOptions {
        command: "support".to_owned(),
        args: vec!["create".to_owned(), " ".to_owned(), "details".to_owned()],
        ..CliOptions::default()
    };

    let message = support_create_body(&cli)
        .err()
        .map(|error| error.to_string());
    assert_eq!(
        message.as_deref(),
        Some("Support ticket subject and details are required.")
    );
}

#[test]
fn support_create_body_rejects_invalid_severity() {
    let cli = CliOptions {
        command: "support".to_owned(),
        args: vec![
            "create".to_owned(),
            "Request failed".to_owned(),
            "Request id request_123 failed.".to_owned(),
        ],
        severity: "critical".to_owned(),
        ..CliOptions::default()
    };

    let payload = support_create_body(&cli)
        .err()
        .map(|error| error.payload().clone());
    assert_eq!(
        payload.as_ref().map(|payload| payload.code.as_str()),
        Some("invalid_support_ticket")
    );
    assert_eq!(
        payload.as_ref().map(|payload| payload.message.as_str()),
        Some("Support ticket severity must be low, normal, or urgent.")
    );
}
