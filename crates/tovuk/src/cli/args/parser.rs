use super::{flags, model::CliOptions};
use crate::cli::errors::{Result, agent_error};
use std::env;

const END_OF_OPTIONS: &str = "\x2d\x2d";
const OUTPUT_ENV: &str = "TOVUK_OUTPUT";

pub(crate) fn parse_args(argv: &[String]) -> Result<CliOptions> {
    let mut cli = CliOptions::default();
    apply_output_env(&mut cli)?;
    let mut positional = Vec::new();
    let mut index = 0usize;

    while index < argv.len() {
        let arg = argv[index].clone();
        if arg == END_OF_OPTIONS {
            positional.extend(argv.iter().skip(index + 1).cloned());
            break;
        }
        if let Some((name, inline)) = flags::parse_flag(&arg) {
            let consumed = flags::apply_flag(&mut cli, name, inline, argv, index)?;
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
    validate_command_flags(&cli)?;
    Ok(cli)
}

fn apply_output_env(cli: &mut CliOptions) -> Result<()> {
    let Ok(value) = env::var(OUTPUT_ENV) else {
        return Ok(());
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(());
    }
    flags::set_output_format(cli, value, OUTPUT_ENV, false)
}

fn validate_command_flags(cli: &CliOptions) -> Result<()> {
    for policy in FLAG_POLICIES {
        if (policy.is_used)(cli) && !policy.usage.allows(cli) {
            return Err(agent_error(
                "unknown_argument",
                format!("{} is not supported for this Tovuk command.", policy.name),
                "Run `tovuk --help`, remove flags that do not belong to this command, then retry.",
                cli.output.json,
            ));
        }
    }
    Ok(())
}

struct FlagPolicy {
    name: &'static str,
    is_used: fn(&CliOptions) -> bool,
    usage: FlagUsage,
}

#[derive(Copy, Clone)]
enum FlagUsage {
    Global,
    BillingCheckout,
    RequestListCreateResults,
    RequestListResults,
    SupportCreate,
    SupportList,
}

const FLAG_POLICIES: &[FlagPolicy] = &[
    FlagPolicy {
        name: "--limit",
        is_used: |cli| !cli.limit.is_empty(),
        usage: FlagUsage::RequestListCreateResults,
    },
    FlagPolicy {
        name: "--cursor",
        is_used: |cli| !cli.cursor.is_empty(),
        usage: FlagUsage::RequestListResults,
    },
    FlagPolicy {
        name: "--top-up-usd-cents",
        is_used: |cli| !cli.top_up_usd_cents.is_empty(),
        usage: FlagUsage::BillingCheckout,
    },
    FlagPolicy {
        name: "--token",
        is_used: |cli| !cli.token.is_empty(),
        usage: FlagUsage::Global,
    },
    FlagPolicy {
        name: "--failing-command",
        is_used: |cli| !cli.failing_command.is_empty(),
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        name: "--first-log-line",
        is_used: |cli| !cli.first_log_line.is_empty(),
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        name: "--request-id",
        is_used: |cli| !cli.request_id.is_empty(),
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        name: "--scraper-id",
        is_used: |cli| !cli.scraper_id.is_empty(),
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        name: "--severity",
        is_used: |cli| !cli.severity.is_empty(),
        usage: FlagUsage::SupportCreate,
    },
];

impl FlagUsage {
    fn allows(self, cli: &CliOptions) -> bool {
        match self {
            Self::Global => true,
            Self::BillingCheckout => {
                command_is(cli, "billing", &["checkout"])
                    || command_default_is(cli, "billing", "checkout")
            }
            Self::RequestListCreateResults => {
                command_is(cli, "request", &["list", "create", "results"])
                    || command_default_is(cli, "request", "list")
                    || Self::SupportList.allows(cli)
            }
            Self::RequestListResults => {
                command_is(cli, "request", &["list", "results"])
                    || command_default_is(cli, "request", "list")
                    || Self::SupportList.allows(cli)
            }
            Self::SupportCreate => command_is(cli, "support", &["create"]),
            Self::SupportList => {
                command_is(cli, "support", &["list"]) || command_default_is(cli, "support", "list")
            }
        }
    }
}

fn command_is(cli: &CliOptions, command: &str, subcommands: &[&str]) -> bool {
    cli.command == command
        && subcommands
            .iter()
            .any(|subcommand| cli.args.first().map_or("", String::as_str) == *subcommand)
}

fn command_default_is(cli: &CliOptions, command: &str, default_subcommand: &str) -> bool {
    cli.command == command
        && cli.args.first().map_or(default_subcommand, String::as_str) == default_subcommand
}

#[cfg(test)]
#[path = "parser_tests.rs"]
mod tests;
