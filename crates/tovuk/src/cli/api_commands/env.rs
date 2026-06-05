use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
    project::encode_component,
};
use super::{
    common::{command_arg, service_route},
    generic::{print_authenticated_mutation, service_get_command},
};
use reqwest::Method;
use serde_json::json;

#[derive(Clone, Copy)]
enum EnvCommandSurface {
    Env,
    Secrets,
}

pub(crate) fn env_command(cli: &CliOptions) -> Result<()> {
    scoped_env_command(cli, EnvCommandSurface::Env)
}

pub(crate) fn secrets_command(cli: &CliOptions) -> Result<()> {
    scoped_env_command(cli, EnvCommandSurface::Secrets)
}

fn scoped_env_command(cli: &CliOptions, surface: EnvCommandSurface) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => service_get_command(cli, "env"),
        "set" | "put" => env_set(cli, surface),
        "delete" => env_delete(cli, surface),
        _ => Err(agent_error(
            "unknown_command",
            format!("Unknown {} command.", surface.name()),
            surface.command_help(),
            cli.output.json,
        )),
    }
}

fn env_set(cli: &CliOptions, surface: EnvCommandSurface) -> Result<()> {
    let assignment = cli.args.get(1).cloned().unwrap_or_default();
    let separator = assignment.find('=').unwrap_or(0);
    if separator == 0 {
        return Err(agent_error(
            "invalid_env",
            format!("{} assignment must be KEY=value.", surface.item_name()),
            format!(
                "Pass one uppercase shell-safe {} assignment, for example `API_KEY=value`.",
                surface.item_name().to_lowercase()
            ),
            cli.output.json,
        ));
    }
    let name = &assignment[..separator];
    let value = &assignment[separator + 1..];
    print_authenticated_mutation(
        cli,
        Method::PUT,
        &service_route(cli, "env")?,
        Some(json!({ "name": name, "value": value })),
    )
}

fn env_delete(cli: &CliOptions, surface: EnvCommandSurface) -> Result<()> {
    let message = format!("{} name is required.", surface.item_name());
    let instruction = format!("Use `{}`.", surface.delete_example());
    let name = command_arg(cli, "invalid_env", &message, &instruction)?;
    print_authenticated_mutation(
        cli,
        Method::DELETE,
        &service_route(cli, &format!("env/{}", encode_component(&name)))?,
        None,
    )
}

impl EnvCommandSurface {
    fn command_help(self) -> &'static str {
        match self {
            Self::Env => "Use `tovuk env list`, `env set`, or `env delete`.",
            Self::Secrets => {
                "Use `tovuk secrets list`, `secrets set`, `secrets put`, or `secrets delete`."
            }
        }
    }

    fn delete_example(self) -> &'static str {
        match self {
            Self::Env => "tovuk env delete --service <service> KEY",
            Self::Secrets => "tovuk secrets delete --service <service> KEY",
        }
    }

    fn item_name(self) -> &'static str {
        match self {
            Self::Env => "Environment variable",
            Self::Secrets => "Secret",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Secrets => "secrets",
        }
    }
}
