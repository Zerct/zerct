/// Command-line flag parsing and assignment.
mod flags;
/// Command-line argument parsing and policy validation.
mod parser;
/// Shared flag value extraction helpers.
mod values;

use super::{constants::DEFAULT_API_URL, errors::OutputFormat};

#[cfg(test)]
pub(super) use parser::parse_args;

#[cfg(test)]
/// Builds validated options for contract tests.
///
/// # Panics
///
/// Panics when a test supplies arguments that violate the parser contract.
pub(super) fn options_for_test(values: &[&str]) -> CliOptions {
    let arguments = values.iter().map(ToString::to_string).collect::<Vec<_>>();
    let parsed = CliOptions::try_from(arguments.as_slice());
    assert!(parsed.is_ok(), "test CLI arguments should parse");
    return parsed.unwrap_or_default();
}

#[cfg(test)]
/// Builds test options targeting an explicit public API URL.
pub(super) fn options_with_api_url_for_test(api_url: String) -> CliOptions {
    return CliOptions {
        api_url,
        ..CliOptions::default()
    };
}

#[derive(Clone, Debug)]
/// Validated command-line options used by command handlers.
pub(super) struct CliOptions {
    /// Base URL for public API requests.
    api_url: String,
    /// Positional arguments following the command name.
    args: Vec<String>,
    /// Selected top-level command.
    command: String,
    /// Pagination cursor supplied by the user.
    cursor: String,
    /// Command associated with a support request.
    failing_command: String,
    /// First relevant log line supplied with a support request.
    first_log_line: String,
    /// Requested result page size.
    limit: String,
    /// Requested meta-level action.
    meta_action: MetaAction,
    /// Selected machine-readable or human-readable output format.
    output_format: OutputFormat,
    /// Request identifier associated with a support request.
    request_id: String,
    /// Data-source identifier associated with a command.
    scraper_id: String,
    /// Support request severity.
    severity: String,
    /// Explicit session token override.
    token: String,
    /// Requested billing balance top-up in United States dollar cents.
    top_up_usd_cents: String,
}

impl CliOptions {
    /// Returns the normalized public API base URL.
    pub(super) const fn api_url(&self) -> &str {
        return self.api_url.as_str();
    }

    /// Returns positional arguments after the command name.
    pub(super) const fn args(&self) -> &[String] {
        return self.args.as_slice();
    }

    /// Returns the selected top-level command.
    pub(super) const fn command(&self) -> &str {
        return self.command.as_str();
    }

    /// Returns the pagination cursor.
    pub(super) const fn cursor(&self) -> &str {
        return self.cursor.as_str();
    }

    /// Returns the command attached to a support request.
    pub(super) const fn failing_command(&self) -> &str {
        return self.failing_command.as_str();
    }

    /// Returns the first relevant support log line.
    pub(super) const fn first_log_line(&self) -> &str {
        return self.first_log_line.as_str();
    }

    /// Reports whether help output was requested.
    pub(super) const fn help_requested(&self) -> bool {
        return matches!(self.meta_action, MetaAction::Help);
    }

    /// Reports whether JSON output is active.
    pub(super) const fn is_json(&self) -> bool {
        return self.output_format.is_json();
    }

    /// Returns the requested result limit.
    pub(super) const fn limit(&self) -> &str {
        return self.limit.as_str();
    }

    /// Returns the selected output format.
    pub(super) const fn output_format(&self) -> OutputFormat {
        return self.output_format;
    }

    /// Returns the support request identifier.
    pub(super) const fn request_id(&self) -> &str {
        return self.request_id.as_str();
    }

    /// Returns the selected data-source identifier.
    pub(super) const fn scraper_id(&self) -> &str {
        return self.scraper_id.as_str();
    }

    /// Returns the requested support severity.
    pub(super) const fn severity(&self) -> &str {
        return self.severity.as_str();
    }

    /// Returns the explicit session token override.
    pub(super) const fn token(&self) -> &str {
        return self.token.as_str();
    }

    /// Returns the requested billing top-up in United States dollar cents.
    pub(super) const fn top_up_usd_cents(&self) -> &str {
        return self.top_up_usd_cents.as_str();
    }

    /// Reports whether version output was requested.
    pub(super) const fn version_requested(&self) -> bool {
        return matches!(self.meta_action, MetaAction::Version);
    }
}

impl Default for CliOptions {
    fn default() -> Self {
        return Self {
            api_url: DEFAULT_API_URL.to_owned(),
            args: Vec::new(),
            command: "help".to_owned(),
            cursor: String::new(),
            failing_command: String::new(),
            first_log_line: String::new(),
            limit: String::new(),
            meta_action: MetaAction::Run,
            output_format: OutputFormat::Text,
            request_id: String::new(),
            scraper_id: String::new(),
            severity: String::new(),
            token: String::new(),
            top_up_usd_cents: String::new(),
        };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Meta-level action requested before command dispatch.
enum MetaAction {
    /// Print command help.
    Help,
    /// Run the selected command.
    Run,
    /// Print the CLI version.
    Version,
}
