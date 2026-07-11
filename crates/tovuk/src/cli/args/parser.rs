#[cfg(test)]
#[path = "parser_tests.rs"]
/// Command-line parser tests.
mod tests;

use super::{CliOptions, flags};
use crate::cli::errors::{CliError, OutputFormat, Result, agent_error};
use std::env;

/// Marker after which every argument is positional.
const END_OF_OPTIONS: &str = "\x2d\x2d";
/// Validation policies for every supported value-bearing flag.
const FLAG_POLICIES: &[FlagPolicy] = &[
    FlagPolicy {
        is_used: |cli| return !cli.limit.is_empty(),
        name: "--limit",
        usage: FlagUsage::RequestListCreateResults,
    },
    FlagPolicy {
        is_used: |cli| return !cli.cursor.is_empty(),
        name: "--cursor",
        usage: FlagUsage::RequestListResults,
    },
    FlagPolicy {
        is_used: |cli| return !cli.top_up_usd_cents.is_empty(),
        name: "--top-up-usd-cents",
        usage: FlagUsage::BillingCheckout,
    },
    FlagPolicy {
        is_used: |cli| return !cli.token.is_empty(),
        name: "--token",
        usage: FlagUsage::Global,
    },
    FlagPolicy {
        is_used: |cli| return !cli.failing_command.is_empty(),
        name: "--failing-command",
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        is_used: |cli| return !cli.first_log_line.is_empty(),
        name: "--first-log-line",
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        is_used: |cli| return !cli.request_id.is_empty(),
        name: "--request-id",
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        is_used: |cli| return !cli.scraper_id.is_empty(),
        name: "--scraper-id",
        usage: FlagUsage::SupportCreate,
    },
    FlagPolicy {
        is_used: |cli| return !cli.severity.is_empty(),
        name: "--severity",
        usage: FlagUsage::SupportCreate,
    },
];
/// Environment variable selecting machine-readable output.
const OUTPUT_ENV: &str = "TOVUK_OUTPUT";

#[derive(Debug)]
/// Mutable context used while consuming one argument.
struct ArgumentInput<'input> {
    /// Current owned argument.
    argument: String,
    /// Complete command-line argument slice.
    argv: &'input [String],
    /// Options being populated.
    cli: &'input mut CliOptions,
    /// Index of the current argument.
    index: usize,
    /// Positional arguments collected so far.
    positional: &'input mut Vec<String>,
}

#[derive(Clone, Copy, Debug)]
/// Result of consuming one command-line argument.
enum ArgumentStep {
    /// Advance by the contained number of arguments.
    Advance(usize),
    /// Stop option parsing and preserve the remaining arguments.
    Stop,
}

impl<'input> TryFrom<ArgumentInput<'input>> for ArgumentStep {
    type Error = CliError;

    fn try_from(value: ArgumentInput<'input>) -> Result<Self> {
        if value.argument == END_OF_OPTIONS {
            value.positional.extend(
                value
                    .argv
                    .iter()
                    .skip(value.index.saturating_add(0b1))
                    .cloned(),
            );
            return Ok(Self::Stop);
        }
        if value.argument.starts_with('-') {
            let consumed = result_or_return!(flags::ApplyFlag::apply(
                flags::FlagApplication::from(flags::ParsedFlag::from(value.argument.as_str())),
                value.cli,
                value.argv,
                value.index,
            ));
            return Ok(Self::Advance(consumed));
        }
        value.positional.push(value.argument);
        return Ok(Self::Advance(0b1));
    }
}

impl TryFrom<&[String]> for CliOptions {
    type Error = CliError;

    fn try_from(value: &[String]) -> Result<Self> {
        let mut cli = Self::default();
        let OutputEnvironment = result_or_return!(OutputEnvironment::try_from(&mut cli));
        let mut positional = Vec::new();
        let mut index: usize = 0;
        while let Some(argument) = value.get(index).cloned() {
            let step = result_or_return!(ArgumentStep::try_from(ArgumentInput {
                argument,
                argv: value,
                cli: &mut cli,
                index,
                positional: &mut positional,
            }));
            match step {
                ArgumentStep::Advance(consumed) => index = index.saturating_add(consumed),
                ArgumentStep::Stop => break,
            }
        }
        if let Some(command) = positional.first() {
            cli.command.clone_from(command);
            cli.args = positional.into_iter().skip(0b1).collect();
        }
        cli.api_url
            .truncate(cli.api_url.trim_end_matches('/').len());
        let ValidatedOptions = result_or_return!(ValidatedOptions::try_from(&cli));
        return Ok(cli);
    }
}

