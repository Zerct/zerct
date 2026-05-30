use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, print_json},
    project::encode_component,
};
use super::{
    common::app_route,
    generic::{app_get_command, print_authenticated_mutation},
    http::api_request,
};
use reqwest::Method;
use serde_json::{Map, Value, json};

pub(crate) fn platform_command(cli: &CliOptions) -> Result<()> {
    app_get_command(cli, "platform")
}

pub(crate) fn sqlite_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "sqlite_binding_required",
            "SQLite binding name is required.",
            "Use `tovuk sqlite create --app <app> DB --json`.",
            "sqlite/databases",
            "name",
        ),
        _ => unknown_platform_command(cli, "sqlite"),
    }
}

pub(crate) fn kv_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "kv_binding_required",
            "KV binding name is required.",
            "Use `tovuk kv create --app <app> CACHE --json`.",
            "kv/namespaces",
            "name",
        ),
        _ => unknown_platform_command(cli, "kv"),
    }
}

pub(crate) fn queue_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "queue_name_required",
            "Queue name is required.",
            "Use `tovuk queue create --app <app> jobs --json`.",
            "queues",
            "name",
        ),
        _ => unknown_platform_command(cli, "queue"),
    }
}

pub(crate) fn cron_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_cron(cli),
        _ => unknown_platform_command(cli, "cron"),
    }
}

pub(crate) fn durable_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "durable_class_required",
            "Durable Object class name is required.",
            "Use `tovuk durable create --app <app> Room --json`.",
            "durable-objects/namespaces",
            "className",
        ),
        _ => unknown_platform_command(cli, "durable"),
    }
}

pub(crate) fn binding_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_service_binding(cli),
        _ => unknown_platform_command(cli, "binding"),
    }
}

pub(crate) fn caps_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("", String::as_str) {
        "set" => set_usage_cap(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown usage cap command.",
            "Use `tovuk caps set worker_requests --period day --value 100000 --json`.",
            cli.output.json,
        )),
    }
}

fn create_app_resource(
    cli: &CliOptions,
    code: &str,
    message: &str,
    instruction: &str,
    suffix: &str,
    body_key: &str,
) -> Result<()> {
    let name = required_arg(cli, 1, code, message, instruction)?;
    let mut body = Map::new();
    body.insert(body_key.to_owned(), Value::String(name));
    print_authenticated_mutation(
        cli,
        Method::POST,
        &app_route(cli, suffix)?,
        Some(Value::Object(body)),
    )
}

fn create_cron(cli: &CliOptions) -> Result<()> {
    let name = required_arg(
        cli,
        1,
        "cron_name_required",
        "Cron trigger name is required.",
        "Use `tovuk cron create --app <app> nightly \"0 0 * * *\" --json`.",
    )?;
    let cron_expression = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if cron_expression.trim().is_empty() {
        return Err(agent_error(
            "cron_expression_required",
            "Cron expression is required.",
            "Use `tovuk cron create --app <app> nightly \"0 0 * * *\" --json`.",
            cli.output.json,
        ));
    }
    print_authenticated_mutation(
        cli,
        Method::POST,
        &app_route(cli, "cron")?,
        Some(json!({
            "name": name,
            "cronExpression": cron_expression,
        })),
    )
}

fn create_service_binding(cli: &CliOptions) -> Result<()> {
    let binding_name = required_arg(
        cli,
        1,
        "binding_name_required",
        "Service binding name is required.",
        "Use `tovuk binding create --app <app> AUTH_SERVICE --target <target_app> --json`.",
    )?;
    let target_app = if cli.target.is_empty() {
        required_arg(
            cli,
            2,
            "binding_target_required",
            "Service binding target app is required.",
            "Use `tovuk binding create --app <app> AUTH_SERVICE --target <target_app> --json`.",
        )?
    } else {
        cli.target.clone()
    };
    print_authenticated_mutation(
        cli,
        Method::POST,
        &app_route(cli, "service-bindings")?,
        Some(json!({
            "bindingName": binding_name,
            "targetApp": target_app,
        })),
    )
}

fn set_usage_cap(cli: &CliOptions) -> Result<()> {
    let metric = required_arg(
        cli,
        1,
        "cap_metric_required",
        "Usage cap metric is required.",
        "Use `tovuk caps set worker_requests --period day --value 100000 --json`.",
    )?;
    let period = if cli.period.is_empty() {
        cli.args.get(2).cloned().unwrap_or_default()
    } else {
        cli.period.clone()
    };
    let value = if cli.value.is_empty() {
        cli.args.get(3).cloned().unwrap_or_default()
    } else {
        cli.value.clone()
    };
    if period.is_empty() || value.is_empty() {
        return Err(agent_error(
            "cap_period_or_value_required",
            "Usage cap period and value are required.",
            "Use `tovuk caps set worker_requests --period day --value 100000 --json`.",
            cli.output.json,
        ));
    }
    let cap_value = value.parse::<u64>().map_err(|_error| {
        agent_error(
            "invalid_cap_value",
            "Usage cap value must be a positive integer.",
            "Pass an integer value such as `100000`, then retry.",
            cli.output.json,
        )
    })?;
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::PUT,
        &format!("/v1/usage/caps/{}", encode_component(&metric)),
        Some(&token),
        Some(json!({
            "period": period,
            "capValue": cap_value,
            "hardStop": true,
            "notifyAtPercent": 80,
        })),
    )?;
    print_json(&response)
}

fn required_arg(
    cli: &CliOptions,
    index: usize,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    cli.args
        .get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| agent_error(code, message, instruction, cli.output.json))
}

fn unknown_platform_command(cli: &CliOptions, family: &str) -> Result<()> {
    Err(agent_error(
        "unknown_command",
        format!("Unknown {family} command."),
        "Use `list` or `create`, then retry with `--json` for agent-readable output.",
        cli.output.json,
    ))
}
