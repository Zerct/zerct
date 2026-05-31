use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{command_arg, require_service, service_route},
    generic::{print_authenticated_mutation, service_get_command},
};
use reqwest::Method;
use serde_json::json;

pub(crate) fn domains_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => service_get_command(cli, "domains"),
        "add" => domain_add(cli),
        "verify" => domain_scoped_mutation(cli, "verify", Method::POST),
        "delete" => domain_scoped_mutation(cli, "", Method::DELETE),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown domains command.",
            "Use `domains list`, `domains add`, `domains verify`, or `domains delete`.",
            cli.output.json,
        )),
    }
}

fn domain_add(cli: &CliOptions) -> Result<()> {
    let domain = domain_arg(cli, "add")?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &service_route(cli, "domains")?,
        Some(json!({ "domain": domain })),
    )
}

fn domain_scoped_mutation(cli: &CliOptions, action: &str, method: Method) -> Result<()> {
    let domain = domain_arg(cli, if action.is_empty() { "delete" } else { action })?;
    let service = require_service(cli)?;
    let suffix = if action.is_empty() {
        String::new()
    } else {
        format!("/{action}")
    };
    print_authenticated_mutation(
        cli,
        method,
        &format!(
            "/v1/services/{}/domains/{}{}",
            encode_component(&service),
            encode_component(&domain),
            suffix
        ),
        None,
    )
}

fn domain_arg(cli: &CliOptions, command: &str) -> Result<String> {
    command_arg(
        cli,
        "missing_domain",
        "Domain is required.",
        &format!("Use `tovuk domains {command} --service <service> api.example.com`."),
    )
}
