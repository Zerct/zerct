#[cfg(test)]
#[path = "support_tests.rs"]
/// Support command tests.
mod tests;

use super::super::{
    ExecuteCommand,
    args::CliOptions,
    errors::{CliError, OutputFormat, Result, agent_error},
    utils::encode_component,
};
use super::{
    common::{command_arg, joined_args, optional_trimmed_value},
    generic::{print_authenticated_mutation, print_paged_authenticated},
};
use hyper::Method;
use serde::Serialize;
use serde_json::{Value, to_value};

#[derive(Clone, Copy, Debug)]
/// Top-level support command action.
pub(in crate::cli) struct SupportCommand;

impl ExecuteCommand for SupportCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        match cli.args().first().map_or("list", String::as_str) {
            "list" => return print_paged_authenticated(cli, "/v1/support/tickets"),
            "create" => {
                let payload = result_or_return!(SupportPayload::try_from(cli));
                return print_authenticated_mutation(
                    cli,
                    Method::POST,
                    "/v1/support/tickets",
                    Some(payload.0),
                );
            }
            "resolve" => {
                let ticket_id = result_or_return!(command_arg(
                    cli,
                    (
                        "invalid_support_ticket",
                        "Support ticket id is required.",
                        "Use `tovuk support resolve <ticket_id> --json` with an id from support list.",
                    ),
                ));
                return print_authenticated_mutation(
                    cli,
                    Method::POST,
                    &format!(
                        "/v1/support/tickets/{}/resolve",
                        encode_component(&ticket_id)
                    ),
                    None,
                );
            }
            _ => {
                return Err(agent_error(
                    "unknown_command",
                    "Unknown support command.",
                    "Use `tovuk support list --json`, `tovuk support create \"Subject\" \"Details\" --json`, or `tovuk support resolve <ticket_id> --json`.",
                    cli.output_format(),
                ));
            }
        }
    }
}

/// Serialized support ticket request body.
struct SupportPayload(Value);

impl TryFrom<&CliOptions> for SupportPayload {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        let subject = value.args().get(0b1).map_or("", String::as_str).trim();
        let details = joined_args(value, 0b10);
        if subject.is_empty() || details.is_empty() {
            return Err(agent_error(
                "invalid_support_ticket",
                "Support ticket subject and details are required.",
                "Use `tovuk support create \"Short subject\" \"Command output, request id, and first actionable error line\" --json`.",
                value.output_format(),
            ));
        }
        let severity = match optional_trimmed_value(value.severity()) {
            Some(severity) => Some(result_or_return!(SupportSeverity::try_from((
                severity.as_str(),
                value.output_format(),
            )))),
            None => None,
        };
        return to_value(SupportTicketInput {
            details,
            failing_command: optional_trimmed_value(value.failing_command()),
            first_log_line: optional_trimmed_value(value.first_log_line()),
            request_id: optional_trimmed_value(value.request_id()),
            scraper_id: optional_trimmed_value(value.scraper_id()),
            severity,
            subject: subject.to_owned(),
        })
        .map(Self)
        .map_err(|error| {
            return agent_error(
                "invalid_support_ticket",
                format!("Support ticket input could not be encoded: {error}"),
                "Retry with visible support ticket subject and details.",
                value.output_format(),
            );
        });
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
/// Supported public support ticket severities.
enum SupportSeverity {
    /// Low-impact problem.
    Low,
    /// Normal support priority.
    Normal,
    /// Urgent problem requiring prompt attention.
    Urgent,
}

impl TryFrom<(&str, OutputFormat)> for SupportSeverity {
    type Error = CliError;

    fn try_from(value: (&str, OutputFormat)) -> Result<Self> {
        let (severity, output_format) = value;
        match severity {
            "low" => return Ok(Self::Low),
            "normal" => return Ok(Self::Normal),
            "urgent" => return Ok(Self::Urgent),
            _ => {
                return Err(agent_error(
                    "invalid_support_ticket",
                    "Support ticket severity must be low, normal, or urgent.",
                    "Use `--severity low`, `--severity normal`, or `--severity urgent`.",
                    output_format,
                ));
            }
        }
    }
}

#[derive(Serialize)]
/// JSON body used to create a support ticket.
struct SupportTicketInput {
    /// Detailed description of the problem.
    details: String,
    /// Command that encountered the problem.
    #[serde(skip_serializing_if = "Option::is_none")]
    failing_command: Option<String>,
    /// First relevant log line.
    #[serde(skip_serializing_if = "Option::is_none")]
    first_log_line: Option<String>,
    /// Related public request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    /// Related data-source identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    scraper_id: Option<String>,
    /// Optional urgency classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    severity: Option<SupportSeverity>,
    /// Concise ticket subject.
    subject: String,
}

#[cfg(test)]
/// Builds the support request body used by contract tests.
///
/// # Errors
///
/// Returns an error when the test options do not form a valid support request.
fn support_create_body(cli: &CliOptions) -> Result<Value> {
    return SupportPayload::try_from(cli).map(|payload| return payload.0);
}
