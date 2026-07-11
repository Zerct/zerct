use super::super::{
    args::CliOptions,
    errors::{Result, write_stdout_line},
};

#[derive(Clone, Copy, Debug)]
/// Human-readable progress text permitted by the selected output format.
struct ProgressMessage<'message>(Option<&'message str>);

impl<'message> From<(&CliOptions, &'message str)> for ProgressMessage<'message> {
    fn from(value: (&CliOptions, &'message str)) -> Self {
        let (cli, message) = value;
        if cli.is_json() {
            return Self(None);
        }
        return Self(Some(message));
    }
}

/// Prints human-readable progress unless JSON output is active.
///
/// # Errors
///
/// Returns an error when standard output cannot be written.
pub(in crate::cli) fn progress(cli: &CliOptions, message: &str) -> Result<()> {
    if ProgressMessage::from((cli, message)).0.is_some() {
        return write_stdout_line(message);
    }
    return Ok(());
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
