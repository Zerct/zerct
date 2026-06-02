use super::{
    model::CliOptions,
    values::{set_boolean_flag, set_string_flag, set_u64_flag},
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
        "--dry-run" => {
            set_boolean_flag(inline, || cli.deployment.dry_run = true, name, json_output)
        }
        "--public" => {
            set_boolean_flag(inline, || cli.storage.public_read = true, name, json_output)
        }
        "--private" => set_boolean_flag(
            inline,
            || cli.storage.public_read = false,
            name,
            json_output,
        ),
        "--clear-dead-letter-queue" => set_boolean_flag(
            inline,
            || cli.queue.clear_dead_letter_queue = true,
            name,
            json_output,
        ),
        "--operator" => set_boolean_flag(inline, || cli.abuse.operator = true, name, json_output),
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
        "--api"
        | "--service"
        | "--build"
        | "--deploy"
        | "--limit"
        | "--cursor"
        | "--period"
        | "--value"
        | "--notify-at-percent"
        | "--params"
        | "--target"
        | "--token"
        | "--template" => apply_common_value_flag(cli, name, inline, argv, index),
        "--content-type" => apply_storage_value_flag(cli, name, inline, argv, index),
        "--handle" | "--display-name" => apply_account_value_flag(cli, name, inline, argv, index),
        "--expiration" | "--expiration-ttl-seconds" | "--ttl" | "--metadata" => {
            apply_kv_value_flag(cli, name, inline, argv, index)
        }
        "--delay-seconds"
        | "--max-retries"
        | "--retention-seconds"
        | "--max-batch-size"
        | "--max-batch-timeout-seconds"
        | "--dead-letter-queue" => apply_queue_value_flag(cli, name, inline, argv, index),
        "--failing-command" | "--first-log-line" | "--severity" => {
            apply_support_value_flag(cli, name, inline, argv, index)
        }
        "--category" | "--reporter-email" | "--reporter-name" | "--evidence" | "--object-path"
        | "--target-path" => apply_abuse_value_flag(cli, name, inline, argv, index),
        "--wait-timeout" => apply_numeric_value_flag(cli, name, inline, argv, index),
        _ => Err(agent_error(
            "unknown_argument",
            format!("Unknown Tovuk option: {name}."),
            "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
            cli.output.json,
        )),
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
        "--service" => {
            set_string_flag(&mut cli.service, name, inline, argv, index, cli.output.json)
        }
        "--build" => set_string_flag(&mut cli.build, name, inline, argv, index, cli.output.json),
        "--deploy" => set_string_flag(&mut cli.deploy, name, inline, argv, index, cli.output.json),
        "--limit" => set_string_flag(&mut cli.limit, name, inline, argv, index, cli.output.json),
        "--cursor" => set_string_flag(&mut cli.cursor, name, inline, argv, index, cli.output.json),
        "--period" => set_string_flag(&mut cli.period, name, inline, argv, index, cli.output.json),
        "--value" => set_string_flag(&mut cli.value, name, inline, argv, index, cli.output.json),
        "--notify-at-percent" => set_string_flag(
            &mut cli.notify_at_percent,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--params" => set_string_flag(&mut cli.params, name, inline, argv, index, cli.output.json),
        "--target" => set_string_flag(&mut cli.target, name, inline, argv, index, cli.output.json),
        "--token" => set_string_flag(&mut cli.token, name, inline, argv, index, cli.output.json),
        "--template" => set_string_flag(
            &mut cli.template,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn apply_storage_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--content-type" => set_string_flag(
            &mut cli.storage.content_type,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
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

fn apply_kv_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--expiration" => set_string_flag(
            &mut cli.kv.expiration,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--expiration-ttl-seconds" | "--ttl" => set_string_flag(
            &mut cli.kv.expiration_ttl_seconds,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--metadata" => set_string_flag(
            &mut cli.kv.metadata,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn apply_queue_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--delay-seconds" => set_string_flag(
            &mut cli.queue.delay_seconds,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--max-retries" => set_string_flag(
            &mut cli.queue.max_retries,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--max-batch-size" => set_string_flag(
            &mut cli.queue.max_batch_size,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--max-batch-timeout-seconds" => set_string_flag(
            &mut cli.queue.max_batch_timeout_seconds,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--dead-letter-queue" => set_string_flag(
            &mut cli.queue.dead_letter_queue,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--retention-seconds" => set_string_flag(
            &mut cli.queue.retention_seconds,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn apply_support_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
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
        "--severity" => set_string_flag(
            &mut cli.severity,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn apply_abuse_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--category" => set_string_flag(
            &mut cli.abuse.category,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--reporter-email" => set_string_flag(
            &mut cli.abuse.reporter_email,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--reporter-name" => set_string_flag(
            &mut cli.abuse.reporter_name,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--evidence" => set_string_flag(
            &mut cli.abuse.evidence,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--object-path" => set_string_flag(
            &mut cli.abuse.object_path,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        "--target-path" => set_string_flag(
            &mut cli.abuse.target_path,
            name,
            inline,
            argv,
            index,
            cli.output.json,
        ),
        _ => invalid_value_flag_dispatch(cli, name),
    }
}

fn apply_numeric_value_flag(
    cli: &mut CliOptions,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
) -> Result<usize> {
    match name {
        "--wait-timeout" => set_u64_flag(
            &mut cli.deployment.wait_timeout_seconds,
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
