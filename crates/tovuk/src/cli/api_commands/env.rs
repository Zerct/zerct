use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{command_arg, service_route},
    generic::{print_authenticated_mutation, service_get_command},
};
use reqwest::Method;
use serde_json::json;

pub(crate) fn env_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => service_get_command(cli, "env"),
        "set" => env_set(cli),
        "delete" => env_delete(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown env command.",
            "Use `tovuk env list`, `env set`, or `env delete`.",
            cli.output.json,
        )),
    }
}

fn env_set(cli: &CliOptions) -> Result<()> {
    let assignment = cli.args.get(1).cloned().unwrap_or_default();
    let separator = assignment.find('=').unwrap_or(0);
    if separator == 0 {
        return Err(agent_error(
            "invalid_env",
            "Environment assignment must be KEY=value.",
            "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
            cli.output.json,
        ));
    }
    let name = &assignment[..separator];
    let value = &assignment[separator + 1..];
    print_authenticated_mutation(
        cli,
        Method::PUT,
        &service_route(cli, "env")?,
        Some(json!({ "name": name, "value": value })),
    )
}

fn env_delete(cli: &CliOptions) -> Result<()> {
    let name = command_arg(
        cli,
        "invalid_env",
        "Environment variable name is required.",
        "Use `tovuk env delete --service <service> KEY`.",
    )?;
    print_authenticated_mutation(
        cli,
        Method::DELETE,
        &service_route(cli, &format!("env/{}", encode_component(&name)))?,
        None,
    )
}