/// Describes when one value-bearing flag is valid.
struct FlagPolicy {
    /// Predicate reporting whether the flag was supplied.
    is_used: FlagUsagePredicate,
    /// Public spelling of the flag.
    name: &'static str,
    /// Commands permitted to use the flag.
    usage: FlagUsage,
}

#[derive(Copy, Clone)]
/// Command scope accepted by a value-bearing flag.
enum FlagUsage {
    /// Billing checkout commands.
    BillingCheckout,
    /// Every command.
    Global,
    /// Request list, create, and results commands.
    RequestListCreateResults,
    /// Request list and results commands.
    RequestListResults,
    /// Support ticket creation.
    SupportCreate,
    /// Support ticket listing.
    SupportList,
}

impl FlagUsage {
    /// Reports whether this usage scope accepts the parsed command.
    fn allows(self, cli: &CliOptions) -> bool {
        match self {
            Self::Global => return true,
            Self::BillingCheckout => {
                return command_is(cli, "billing", &["checkout"])
                    || command_default_is(cli, "billing", "checkout");
            }
            Self::RequestListCreateResults => {
                return command_is(cli, "request", &["list", "create", "results"])
                    || command_default_is(cli, "request", "list")
                    || Self::SupportList.allows(cli);
            }
            Self::RequestListResults => {
                return command_is(cli, "request", &["list", "results"])
                    || command_default_is(cli, "request", "list")
                    || Self::SupportList.allows(cli);
            }
            Self::SupportCreate => return command_is(cli, "support", &["create"]),
            Self::SupportList => {
                return command_is(cli, "support", &["list"])
                    || command_default_is(cli, "support", "list");
            }
        }
    }
}

/// Predicate that detects whether a flag was supplied.
type FlagUsagePredicate = fn(&CliOptions) -> bool;

#[derive(Clone, Copy, Debug)]
/// Marker confirming output environment processing.
struct OutputEnvironment;

impl TryFrom<&mut CliOptions> for OutputEnvironment {
    type Error = CliError;

    fn try_from(value: &mut CliOptions) -> Result<Self> {
        let Ok(environment_value) = env::var(OUTPUT_ENV) else {
            return Ok(Self);
        };
        let trimmed_value = environment_value.trim();
        if trimmed_value.is_empty() {
            return Ok(Self);
        }
        result_or_return!(flags::set_output_format(
            value,
            trimmed_value,
            OUTPUT_ENV,
            OutputFormat::Text,
        ));
        return Ok(Self);
    }
}

#[derive(Clone, Copy, Debug)]
/// Marker confirming command-specific flag validation.
struct ValidatedOptions;

impl TryFrom<&CliOptions> for ValidatedOptions {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        let invalid_policy = FLAG_POLICIES
            .iter()
            .find(|policy| return (policy.is_used)(value) && !policy.usage.allows(value));
        let Some(policy) = invalid_policy else {
            return Ok(Self);
        };
        return Err(agent_error(
            "unknown_argument",
            format!("{} is not supported for this Tovuk command.", policy.name),
            "Run `tovuk --help`, remove flags that do not belong to this command, then retry.",
            value.output_format(),
        ));
    }
}

/// Reports whether the parsed command selects its default subcommand.
fn command_default_is(cli: &CliOptions, command: &str, default_subcommand: &str) -> bool {
    return cli.command == command
        && cli.args.first().map_or(default_subcommand, String::as_str) == default_subcommand;
}

/// Reports whether the parsed command matches one of the given subcommands.
fn command_is(cli: &CliOptions, command: &str, subcommands: &[&str]) -> bool {
    return cli.command == command
        && subcommands
            .iter()
            .any(|subcommand| return cli.args.first().map_or("", String::as_str) == *subcommand);
}

#[cfg(test)]
/// Parses arguments for cross-module unit tests.
///
/// # Errors
///
/// Returns an error when a supplied option or command-specific flag is invalid.
pub(in crate::cli) fn parse_args(argv: &[String]) -> Result<CliOptions> {
    return CliOptions::try_from(argv);
}
