use super::{
    CliOptions, MetaAction,
    values::{FlagValue, arg_has_inline_value, set_boolean_flag},
};
use crate::cli::errors::{OutputFormat, Result, agent_error};

/// Prefix used by long-form command-line flags.
const LONG_FLAG_PREFIX: &str = "\x2d\x2d";

/// Applies a boolean flag to parsed CLI options.
trait ApplyBooleanFlag {
    /// Validates and applies the boolean flag.
    ///
    /// # Errors
    ///
    /// Returns an error when the flag incorrectly includes a value.
    fn apply(self, cli: &mut CliOptions) -> Result<usize>;
}

/// Applies a parsed flag while tracking consumed arguments.
pub(super) trait ApplyFlag {
    /// Applies the flag and returns the number of consumed arguments.
    ///
    /// # Errors
    ///
    /// Returns an error when the flag or its value is invalid.
    fn apply(self, cli: &mut CliOptions, argv: &[String], index: usize) -> Result<usize>;
}

/// Applies a flag that requires a value.
trait ApplyValueFlag {
    /// Parses and assigns the flag value, returning consumed arguments.
    ///
    /// # Errors
    ///
    /// Returns an error when the flag is unknown, missing a value, or invalid.
    fn apply(self, cli: &mut CliOptions, argv: &[String], index: usize) -> Result<usize>;
}

/// Assigns an already parsed flag value to its destination option.
trait AssignFlagValue {
    /// Assigns the value selected by the flag action.
    ///
    /// # Errors
    ///
    /// Returns an error when an output format value is unsupported.
    fn assign(self, cli: &mut CliOptions) -> Result<()>;
}

#[derive(Debug)]
/// Parsed value ready to be assigned to the matching CLI option.
struct AssignedFlagValue {
    /// Destination option selected by the flag name.
    action: FlagAction,
    /// Output format active while emitting validation errors.
    output_format: OutputFormat,
    /// Original flag name used in diagnostics.
    source: String,
    /// Parsed flag value.
    value: String,
}

impl AssignFlagValue for AssignedFlagValue {
    fn assign(self, cli: &mut CliOptions) -> Result<()> {
        match self.action {
            FlagAction::Cursor => cli.cursor = self.value,
            FlagAction::FailingCommand => cli.failing_command = self.value,
            FlagAction::FirstLogLine => cli.first_log_line = self.value,
            FlagAction::Limit => cli.limit = self.value,
            FlagAction::Output => {
                return set_output_format(
                    cli,
                    self.value.as_str(),
                    self.source.as_str(),
                    self.output_format,
                );
            }
            FlagAction::RequestId => cli.request_id = self.value,
            FlagAction::ScraperId => cli.scraper_id = self.value,
            FlagAction::Severity => cli.severity = self.value,
            FlagAction::Token => cli.token = self.value,
            FlagAction::TopUpUsdCents => cli.top_up_usd_cents = self.value,
            FlagAction::Help | FlagAction::Json | FlagAction::Unknown | FlagAction::Version => {
                return Ok(());
            }
        }
        return Ok(());
    }
}

#[derive(Debug)]
/// Boolean flag awaiting application to CLI options.
struct BooleanFlagApplication {
    /// Boolean action selected by the flag name.
    action: FlagAction,
    /// Unsupported inline value, when one was supplied.
    inline: Option<String>,
    /// Original flag name used in diagnostics.
    name: String,
}

