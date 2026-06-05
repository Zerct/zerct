use super::{
    api_commands::{
        abuse_command, account_command, billing_command, binding_command, caps_command,
        cron_command, domains_command, env_command, kv_command, logs_command, nodes_command,
        pricing, print_authenticated, queue_command, service_command, sqlite_command,
        state_command, storage_command, support_command,
    },
    args::{parse_args, project_path},
    auth::login,
    check::check_project,
    constants::VERSION,
    deploy::deploy,
    dev::dev,
    errors::{Result, agent_error},
    help::help_text,
    templates::new_project,
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
        "help" => {
            println!("{}", help_text());
            Ok(())
        }
        "new" => new_project(&project_path(cli.args.first())?, &cli.template),
        "check" => check_project(&project_path(cli.args.first())?, cli.output.json),
        "dev" => dev(&project_path(cli.args.first())?, &cli),
        "login" => login(&cli),
        "deploy" => deploy(&project_path(cli.args.first())?, &cli),
        "pricing" => pricing(&cli),
        "account" => account_command(&cli),
        "usage" => print_authenticated(&cli, "/v1/usage"),
        "service" => service_command(&cli),
        "logs" => logs_command(&cli),
        "sqlite" => sqlite_command(&cli),
        "kv" => kv_command(&cli),
        "queue" => queue_command(&cli),
        "cron" => cron_command(&cli),
        "state" => state_command(&cli),
        "binding" => binding_command(&cli),
        "limits" => caps_command(&cli),
        "env" => env_command(&cli),
        "domains" => domains_command(&cli),
        "storage" => storage_command(&cli),
        "billing" => billing_command(&cli),
        "support" => support_command(&cli),
        "abuse" => abuse_command(&cli),
        "nodes" => nodes_command(&cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown Tovuk command.",
            "Run `tovuk --help` and retry with a supported command.",
            cli.output.json,
        )),
    }?;

    Ok(ExitCode::SUCCESS)
}
