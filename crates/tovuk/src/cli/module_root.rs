/// Extracts a successful value or returns the original error.
macro_rules! result_or_return {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error),
        }
    };
}

#[path = "api_commands/module_root.rs"]
/// Public API command handlers.
mod api_commands;
#[path = "args/module_root.rs"]
/// Strict command-line option model and parser.
mod args;
#[path = "auth/module_root.rs"]
/// Session discovery and interactive login.
mod auth;
/// Stable CLI constants.
mod constants;
/// Error payload and output policy.
mod errors;
/// Public command help text.
mod help;
/// Process entrypoint and command dispatch.
mod runtime;
#[path = "utils/module_root.rs"]
/// Shared URL, field, output, and browser utilities.
mod utils;

use std::process::ExitCode;

/// Native Tovuk CLI application entrypoint.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Application;

impl Run for Application {
    #[inline]
    fn run(self) -> ExitCode {
        return runtime::RunRuntime::run(runtime::Runtime);
    }
}

/// Executes one parsed command against the selected CLI options.
trait ExecuteCommand {
    /// Runs the command.
    ///
    /// # Errors
    ///
    /// Returns an error when validation, authentication, transport, or output fails.
    fn execute(self, cli: &args::CliOptions) -> errors::Result<()>;
}

/// Runs a CLI application to completion.
pub trait Run {
    /// Executes the application and returns its process exit status.
    fn run(self) -> ExitCode;
}

#[cfg(test)]
mod tests {
    use super::{Application, Run};
    use std::process::ExitCode;

    /// Function signature implemented by the public application runner.
    type ApplicationRunner = fn(Application) -> ExitCode;

    #[test]
    /// Verifies the public application runner keeps its stable callable shape.
    ///
    /// # Panics
    ///
    /// Panics when the application or runner signature changes unexpectedly.
    fn application_entrypoint_has_stable_signature() {
        let application = Application;
        let runner: ApplicationRunner = Run::run;

        assert_eq!(size_of_val(&application), size_of::<Application>());
        assert_eq!(size_of_val(&runner), size_of::<ApplicationRunner>());
    }
}
