use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{command_arg, joined_args, optional_trimmed_value},
    generic::{print_authenticated_mutation, print_paged_authenticated},
};
use reqwest::Method;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct SupportTicketInput {
    subject: String,
    details: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scraper_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failing_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_log_line: Option<String>,
}

pub(crate) fn support_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_paged_authenticated(cli, "/v1/support/tickets"),
        "create" => support_create(cli),
        "resolve" => support_resolve(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown support command.",
            "Use `tovuk support list --json`, `tovuk support create \"Subject\" \"Details\" --json`, or `tovuk support resolve <ticket_id> --json`.",
            cli.output.json,
        )),
    }
}

fn support_resolve(cli: &CliOptions) -> Result<()> {
    let ticket_id = command_arg(
        cli,
        "invalid_support_ticket",
        "Support ticket id is required.",
        "Use `tovuk support resolve <ticket_id> --json` with an id from support list.",
    )?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &format!(
            "/v1/support/tickets/{}/resolve",
            encode_component(&ticket_id)
        ),
        None,
    )
}

fn support_create(cli: &CliOptions) -> Result<()> {
    print_authenticated_mutation(
        cli,
        Method::POST,
        "/v1/support/tickets",
        Some(support_create_body(cli)?),
    )
}

fn support_create_body(cli: &CliOptions) -> Result<Value> {
    let subject = cli.args.get(1).map_or("", String::as_str).trim();
    let details = support_details(cli);
    if subject.is_empty() || details.is_empty() {
        return Err(agent_error(
            "invalid_support_ticket",
            "Support ticket subject and details are required.",
            "Use `tovuk support create \"Short subject\" \"Command output, request id, and first actionable error line\" --json`.",
            cli.output.json,
        ));
    }
    serde_json::to_value(SupportTicketInput {
        subject: subject.to_owned(),
        details,
        severity: support_severity(cli),
        request_id: optional_trimmed_value(cli.request_id.as_str()),
        scraper_id: optional_trimmed_value(cli.scraper_id.as_str()),
        failing_command: optional_trimmed_value(cli.failing_command.as_str()),
        first_log_line: optional_trimmed_value(cli.first_log_line.as_str()),
    })
    .map_err(|error| {
        agent_error(
            "invalid_support_ticket",
            format!("Support ticket input could not be encoded: {error}"),
            "Retry with visible support ticket subject and details.",
            cli.output.json,
        )
    })
}

fn support_details(cli: &CliOptions) -> String {
    joined_args(cli, 2)
}

fn support_severity(cli: &CliOptions) -> String {
    optional_trimmed_value(cli.severity.as_str()).unwrap_or_else(|| "normal".to_owned())
}

#[cfg(test)]
mod tests {
    use super::support_create_body;
    use crate::cli::args::CliOptions;
    use serde_json::json;

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
                "details": "request_123 timed out",
                "severity": "normal"
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
}
