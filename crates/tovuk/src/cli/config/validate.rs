use super::{
    super::{
        command_policy::{
            command_name_from_token, command_tokens, is_noop_command,
            uses_javascript_backend_runtime,
        },
        constants::RUST_STRICT_CLIPPY_DENY_LINTS,
        frontend_checks::{
            has_frontend_install_command, has_frontend_script_run, uses_javascript_linter,
        },
        project::{is_dns_safe_name, is_safe_relative_directory, is_safe_relative_path},
        project_kind::ProjectKind,
        resource_config::validate_resource_config,
    },
    model::TovukConfig,
};

pub(crate) fn validate_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let name = config.name.as_deref().unwrap_or_default();
    if !is_dns_safe_name(name) {
        return Err("name must be lowercase DNS-safe text up to 48 characters".to_owned());
    }
    validate_capabilities_config(config)?;
    validate_dev_config(config)?;
    if config.kind.is_fullstack() {
        validate_fullstack_config(config)?;
    } else {
        validate_build_config(config)?;
        if config.kind.is_static_frontend() {
            validate_output(config.build.output.as_deref(), "[build].output")?;
        } else {
            validate_rust_worker_config(config)?;
        }
    }
    Ok(())
}

fn validate_dev_config(config: &TovukConfig) -> std::result::Result<(), String> {
    if let Some(port) = config.dev.worker_port {
        validate_port(port, "[dev].worker_port")?;
    }
    if let Some(port) = config.dev.frontend_port {
        validate_port(port, "[dev].frontend_port")?;
    }
    Ok(())
}

fn validate_capabilities_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let expected_static_frontend = matches!(
        config.kind,
        ProjectKind::StaticFrontend | ProjectKind::Fullstack
    );
    let expected_worker = matches!(
        config.kind,
        ProjectKind::RustWorker | ProjectKind::Fullstack
    );
    if config.capabilities.static_frontend.is_enabled() != expected_static_frontend {
        return Err(format!(
            "[capabilities].static_frontend must be {expected_static_frontend} for kind = \"{}\"",
            config.kind.as_str()
        ));
    }
    if config.capabilities.worker.is_enabled() != expected_worker {
        return Err(format!(
            "[capabilities].worker must be {expected_worker} for kind = \"{}\"",
            config.kind.as_str()
        ));
    }
    for key in required_platform_capabilities() {
        if !config.capabilities.value_for_key(key) {
            return Err(format!(
                "[capabilities].{key} must be true because Tovuk exposes it for every service"
            ));
        }
    }
    Ok(())
}

fn required_platform_capabilities() -> [&'static str; 6] {
    [
        "logs",
        "builds",
        "usage_caps",
        "billing",
        "support",
        "abuse",
    ]
}

fn validate_build_config(config: &TovukConfig) -> std::result::Result<(), String> {
    if config.build.command.trim().is_empty() {
        return Err("[build].command is required".to_owned());
    }
    if config.build.check.trim().is_empty() {
        return Err("[build].check is required".to_owned());
    }
    validate_check_command(config.kind, &config.build.check)?;
    if config.kind.is_rust_worker() {
        validate_rust_build_command(&config.build.command)?;
    }
    Ok(())
}

fn validate_rust_worker_config(config: &TovukConfig) -> std::result::Result<(), String> {
    if config.build.output.is_some() {
        return Err("[build].output is only valid for static_frontend".to_owned());
    }
    validate_rust_run_command(config.run.command.as_deref())?;
    validate_port(config.run.port, "[run].port")?;
    validate_health(&config.run.health, "[run].health")?;
    validate_resource_config(&config.resources)
}

fn validate_fullstack_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let backend_root = validate_root(config.backend.root.as_deref(), "[worker].root")?;
    let frontend_root = validate_root(config.frontend.root.as_deref(), "[frontend].root")?;
    if backend_root == frontend_root {
        return Err("[worker].root and [frontend].root must be different directories".to_owned());
    }
    validate_rust_check_command(require_config_command(
        config.backend.check.as_deref(),
        "[worker].check",
    )?)?;
    validate_rust_build_command(require_config_command(
        config.backend.build.as_deref(),
        "[worker].build",
    )?)?;
    validate_rust_run_command(config.backend.command.as_deref())?;
    validate_port(config.backend.port.unwrap_or(0), "[worker].port")?;
    validate_health(
        config.backend.health.as_deref().unwrap_or_default(),
        "[worker].health",
    )?;
    validate_frontend_check_command(require_config_command(
        config.frontend.check.as_deref(),
        "[frontend].check",
    )?)?;
    require_config_command(config.frontend.build.as_deref(), "[frontend].build")?;
    validate_output(config.frontend.output.as_deref(), "[frontend].output")?;
    validate_resource_config(&config.resources)
}

