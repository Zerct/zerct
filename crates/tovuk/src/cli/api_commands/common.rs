use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    utils::encode_component,
};

/// Stable code, message, and recovery instruction for a missing argument.
pub(super) type ArgumentError = (&'static str, &'static str, &'static str);

/// Returns the first command argument.
///
/// # Errors
///
/// Returns the supplied validation error when the argument is absent.
pub(super) fn command_arg(cli: &CliOptions, error: ArgumentError) -> Result<String> {
    return required_arg(cli, 0b1, error);
}

/// Joins trimmed non-empty positional arguments from `start_index` onward.
pub(super) fn joined_args(cli: &CliOptions, start_index: usize) -> String {
    return cli
        .args()
        .iter()
        .skip(start_index)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| return !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
}

/// Returns a trimmed owned value when the input is non-empty.
pub(super) fn optional_trimmed_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    return Some(trimmed.to_owned());
}

/// Builds an encoded pagination query string from parsed options.
pub(super) fn page_query(cli: &CliOptions) -> String {
    let mut params = Vec::new();
    if !cli.limit().is_empty() {
        params.push(format!("limit={}", encode_component(cli.limit())));
    }
    if !cli.cursor().is_empty() {
        params.push(format!("cursor={}", encode_component(cli.cursor())));
    }
    if params.is_empty() {
        return String::new();
    }
    return format!("?{}", params.join("&"));
}

/// Returns a required non-empty positional argument.
///
/// # Errors
///
/// Returns the supplied validation error when the argument is absent or empty.
pub(super) fn required_arg(cli: &CliOptions, index: usize, error: ArgumentError) -> Result<String> {
    let (code, message, instruction) = error;
    return cli
        .args()
        .get(index)
        .cloned()
        .filter(|value| return !value.is_empty())
        .ok_or_else(|| return agent_error(code, message, instruction, cli.output_format()));
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
