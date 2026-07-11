use super::super::{
    ExecuteCommand,
    args::CliOptions,
    errors::{Result, agent_error},
};
use super::generic::print_authenticated;

/// Public account activity route.
const ACCOUNT_ACTIVITY_PATH: &str = "/v1/account/activity";

#[derive(Clone, Copy, Debug)]
/// Top-level account command action.
pub(in crate::cli) struct AccountCommand;

impl ExecuteCommand for AccountCommand {
    fn execute(self, cli: &CliOptions) -> Result<()> {
        match cli.args().first().map_or("show", String::as_str) {
            "show" => return print_authenticated(cli, "/v1/account"),
            "activity" => return print_authenticated(cli, ACCOUNT_ACTIVITY_PATH),
            _ => {
                return Err(agent_error(
                    "unknown_account_command",
                    "Unknown account command.",
                    "Use `tovuk account show --json` or `tovuk account activity --json`.",
                    cli.output_format(),
                ));
            }
        }
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
