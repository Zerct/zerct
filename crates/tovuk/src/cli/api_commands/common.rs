use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use serde_json::{Map, Value};

pub(crate) fn insert_optional(body: &mut Map<String, Value>, key: &str, value: &str) {
    if !value.is_empty() {
        body.insert(key.to_owned(), Value::String(value.to_owned()));
    }
}

pub(crate) fn command_arg(
    cli: &CliOptions,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    cli.args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| agent_error(code, message, instruction, cli.output.json))
}

pub(crate) fn require_app(cli: &CliOptions) -> Result<String> {
    if cli.app.is_empty() {
        return Err(agent_error(
            "missing_app",
            "App is required.",
            "Pass `--app <app>` using either the app name from tovuk.toml or the app id printed by deploy.",
            cli.output.json,
        ));
    }
    Ok(cli.app.clone())
}

pub(crate) fn app_route(cli: &CliOptions, suffix: &str) -> Result<String> {
    Ok(format!(
        "/v1/apps/{}/{}",
        encode_component(&require_app(cli)?),
        suffix
    ))
}

pub(crate) fn page_query(cli: &CliOptions) -> String {
    let mut params = Vec::new();
    if !cli.limit.is_empty() {
        params.push(format!("limit={}", encode_component(&cli.limit)));
    }
    if !cli.cursor.is_empty() {
        params.push(format!("cursor={}", encode_component(&cli.cursor)));
    }
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}
