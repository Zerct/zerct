use super::super::{
    args::CliOptions,
    errors::{Result, agent_error},
};
use super::generic::print_paged_authenticated;

const ACCOUNT_ACTIVITY_PATH: &str = "/v1/account/activity";

pub(crate) fn account_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("show", String::as_str) {
        "show" => print_paged_authenticated(cli, "/v1/account"),
        "activity" => print_paged_authenticated(cli, ACCOUNT_ACTIVITY_PATH),
        _ => Err(agent_error(
            "unknown_account_command",
            "Unknown account command.",
            "Use `tovuk account show --json` or `tovuk account activity --json`.",
            cli.output.json,
        )),
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