fn require_config_command<'a>(
    value: Option<&'a str>,
    field: &str,
) -> std::result::Result<&'a str, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} is required"))
}

fn validate_root<'a>(value: Option<&'a str>, field: &str) -> std::result::Result<&'a str, String> {
    let value = value
        .ok_or_else(|| format!("{field} must be a safe relative directory such as api or web"))?;
    if is_safe_relative_path(value) {
        Ok(value)
    } else {
        Err(format!(
            "{field} must be a safe relative directory such as api or web"
        ))
    }
}

fn validate_port(value: u16, field: &str) -> std::result::Result<(), String> {
    if value == 0 {
        Err(format!("{field} must be between 1 and 65535"))
    } else {
        Ok(())
    }
}

fn validate_health(value: &str, field: &str) -> std::result::Result<(), String> {
    if value.starts_with('/') {
        Ok(())
    } else {
        Err(format!("{field} must be an absolute path"))
    }
}

fn validate_output(value: Option<&str>, field: &str) -> std::result::Result<(), String> {
    if value.is_some_and(is_safe_relative_directory) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a safe relative directory like dist or ."
        ))
    }
}

fn validate_check_command(kind: ProjectKind, command: &str) -> std::result::Result<(), String> {
    if kind.is_static_frontend() {
        validate_frontend_check_command(command)
    } else {
        validate_rust_check_command(command)
    }
}

fn validate_rust_check_command(command: &str) -> std::result::Result<(), String> {
    let required = [
        "cargo fmt --all --check",
        "cargo check --locked --release --all-targets --all-features",
        "cargo test --locked --release --all-targets --all-features",
        "cargo clippy --locked --release --all-targets --all-features",
        "-D warnings",
    ];
    let lint_ok = RUST_STRICT_CLIPPY_DENY_LINTS
        .iter()
        .all(|lint| command.contains(&format!("-D {lint}")));
    if required.iter().all(|fragment| command.contains(fragment)) && lint_ok {
        Ok(())
    } else {
        Err("[build].check must run rustfmt, locked release-mode cargo check, locked release-mode tests, and strict Clippy resource lints".to_owned())
    }
}

fn validate_rust_build_command(command: &str) -> std::result::Result<(), String> {
    if uses_javascript_backend_runtime(command) {
        return Err(
            "Rust worker build commands cannot invoke JavaScript or TypeScript runtimes; use cargo build --release"
                .to_owned(),
        );
    }
    let tokens = command_tokens(command);
    if tokens
        .iter()
        .any(|token| command_name_from_token(token) == "cargo")
        && tokens.iter().any(|token| token == "build")
        && tokens.iter().any(|token| token == "--release")
    {
        Ok(())
    } else {
        Err("Rust worker build commands must run cargo build --release".to_owned())
    }
}

fn validate_rust_run_command(command: Option<&str>) -> std::result::Result<(), String> {
    let value = command.unwrap_or_default();
    if uses_javascript_backend_runtime(value) {
        return Err(
            "Rust worker runtime commands cannot invoke JavaScript or TypeScript runtimes; run ./target/release/<binary> instead"
                .to_owned(),
        );
    }
    if command_tokens(value)
        .iter()
        .any(|token| token.contains("target/release/"))
    {
        Ok(())
    } else {
        Err("Rust worker runtime commands must start a binary under ./target/release/".to_owned())
    }
}

fn validate_frontend_check_command(command: &str) -> std::result::Result<(), String> {
    if is_noop_command(command) {
        return Ok(());
    }
    if uses_javascript_linter(command) {
        return Err("[build].check must not run JavaScript-based lint or format tooling; use oxlint or biome".to_owned());
    }
    let tokens = command_tokens(command);
    if has_frontend_install_command(&tokens)
        && has_frontend_script_run(&tokens, "typecheck")
        && has_frontend_script_run(&tokens, "lint")
    {
        Ok(())
    } else {
        Err("[build].check must install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --include=dev --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`".to_owned())
    }
}