impl ApplyBooleanFlag for BooleanFlagApplication {
    fn apply(self, cli: &mut CliOptions) -> Result<usize> {
        let output_format = cli.output_format();
        match self.action {
            FlagAction::Help => {
                return set_boolean_flag(
                    self.inline.as_ref(),
                    || cli.meta_action = MetaAction::Help,
                    self.name.as_str(),
                    output_format,
                );
            }
            FlagAction::Json => {
                return set_boolean_flag(
                    self.inline.as_ref(),
                    || cli.output_format = OutputFormat::Json,
                    self.name.as_str(),
                    output_format,
                );
            }
            FlagAction::Version => {
                return set_boolean_flag(
                    self.inline.as_ref(),
                    || cli.meta_action = MetaAction::Version,
                    self.name.as_str(),
                    output_format,
                );
            }
            FlagAction::Cursor
            | FlagAction::FailingCommand
            | FlagAction::FirstLogLine
            | FlagAction::Limit
            | FlagAction::Output
            | FlagAction::RequestId
            | FlagAction::ScraperId
            | FlagAction::Severity
            | FlagAction::Token
            | FlagAction::TopUpUsdCents
            | FlagAction::Unknown => return Ok(0b1),
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// Action associated with a recognized flag name.
enum FlagAction {
    /// Set the pagination cursor.
    Cursor,
    /// Set the command attached to a support request.
    FailingCommand,
    /// Set the first relevant support log line.
    FirstLogLine,
    /// Request help output.
    Help,
    /// Select JSON output.
    Json,
    /// Set the result limit.
    Limit,
    /// Select an output format by name.
    Output,
    /// Set the support request identifier.
    RequestId,
    /// Set the data-source identifier.
    ScraperId,
    /// Set the support severity.
    Severity,
    /// Set an explicit session token.
    Token,
    /// Set a billing top-up amount in United States dollar cents.
    TopUpUsdCents,
    /// Represent an unsupported flag.
    Unknown,
    /// Request version output.
    Version,
}

impl From<&str> for FlagAction {
    fn from(value: &str) -> Self {
        match value {
            "--cursor" => return Self::Cursor,
            "--failing-command" => return Self::FailingCommand,
            "--first-log-line" => return Self::FirstLogLine,
            "--help" | "-h" => return Self::Help,
            "--json" => return Self::Json,
            "--limit" => return Self::Limit,
            "--output" => return Self::Output,
            "--request-id" => return Self::RequestId,
            "--scraper-id" => return Self::ScraperId,
            "--severity" => return Self::Severity,
            "--token" => return Self::Token,
            "--top-up-usd-cents" => return Self::TopUpUsdCents,
            "--version" | "-v" | "-V" => return Self::Version,
            _ => return Self::Unknown,
        }
    }
}

#[derive(Debug)]
/// Parsed flag ready for semantic application.
pub(super) struct FlagApplication {
    /// Parsed flag name and optional inline value.
    parsed: ParsedFlag,
}

impl ApplyFlag for FlagApplication {
    fn apply(self, cli: &mut CliOptions, argv: &[String], index: usize) -> Result<usize> {
        let ParsedFlag {
            action,
            inline,
            name,
        } = self.parsed;
        match action {
            FlagAction::Help | FlagAction::Json | FlagAction::Version => {
                return ApplyBooleanFlag::apply(
                    BooleanFlagApplication {
                        action,
                        inline,
                        name,
                    },
                    cli,
                );
            }
            FlagAction::Cursor
            | FlagAction::FailingCommand
            | FlagAction::FirstLogLine
            | FlagAction::Limit
            | FlagAction::Output
            | FlagAction::RequestId
            | FlagAction::ScraperId
            | FlagAction::Severity
            | FlagAction::Token
            | FlagAction::TopUpUsdCents
            | FlagAction::Unknown => {
                return ApplyValueFlag::apply(
                    ValueFlagApplication {
                        action,
                        inline,
                        name,
                    },
                    cli,
                    argv,
                    index,
                );
            }
        }
    }
}

impl From<ParsedFlag> for FlagApplication {
    fn from(value: ParsedFlag) -> Self {
        return Self { parsed: value };
    }
}

#[derive(Debug)]
/// Flag action plus its original spelling and optional inline value.
pub(super) struct ParsedFlag {
    /// Action selected by the flag name.
    action: FlagAction,
    /// Value supplied through `--name=value` syntax.
    inline: Option<String>,
    /// Original flag name.
    name: String,
}

impl From<&str> for ParsedFlag {
    fn from(value: &str) -> Self {
        let (name, inline) = if value.starts_with(LONG_FLAG_PREFIX)
            && let Some((name, inline)) = value.split_once('=')
            && name.len() > 0b10
        {
            (name.to_owned(), Some(inline.to_owned()))
        } else {
            (value.to_owned(), None)
        };
        return Self {
            action: FlagAction::from(name.as_str()),
            inline,
            name,
        };
    }
}

#[derive(Debug)]
/// Value-bearing flag awaiting application to CLI options.
struct ValueFlagApplication {
    /// Value action selected by the flag name.
    action: FlagAction,
    /// Value supplied through `--name=value` syntax.
    inline: Option<String>,
    /// Original flag name used in diagnostics.
    name: String,
}

impl ApplyValueFlag for ValueFlagApplication {
    fn apply(self, cli: &mut CliOptions, argv: &[String], index: usize) -> Result<usize> {
        let output_format = cli.output_format();
        if matches!(self.action, FlagAction::Unknown) {
            return Err(agent_error(
                "unknown_argument",
                format!("Unknown Tovuk option: {}.", self.name),
                "Run `tovuk --help`, remove or correct the unsupported option, then retry.",
                output_format,
            ));
        }
        let source = self.name.clone();
        let value = String::from(result_or_return!(FlagValue::try_from((
            self.name,
            self.inline,
            argv,
            index,
            output_format,
        ))));
        result_or_return!(AssignFlagValue::assign(
            AssignedFlagValue {
                action: self.action,
                output_format,
                source,
                value,
            },
            cli,
        ));
        if arg_has_inline_value(argv, index) {
            return Ok(0b1);
        }
        return Ok(0b10);
    }
}

/// Selects JSON output from an explicit output-format value.
///
/// # Errors
///
/// Returns an error when `value` does not name the supported JSON format.
pub(super) fn set_output_format(
    cli: &mut CliOptions,
    value: &str,
    source: &str,
    output_format: OutputFormat,
) -> Result<()> {
    if value.eq_ignore_ascii_case("json") {
        cli.output_format = OutputFormat::Json;
        return Ok(());
    }
    return Err(agent_error(
        "invalid_argument",
        format!("{source} must be `json`."),
        format!("Set {source} to `json`, or omit it for default command output."),
        output_format,
    ));
}
