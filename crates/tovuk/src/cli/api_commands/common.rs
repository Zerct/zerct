use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};

pub(crate) fn command_arg(
    cli: &CliOptions,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    required_arg(cli, 1, code, message, instruction)
}

pub(crate) fn required_arg(
    cli: &CliOptions,
    index: usize,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    cli.args
        .get(index)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| agent_error(code, message, instruction, cli.output.json))
}

pub(crate) fn joined_args(cli: &CliOptions, start_index: usize) -> String {
    cli.args
        .iter()
        .skip(start_index)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn optional_trimmed_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
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

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
