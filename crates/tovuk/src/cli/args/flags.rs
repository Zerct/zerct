use super::{
    model::CliOptions,
    values::{set_boolean_flag, set_string_flag},
};
use crate::cli::errors::{Result, agent_error};

const LONG_FLAG_PREFIX: &str = "\x2d\x2d";

pub(super) fn parse_flag(arg: &str) -> Option<(&str, Option<String>)> {
    if !arg.starts_with('-') {
        return None;
    }
    if arg.starts_with(LONG_FLAG_PREFIX)
        && let Some(index) = arg.find('=')
        && index > 2
    {
        return Some((&arg[..index], Some(arg[index + 1..].to_owned())));
    }
    Some((arg, None))
}

pub(super) fn apply_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    let json_output = cli.output.json;
    if let Some(consumed) = apply_boolean_flag(cli, name, inline.as_ref(), json_output)? {
        return Ok(consumed);
    }
    apply_value_flag(cli, name, inline, argv, index)
}

fn apply_boolean_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<&String>,
    json_output: bool,
) -> Result<Option<usize>> {
    match name {
        "--help" | "-h" => set_boolean_flag(inline, || cli.output.help = true, name, json_output),
        "--version" | "-v" | "-V" => {
            set_boolean_flag(inline, || cli.output.version = true, name, json_output)
        }
        "--json" => set_boolean_flag(inline, || cli.output.json = true, name, json_output),
        _ => return Ok(None),
    }
    .map(Some)
}

fn apply_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--api" | "--limit" | "--cursor" | "--token" | "--output" => {
            apply_common_value_flag(cli, name, inline, argv, index)
        }
        "--handle" | "--display-name" => apply_account_value_flag(cli, name, inline, argv, index),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn apply_common_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--api" => set_string_flag(&mut cli.api_url, name, inline, argv, index, cli.output.json),
        "--limit" => set_string_flag(&mut cli.limit, name, inline, argv, index, cli.output.json),
        "--cursor" => set_string_flag(&mut cli.cursor, name, inline, argv, index, cli.output.json),
        "--token" => set_string_flag(&mut cli.token, name, inline, argv, index, cli.output.json),
        "--output" => {
            let value = super::values::flag_value(name, inline, argv, index, cli.output.json)?;
            set_output_format(cli, &value, name, cli.output.json)?;
            Ok(super::values::flag_consumed(argv, index))
        }
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

pub(super) fn set_output_format(
    cli: &mut CliOptions,
    value: &str,
    source: &str,
    json_output: bool,
) -> Result<()> {
    if value.eq_ignore_ascii_case("json") {
        cli.output.json = true;
        return Ok(());
    }
    if value.eq_ignore_ascii_case("text") {
        cli.output.json = false;
        return Ok(());
    }
    Err(agent_error(
        "invalid_argument",
        format!("{source} must be `json` or `text`."),
        format!(
            "Set {source} to `json` for agent-readable output or `text` for human-readable output."
        ),
        json_output,
    ))
}

fn apply_account_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--handle" => set_string_flag(
            &mut cli.account.handle,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--display-name" => set_string_flag(
            &mut cli.account.display_name,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn invalid_value_flag_dispatch(cli: &CliOptions, name: &str) -> Result<usize> {
    Err(agent_error(
        "unknown_argument",
        format!("Unknown Tovuk option: {name}."),
        "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
        cli.output.json,
    ))
}
