mod api_commands;
mod args;
mod auth;
mod command_policy;
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
mod project_layout;
mod resource_config;
mod runtime;
mod source_policy;
mod template_sources;
mod templates;
mod toml_values;

/// Runs the native Tovuk CLI.
pub(crate) fn entrypoint() -> std::process::ExitCode {
    runtime::runtime_entrypoint()
}
