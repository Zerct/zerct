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
mod tests {
    use super::{joined_args, optional_trimmed_value, required_arg};
    use crate::cli::args::CliOptions;

    #[test]
    fn joined_args_trims_and_drops_empty_values() {
        let cli = CliOptions {
            args: vec![
                "create".to_owned(),
                " first ".to_owned(),
                " ".to_owned(),
                "second".to_owned(),
            ],
            ..CliOptions::default()
        };

        assert_eq!(joined_args(&cli, 1), "first second");
    }

    #[test]
    fn optional_trimmed_value_drops_blank_input() {
        assert_eq!(optional_trimmed_value(" value "), Some("value".to_owned()));
        assert_eq!(optional_trimmed_value(" "), None);
    }

    #[test]
    fn required_arg_reads_requested_position() {
        let cli = CliOptions {
            args: vec!["show".to_owned(), "request_123".to_owned()],
            ..CliOptions::default()
        };

        assert_eq!(
            required_arg(&cli, 1, "missing", "missing", "retry").ok(),
            Some("request_123".to_owned())
        );
    }
}
