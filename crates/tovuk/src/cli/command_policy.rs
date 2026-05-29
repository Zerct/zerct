use super::constants::JAVASCRIPT_BACKEND_RUNTIMES;

pub(crate) fn command_tokens(command: &str) -> Vec<String> {
    command
        .replace(['&', '|', ';', '(', ')'], " ")
        .split_whitespace()
        .map(|token| token.trim_matches(['"', '\'']).to_owned())
        .filter(|token| !token.is_empty())
        .collect()
}

pub(crate) fn command_name_from_token(token: &str) -> String {
    token.rsplit('/').next().unwrap_or_default().to_owned()
}

pub(crate) fn uses_javascript_backend_runtime(command: &str) -> bool {
    command_tokens(command)
        .iter()
        .any(|token| JAVASCRIPT_BACKEND_RUNTIMES.contains(&command_name_from_token(token).as_str()))
}

pub(crate) fn is_noop_command(command: &str) -> bool {
    let command = command.trim();
    command == ":" || command == "true"
}
