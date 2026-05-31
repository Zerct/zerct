use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, print_json},
    project::encode_component,
};
use super::{
    common::{command_arg, insert_optional},
    generic::{print_authenticated_mutation, print_paged_authenticated},
    http::api_request,
};
use reqwest::Method;
use serde_json::{Map, Value};

pub(crate) fn support_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_paged_authenticated(cli, "/v1/support/tickets"),
        "create" => support_create(cli),
        "resolve" => support_resolve(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown support command.",
            "Use `tovuk support list --json` or `support create` with subject and details.",
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
    let subject = cli.args.get(1).cloned().unwrap_or_default();
    let details = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if subject.is_empty() || details.trim().is_empty() {
        return Err(agent_error(
            "invalid_support_ticket",
            "Support ticket subject and details are required.",
            "Use `tovuk support create \"Short subject\" \"Command, service id, build id, deploy id, and first actionable log line\" --json`.",
            cli.output.json,
        ));
    }

    let token = read_or_login_token(cli)?;
    let mut body = Map::new();
    body.insert("subject".to_owned(), Value::String(subject));
    body.insert(
        "details".to_owned(),
        Value::String(details.trim().to_owned()),
    );
    body.insert(
        "severity".to_owned(),
        Value::String(if cli.severity.is_empty() {
            "normal".to_owned()
        } else {
            cli.severity.clone()
        }),
    );
    insert_optional(&mut body, "service_id", &cli.service);
    insert_optional(&mut body, "failing_command", &cli.failing_command);
    insert_optional(&mut body, "build_id", &cli.build);
    insert_optional(&mut body, "deploy_id", &cli.deploy);
    insert_optional(&mut body, "first_log_line", &cli.first_log_line);
    let response = api_request(
        cli,
        Method::POST,
        "/v1/support/tickets",
        Some(&token),
        Some(Value::Object(body)),
    )?;
    print_json(&response)
}
