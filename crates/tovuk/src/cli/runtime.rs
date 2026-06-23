use super::{
    api_commands::{
        abuse_command, account_command, billing_command, pricing, print_authenticated,
        request_command, scraper_command, support_command,
    },
    args::parse_args,
    auth::login,
    constants::VERSION,
    errors::{Result, agent_error},
    help::help_text,
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
        "login" => login(&cli),
        "pricing" => pricing(&cli),
        "scraper" => scraper_command(&cli),
        "request" => request_command(&cli),
        "account" => account_command(&cli),
        "usage" => print_authenticated(&cli, "/v1/usage"),
        "billing" => billing_command(&cli),
        "support" => support_command(&cli),
        "abuse" => abuse_command(&cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown Tovuk command.",
            "Run `tovuk --help` and retry with a supported command.",
            cli.output.json,
        )),
    }?;

    Ok(ExitCode::SUCCESS)
}
