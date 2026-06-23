mod api_commands;
mod args;
mod auth;
mod constants;
mod errors;
mod help;
mod project;
mod runtime;

/// Runs the native Tovuk CLI.
pub(crate) fn entrypoint() -> std::process::ExitCode {
    runtime::runtime_entrypoint()
}
