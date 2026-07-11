use super::{
    ExecuteCommand,
    api_commands::{
        AccountCommand, ApiKeyCommand, BillingCommand, PricingCommand, RequestCommand,
        ScraperCommand, SupportCommand, print_authenticated,
    },
    args::CliOptions,
    auth::LoginCommand,
    constants::VERSION,
    errors::{CliError, Result, agent_error, write_stdout_line},
    help::help_text,
};
use std::{env, process::ExitCode};

impl From<CliError> for ExitCode {
    #[inline]
    fn from(value: CliError) -> Self {
        if value.print().is_err() {
            return Self::FAILURE;
        }
        return Self::from(value.exit_code());
    }
}

impl TryFrom<RuntimeExecution> for ExitCode {
    type Error = CliError;

    #[inline]
    fn try_from(value: RuntimeExecution) -> Result<Self> {
        let RuntimeExecution = value;
        let argv = env::args().skip(0b1).collect::<Vec<_>>();
        let cli = result_or_return!(CliOptions::try_from(argv.as_slice()));
        if cli.help_requested() {
            result_or_return!(write_stdout_line(&help_text()));
            return Ok(Self::SUCCESS);
        }
        if cli.version_requested() {
            result_or_return!(write_stdout_line(VERSION));
            return Ok(Self::SUCCESS);
        }
        result_or_return!(match cli.command() {
            "help" => write_stdout_line(&help_text()),
            "login" => ExecuteCommand::execute(LoginCommand, &cli),
            "pricing" => ExecuteCommand::execute(PricingCommand, &cli),
            "scraper" => ExecuteCommand::execute(ScraperCommand, &cli),
            "request" => ExecuteCommand::execute(RequestCommand, &cli),
            "account" => ExecuteCommand::execute(AccountCommand, &cli),
            "api-key" => ExecuteCommand::execute(ApiKeyCommand, &cli),
            "usage" => print_authenticated(&cli, "/v1/usage"),
            "billing" => ExecuteCommand::execute(BillingCommand, &cli),
            "support" => ExecuteCommand::execute(SupportCommand, &cli),
            _ => Err(agent_error(
                "unknown_command",
                "Unknown Tovuk command.",
                "Run `tovuk --help` and retry with a supported command.",
                cli.output_format(),
            )),
        });
        return Ok(Self::SUCCESS);
    }
}

/// Runs the CLI process and converts failures into exit statuses.
pub(super) trait RunRuntime {
    /// Executes the runtime and returns its process exit status.
    fn run(self) -> ExitCode;
}

#[derive(Clone, Copy, Debug)]
/// Runtime action exposed to the crate entrypoint.
pub(super) struct Runtime;

impl RunRuntime for Runtime {
    fn run(self) -> ExitCode {
        match ExitCode::try_from(RuntimeExecution) {
            Ok(code) => return code,
            Err(error) => return ExitCode::from(error),
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// Internal runtime execution marker.
struct RuntimeExecution;
