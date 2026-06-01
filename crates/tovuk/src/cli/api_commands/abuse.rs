use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, print_json},
    project::encode_component,
};
use super::{
    common::{command_arg, insert_optional},
    generic::print_paged_authenticated,
    http::api_request,
};
use reqwest::Method;
use serde_json::{Map, Value};

pub(crate) fn abuse_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_paged_authenticated(cli, "/v1/abuse/reports"),
        "report" => abuse_report(cli),
        "appeal" => abuse_appeal(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown abuse command.",
            "Use `tovuk abuse report <target_url> \"Summary\" \"Details\" --category phishing --reporter-email reporter@example.com --evidence \"Evidence\" --json`, `tovuk abuse list --json`, or `tovuk abuse appeal <report_id> \"Details\" --json`.",
            cli.output.json,
        )),
    }
}

fn abuse_report(cli: &CliOptions) -> Result<()> {
    let target_url = cli.args.get(1).cloned().unwrap_or_default();
    let summary = cli.args.get(2).cloned().unwrap_or_default();
    let details = cli
        .args
        .iter()
        .skip(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if target_url.is_empty()
        || summary.is_empty()
        || details.trim().is_empty()
        || cli.abuse.reporter_email.trim().is_empty()
    {
        return Err(agent_error(
            "invalid_abuse_report",
            "Abuse report target URL, summary, details, and reporter email are required.",
            "Use `tovuk abuse report <target_url> \"Summary\" \"Details\" --category phishing --reporter-email reporter@example.com --evidence \"Evidence\" --json`.",
            cli.output.json,
        ));
    }

    let mut body = Map::new();
    body.insert(
        "reporter_email".to_owned(),
        Value::String(cli.abuse.reporter_email.clone()),
    );
    body.insert(
        "category".to_owned(),
        Value::String(if cli.abuse.category.is_empty() {
            "other".to_owned()
        } else {
            cli.abuse.category.clone()
        }),
    );
    body.insert("target_url".to_owned(), Value::String(target_url));
    body.insert("summary".to_owned(), Value::String(summary));
    body.insert(
        "details".to_owned(),
        Value::String(details.trim().to_owned()),
    );
    body.insert(
        "evidence".to_owned(),
        Value::String(if cli.abuse.evidence.trim().is_empty() {
            details.trim().to_owned()
        } else {
            cli.abuse.evidence.clone()
        }),
    );
    insert_optional(&mut body, "reporter_name", &cli.abuse.reporter_name);
    insert_optional(&mut body, "service_id", &cli.service);
    insert_optional(&mut body, "target_path", &cli.abuse.target_path);
    insert_optional(&mut body, "object_path", &cli.abuse.object_path);
    let response = api_request(
        cli,
        Method::POST,
        "/v1/abuse/reports",
        None,
        Some(Value::Object(body)),
    )?;
    print_json(&response)
}

fn abuse_appeal(cli: &CliOptions) -> Result<()> {
    let report_id = command_arg(
        cli,
        "invalid_abuse_report",
        "Abuse report id is required.",
        "Use `tovuk abuse appeal <report_id> \"Remediation details\" --json` with an id from abuse list.",
    )?;
    let details = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if details.trim().is_empty() {
        return Err(agent_error(
            "invalid_abuse_appeal",
            "Abuse appeal details are required.",
            "Use `tovuk abuse appeal <report_id> \"Remediation details\" --evidence \"Evidence\" --json`.",
            cli.output.json,
        ));
    }

    let token = read_or_login_token(cli)?;
    let mut body = Map::new();
    body.insert(
        "details".to_owned(),
        Value::String(details.trim().to_owned()),
    );
    insert_optional(&mut body, "evidence", &cli.abuse.evidence);
    let response = api_request(
        cli,
        Method::POST,
        &format!("/v1/abuse/reports/{}/appeal", encode_component(&report_id)),
        Some(&token),
        Some(Value::Object(body)),
    )?;
    print_json(&response)
}
