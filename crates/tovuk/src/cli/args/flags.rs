use super::{
    model::CliOptions,
    values::{set_boolean_flag, set_string_flag, set_u16_flag, set_u64_flag},
};
use crate::cli::errors::{Result, agent_error};

pub(super) fn parse_flag(arg: &str) -> Option<(&str, Option<String>)> {
    if !arg.starts_with('-') {
        return None;
    }
    if arg.starts_with("--")
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
        "--database" => {
            set_boolean_flag(inline, || cli.deployment.database = true, name, json_output)
        }
        "--no-database" => set_boolean_flag(
            inline,
            || cli.deployment.database = false,
            name,
            json_output,
        ),
        "--wait" => set_boolean_flag(inline, || cli.deployment.wait = true, name, json_output),
        "--public" => {
            set_boolean_flag(inline, || cli.storage.public_read = true, name, json_output)
        }
        "--private" => set_boolean_flag(
            inline,
            || cli.storage.public_read = false,
            name,
            json_output,
        ),
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
        "--api" => set_string_flag(&mut cli.api_url, name, inline, argv, index, cli.output.json),
        "--app" => set_string_flag(&mut cli.app, name, inline, argv, index, cli.output.json),
        "--build" => set_string_flag(&mut cli.build, name, inline, argv, index, cli.output.json),
        "--content-type" => set_string_flag(
            &mut cli.storage.content_type,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--deploy" => set_string_flag(&mut cli.deploy, name, inline, argv, index, cli.output.json),
        "--failing-command" => set_string_flag(
            &mut cli.failing_command,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--first-log-line" => set_string_flag(
            &mut cli.first_log_line,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--limit" => set_string_flag(&mut cli.limit, name, inline, argv, index, cli.output.json),
        "--cursor" => set_string_flag(&mut cli.cursor, name, inline, argv, index, cli.output.json),
        "--period" => set_string_flag(&mut cli.period, name, inline, argv, index, cli.output.json),
        "--value" => set_string_flag(&mut cli.value, name, inline, argv, index, cli.output.json),
        "--target" => set_string_flag(&mut cli.target, name, inline, argv, index, cli.output.json),
        "--severity" => set_string_flag(
            &mut cli.severity,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--token" => set_string_flag(&mut cli.token, name, inline, argv, index, cli.output.json),
        "--template" => set_string_flag(
            &mut cli.template,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--port" => set_u16_flag(&mut cli.port, name, inline, argv, index, cli.output.json),
        "--wait-timeout" => set_u64_flag(
            &mut cli.deployment.wait_timeout_seconds,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => Err(agent_error(
            "unknown_argument",
            format!("Unknown Tovuk option: {name}."),
            "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
            cli.output.json,
        )),
    }
}
