use super::{
    api_commands::{
        app_get_command, app_route, billing_command, builds_command, capabilities, deploys_command,
        domains_command, env_command, logs_command, print_authenticated, print_paged_authenticated,
        storage_command, support_command,
    },
    args::{parse_args, project_path},
    auth::login,
    constants::VERSION,
    deploy::deploy,
    doctor::doctor_project,
    errors::{Result, agent_error},
    help::help_text,
    preview::preview_project,
    templates::{init_project, install_project},
};
use std::{env, process::ExitCode};

pub(crate) fn runtime_entrypoint() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            error.print();
            ExitCode::from(error.exit_code())
        }
    }
}

pub(crate) fn run() -> Result<ExitCode> {
    let argv = env::args().skip(1).collect::<Vec<_>>();
    let cli = parse_args(&argv)?;
    if cli.output.help {
        println!("{}", help_text());
        return Ok(ExitCode::SUCCESS);
    }
    if cli.output.version {
        println!("{VERSION}");
        return Ok(ExitCode::SUCCESS);
    }

    match cli.command.as_str() {
        "init" => init_project(&project_path(cli.args.first())?, &cli.template),
        "install" => install_project(&project_path(cli.args.first())?, &cli.template),
        "doctor" => doctor_project(&project_path(cli.args.first())?, cli.output.json),
        "preview" => preview_project(&project_path(cli.args.first())?, cli.port),
        "login" => login(&cli),
        "deploy" => deploy(&project_path(cli.args.first())?, &cli),
        "capabilities" => capabilities(&cli),
        "me" => print_authenticated(&cli, "/v1/me"),
        "usage" => print_authenticated(&cli, "/v1/usage"),
        "activity" => print_paged_authenticated(&cli, "/v1/activity"),
        "apps" => print_authenticated(&cli, "/v1/apps"),
        "overview" => print_paged_authenticated(&cli, &app_route(&cli, "overview")?),
        "deploys" => deploys_command(&cli),
        "builds" => builds_command(&cli),
        "logs" => logs_command(&cli),
        "status" => app_get_command(&cli, "status"),
        "inspect" => app_get_command(&cli, "inspect"),
        "db" | "database" => app_get_command(&cli, "database"),
        "env" => env_command(&cli),
        "domains" => domains_command(&cli),
        "storage" | "files" | "media" => storage_command(&cli),
        "billing" => billing_command(&cli),
        "support" => support_command(&cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown Tovuk command.",
            "Run `tovuk --help` and retry with a supported command.",
            cli.output.json,
        )),
    }?;

    Ok(ExitCode::SUCCESS)
}
