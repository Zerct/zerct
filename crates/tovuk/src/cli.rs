mod api_commands;
mod args;
mod auth;
mod config;
mod constants;
mod deploy;
mod doctor;
mod errors;
mod frontend_checks;
mod help;
mod preview;
mod project;
mod project_kind;
mod runtime;
mod template_sources;
mod templates;

/// Runs the native Tovuk CLI.
pub(crate) fn entrypoint() -> std::process::ExitCode {
    runtime::runtime_entrypoint()
}
