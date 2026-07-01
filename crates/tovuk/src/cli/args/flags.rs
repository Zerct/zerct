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
    if name == "--output" {
        return apply_output_value_flag(cli, name, inline, argv, index);
    }
    let json_output = cli.output.json;
    let Some(target) = string_flag_target(cli, name) else {
        return invalid_value_flag_dispatch(cli, name);
    };
    set_string_flag(target, name, inline, argv, index, json_output)
}

fn apply_output_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    let json_output = cli.output.json;
    let value = super::values::flag_value(name, inline, argv, index, json_output)?;
    set_output_format(cli, value.as_str(), name, json_output)?;
    Ok(super::values::flag_consumed(argv, index))
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

fn invalid_value_flag_dispatch(cli: &CliOptions, name: &str) -> Result<usize> {
    Err(agent_error(
        "unknown_argument",
        format!("Unknown Tovuk option: {name}."),
        "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
        cli.output.json,
    ))
}

fn string_flag_target<'a>(cli: &'a mut CliOptions, name: &str) -> Option<&'a mut String> {
    match name {
        "--api" => Some(&mut cli.api_url),
        "--limit" => Some(&mut cli.limit),
        "--cursor" => Some(&mut cli.cursor),
        "--token" => Some(&mut cli.token),
        "--failing-command" => Some(&mut cli.failing_command),
        "--first-log-line" => Some(&mut cli.first_log_line),
        "--request-id" => Some(&mut cli.request_id),
        "--scraper-id" => Some(&mut cli.scraper_id),
        "--severity" => Some(&mut cli.severity),
        _ => None,
    }
}
