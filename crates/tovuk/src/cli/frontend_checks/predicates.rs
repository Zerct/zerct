use super::super::{
    command_policy::{command_name_from_token, command_tokens},
    constants::{FRONTEND_INSTALL_COMMANDS, FRONTEND_PACKAGE_MANAGERS, JAVASCRIPT_LINTERS},
};

pub(crate) fn uses_javascript_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        JAVASCRIPT_LINTERS.contains(&command_name.as_str())
            || (command_name == "next"
                && tokens.get(index + 1).is_some_and(|value| value == "lint"))
    })
}

pub(super) fn uses_strict_frontend_typechecker(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        (command_name == "oxlint"
            && tokens.iter().any(|value| value == "--type-aware")
            && tokens.iter().any(|value| value == "--type-check"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|value| value == "check"))
    })
}

pub(super) fn uses_native_frontend_linter(command: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        let command_name = command_name_from_token(token);
        command_name == "oxlint"
            || (command_name == "biome"
                && tokens
                    .get(index + 1)
                    .is_some_and(|value| value == "check" || value == "lint"))
            || (command_name == "deno"
                && tokens.get(index + 1).is_some_and(|value| value == "lint"))
    })
}

pub(super) fn uses_native_dead_code_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "dead-code")
}

pub(super) fn uses_native_duplicate_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "dupes")
}

pub(super) fn uses_native_health_checker(command: &str) -> bool {
    uses_fallow_subcommand(command, "health")
}

fn uses_fallow_subcommand(command: &str, subcommand: &str) -> bool {
    let tokens = command_tokens(command);
    tokens.iter().enumerate().any(|(index, token)| {
        command_name_from_token(token) == "fallow"
            && tokens
                .get(index + 1)
                .is_some_and(|value| value == subcommand)
    })
}

pub(crate) fn has_frontend_install_command(tokens: &[String]) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        FRONTEND_INSTALL_COMMANDS.contains(
            &format!(
                "{} {}",
                command_name_from_token(token),
                tokens.get(index + 1).map_or("", String::as_str)
            )
            .as_str(),
        )
    })
}

pub(crate) fn has_frontend_script_run(tokens: &[String], script: &str) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if !FRONTEND_PACKAGE_MANAGERS.contains(&command_name_from_token(token).as_str())
            || tokens.get(index + 1).is_none_or(|value| value != "run")
        {
            return false;
        }
        tokens.get(index + 2).is_some_and(|value| value == script)
            || (tokens
                .get(index + 2)
                .is_some_and(|value| value.starts_with('-'))
                && tokens.get(index + 3).is_some_and(|value| value == script))
    })
}

pub(super) fn referenced_package_scripts(command: &str) -> Vec<String> {
    let tokens = command_tokens(command);
    let mut scripts = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !FRONTEND_PACKAGE_MANAGERS.contains(&command_name_from_token(token).as_str())
            || tokens.get(index + 1).is_none_or(|value| value != "run")
        {
            continue;
        }
        if let Some(script) = script_name_after_run(&tokens, index + 2) {
            scripts.push(script);
        }
    }
    scripts
}

fn script_name_after_run(tokens: &[String], start: usize) -> Option<String> {
    let mut index = start;
    while tokens
        .get(index)
        .is_some_and(|token| token.starts_with('-'))
    {
        index += 1;
    }
    tokens.get(index).cloned()
}
