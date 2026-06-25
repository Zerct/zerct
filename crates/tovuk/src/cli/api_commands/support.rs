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
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<String>,
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

fn support_severity(cli: &CliOptions) -> Option<String> {
    optional_trimmed_value(cli.severity.as_str())
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
