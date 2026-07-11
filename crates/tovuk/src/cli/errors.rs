use core::result::Result as CoreResult;
use serde::{Deserialize, Serialize};
use serde_json::{Value, to_string_pretty};
use std::io::{self as standard_io, Write as _};

#[derive(Debug)]
/// Optional links and recovery guidance attached to an error.
pub(in crate::cli) struct AgentErrorContext {
    /// Recommended recovery action.
    agent_instruction: Option<String>,
    /// Optional billing URL.
    checkout_url: Option<String>,
    /// Optional public documentation URL.
    docs_url: Option<String>,
}

impl AgentErrorContext {
    /// Creates optional error recovery context.
    pub(in crate::cli) const fn new(
        agent_instruction: Option<String>,
        docs_url: Option<String>,
        checkout_url: Option<String>,
    ) -> Self {
        return Self {
            agent_instruction,
            checkout_url,
            docs_url,
        };
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Stable machine-readable error returned to automation clients.
pub(super) struct AgentErrorPayload {
    /// Recommended recovery action.
    agent_instruction: Option<String>,
    /// Optional billing URL associated with the failure.
    checkout_url: Option<String>,
    /// Stable error code.
    code: String,
    /// Optional public documentation URL.
    docs_url: Option<String>,
    /// Human-readable failure summary.
    message: String,
}

impl AgentErrorPayload {
    #[cfg(test)]
    /// Returns the stable error code.
    pub(super) const fn code(&self) -> &str {
        return self.code.as_str();
    }

    #[cfg(test)]
    /// Returns the optional public documentation URL.
    pub(super) fn docs_url(&self) -> Option<&str> {
        return self.docs_url.as_deref();
    }

    #[cfg(test)]
    /// Returns the human-readable error message.
    pub(super) const fn message(&self) -> &str {
        return self.message.as_str();
    }

    /// Creates a stable error payload.
    pub(in crate::cli) fn new(code: String, message: String, context: AgentErrorContext) -> Self {
        return Self {
            agent_instruction: context.agent_instruction,
            checkout_url: context.checkout_url,
            code,
            docs_url: context.docs_url,
            message,
        };
    }
}

#[derive(Debug)]
/// Compact command-line error value.
pub(super) struct CliError(Box<CliFailure>);

impl CliError {
    /// Returns the process exit code associated with the failure.
    pub(super) const fn exit_code(&self) -> u8 {
        return self.0.exit_code;
    }

    #[cfg(test)]
    /// Returns the human-readable error message.
    pub(super) fn message(&self) -> &str {
        return self.0.payload.message.as_str();
    }

    /// Creates an error with explicit payload, format, and exit code.
    pub(in crate::cli) fn new(
        payload: AgentErrorPayload,
        output_format: OutputFormat,
        exit_code: u8,
    ) -> Self {
        return Self(Box::new(CliFailure {
            exit_code,
            output_format,
            payload,
        }));
    }

    #[cfg(test)]
    /// Returns the stable error payload.
    pub(super) fn payload(&self) -> &AgentErrorPayload {
        return &self.0.payload;
    }

    /// Prints the error using its selected output format.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the diagnostic cannot be written to standard error.
    pub(super) fn print(&self) -> standard_io::Result<()> {
        let payload = &self.0.payload;
        if self.0.output_format.is_json() {
            return match to_string_pretty(payload) {
                Ok(source) => write_stderr_line(&source),
                Err(error) => write_stderr_line(&format!("Tovuk command failed: {error}")),
            };
        }
        result_or_return!(write_stderr_line(&payload.message));
        if let Some(instruction) = payload
            .agent_instruction
            .as_deref()
            .filter(|value| return !value.is_empty())
        {
            result_or_return!(write_stderr_line(&format!(
                "agent_instruction: {instruction}"
            )));
        }
        if let Some(docs_url) = payload
            .docs_url
            .as_deref()
            .filter(|value| return !value.is_empty())
        {
            result_or_return!(write_stderr_line(&format!("docs: {docs_url}")));
        }
        if let Some(checkout_url) = payload
            .checkout_url
            .as_deref()
            .filter(|value| return !value.is_empty())
        {
            result_or_return!(write_stderr_line(&format!("checkout: {checkout_url}")));
        }
        return Ok(());
    }
}

#[derive(Debug)]
/// Internal error state including output and process-exit policy.
struct CliFailure {
    /// Process exit code.
    exit_code: u8,
    /// Format used when printing the error.
    output_format: OutputFormat,
    /// Stable error payload.
    payload: AgentErrorPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Supported command output formats.
pub(in crate::cli) enum OutputFormat {
    /// Machine-readable JSON output.
    Json,
    /// Human-readable text output.
    Text,
}

impl OutputFormat {
    /// Reports whether the format is machine-readable JSON.
    pub(in crate::cli) const fn is_json(self) -> bool {
        return matches!(self, Self::Json);
    }
}

/// Result type used throughout the CLI.
pub(super) type Result<T> = CoreResult<T, CliError>;

/// Creates a user-actionable CLI error.
pub(super) fn agent_error(
    code: impl Into<String>,
    message: impl Into<String>,
    agent_instruction: impl Into<String>,
    output_format: OutputFormat,
) -> CliError {
    return agent_error_with_context(
        code,
        message,
        AgentErrorContext::new(Some(agent_instruction.into()), None, None),
        output_format,
    );
}

/// Creates a user-actionable CLI error with optional links and billing context.
pub(super) fn agent_error_with_context(
    code: impl Into<String>,
    message: impl Into<String>,
    context: AgentErrorContext,
    output_format: OutputFormat,
) -> CliError {
    return CliError::new(
        AgentErrorPayload::new(code.into(), message.into(), context),
        output_format,
        0b1,
    );
}

/// Creates an internal command failure with standard recovery guidance.
pub(super) fn internal_error(message: impl Into<String>) -> CliError {
    return agent_error(
        "internal_error",
        message.into(),
        "Retry the command. If it keeps failing, create a Tovuk support ticket with command output.",
        OutputFormat::Text,
    );
}

/// Writes a JSON value to standard output in a stable pretty-printed form.
///
/// # Errors
///
/// Returns an error when serialization or standard-output writing fails.
pub(super) fn print_json(value: &Value) -> Result<()> {
    let source = result_or_return!(
        to_string_pretty(value).map_err(|error| return internal_error(error.to_string()))
    );
    return write_stdout_line(&source);
}

/// Writes one line to standard error.
///
/// # Errors
///
/// Returns an I/O error when standard error cannot be written.
pub(in crate::cli) fn write_stderr_line(source: &str) -> standard_io::Result<()> {
    return writeln!(standard_io::stderr().lock(), "{source}");
}

/// Writes one line to standard output.
///
/// # Errors
///
/// Returns an error when standard output cannot be written.
pub(in crate::cli) fn write_stdout_line(source: &str) -> Result<()> {
    result_or_return!(
        writeln!(standard_io::stdout().lock(), "{source}")
            .map_err(|error| return internal_error(error.to_string()))
    );
    return Ok(());
}
