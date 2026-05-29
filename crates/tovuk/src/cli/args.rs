use super::{
    constants::{DEFAULT_API_URL, DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS},
    errors::{Result, agent_error, internal_error},
};
use std::{env, path::PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct CliOptions {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) api_url: String,
    pub(crate) app: String,
    pub(crate) build: String,
    pub(crate) deploy: String,
    pub(crate) limit: String,
    pub(crate) cursor: String,
    pub(crate) content_type: String,
    pub(crate) failing_command: String,
    pub(crate) first_log_line: String,
    pub(crate) token: String,
    pub(crate) template: String,
    pub(crate) severity: String,
    pub(crate) port: u16,
    pub(crate) deployment: DeploymentOptions,
    pub(crate) storage: StorageOptions,
    pub(crate) output: OutputOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct DeploymentOptions {
    pub(crate) database: bool,
    pub(crate) wait: bool,
    pub(crate) wait_timeout_seconds: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct StorageOptions {
    pub(crate) public_read: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct OutputOptions {
    pub(crate) json: bool,
    pub(crate) help: bool,
    pub(crate) version: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            command: "help".to_owned(),
            args: Vec::new(),
            api_url: DEFAULT_API_URL.to_owned(),
            app: String::new(),
            build: String::new(),
            deploy: String::new(),
            limit: String::new(),
            cursor: String::new(),
            content_type: String::new(),
            failing_command: String::new(),
            first_log_line: String::new(),
            token: String::new(),
            template: String::new(),
            severity: String::new(),
            port: 0,
            deployment: DeploymentOptions {
                database: false,
                wait: false,
                wait_timeout_seconds: DEFAULT_DEPLOY_WAIT_TIMEOUT_SECONDS,
            },
            storage: StorageOptions { public_read: false },
            output: OutputOptions {
                json: false,
                help: false,
                version: false,
            },
        }
    }
}

pub(crate) fn parse_args(argv: &[String]) -> Result<CliOptions> {
    let mut cli = CliOptions::default();
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < argv.len() {
        let arg = argv[index].clone();
        if arg == "--" {
            positional.extend(argv.iter().skip(index + 1).cloned());
            break;
        }
        if let Some((name, inline)) = parse_flag(&arg) {
            let consumed = apply_flag(&mut cli, name, inline, argv, index)?;
            index += consumed;
        } else if arg.starts_with('-') {
            return Err(agent_error(
                "unknown_argument",
                format!("Unknown Tovuk option: {arg}."),
                "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
                cli.output.json,
            ));
        } else {
            positional.push(arg);
            index += 1;
        }
    }

    if let Some(command) = positional.first() {
        cli.command.clone_from(command);
        cli.args = positional.into_iter().skip(1).collect();
    }
    cli.api_url
        .truncate(cli.api_url.trim_end_matches('/').len());
    Ok(cli)
}

pub(crate) fn parse_flag(arg: &str) -> Option<(&str, Option<String>)> {
    if !arg.starts_with('-') {
        return None;
    }
    if arg.starts_with("--") {
        if let Some(index) = arg.find('=') {
            if index > 2 {
                return Some((&arg[..index], Some(arg[index + 1..].to_owned())));
            }
        }
    }
    Some((arg, None))
}

pub(crate) fn apply_flag(
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
            &mut cli.content_type,
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

pub(crate) fn set_string_flag(
    target: &mut String,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<usize> {
    *target = flag_value(name, inline, argv, index, json_output)?;
    Ok(flag_consumed(argv, index))
}

pub(crate) fn set_u16_flag(
    target: &mut u16,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<usize> {
    *target = parse_u16(
        &flag_value(name, inline, argv, index, json_output)?,
        name,
        json_output,
    )?;
    Ok(flag_consumed(argv, index))
}

pub(crate) fn set_u64_flag(
    target: &mut u64,
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<usize> {
    *target = parse_u64(
        &flag_value(name, inline, argv, index, json_output)?,
        name,
        json_output,
    )?;
    Ok(flag_consumed(argv, index))
}

pub(crate) fn set_boolean_flag(
    inline: Option<&String>,
    mut set: impl FnMut(),
    name: &str,
    json_output: bool,
) -> Result<usize> {
    if inline.is_some() {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} does not accept a value."),
            format!("Use {name} without =value."),
            json_output,
        ));
    }
    set();
    Ok(1)
}

pub(crate) fn flag_value(
    name: &str,
    inline: Option<String>,
    argv: &[String],
    index: usize,
    json_output: bool,
) -> Result<String> {
    let value = if let Some(value) = inline {
        value
    } else {
        argv.get(index + 1).cloned().ok_or_else(|| {
            agent_error(
                "missing_argument",
                format!("{name} requires a value."),
                format!("Pass a value after {name}."),
                json_output,
            )
        })?
    };
    if value.is_empty() || (!arg_has_inline_value(argv, index) && value.starts_with("--")) {
        return Err(agent_error(
            "missing_argument",
            format!("{name} requires a value."),
            format!("Pass a value after {name}."),
            json_output,
        ));
    }
    Ok(value)
}

pub(crate) fn flag_consumed(argv: &[String], index: usize) -> usize {
    if arg_has_inline_value(argv, index) {
        1
    } else {
        2
    }
}

pub(crate) fn arg_has_inline_value(argv: &[String], index: usize) -> bool {
    argv.get(index)
        .is_some_and(|arg| arg.starts_with("--") && arg.contains('='))
}

pub(crate) fn parse_u16(value: &str, name: &str, json_output: bool) -> Result<u16> {
    let parsed = value.parse::<u16>().map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as a positive integer."),
            json_output,
        )
    })?;
    if parsed == 0 {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as a positive integer."),
            json_output,
        ));
    }
    Ok(parsed)
}

pub(crate) fn parse_u64(value: &str, name: &str, json_output: bool) -> Result<u64> {
    let parsed = value.parse::<u64>().map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as seconds, for example {name} 900."),
            json_output,
        )
    })?;
    if parsed == 0 {
        return Err(agent_error(
            "invalid_argument",
            format!("{name} must be a positive integer."),
            format!("Pass {name} as seconds, for example {name} 900."),
            json_output,
        ));
    }
    Ok(parsed)
}

pub(crate) fn project_path(value: Option<&String>) -> Result<PathBuf> {
    let path = value.map_or_else(PathBuf::new, PathBuf::from);
    let path = if path.as_os_str().is_empty() {
        env::current_dir().map_err(|error| internal_error(error.to_string()))?
    } else if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|error| internal_error(error.to_string()))?
            .join(path)
    };
    Ok(path)
}
