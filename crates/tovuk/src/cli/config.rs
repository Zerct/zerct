use super::{
    constants::{
        DEFAULT_RUST_CHECK_COMMAND, JAVASCRIPT_BACKEND_RUNTIMES, RUST_STRICT_CLIPPY_DENY_LINTS,
    },
    frontend_checks::{
        command_name_from_token, command_tokens, frontend_build_command, frontend_check_command,
        has_frontend_install_command, has_frontend_script_run, uses_javascript_linter,
    },
    project::{is_dns_safe_name, is_safe_relative_directory, is_safe_relative_path},
};
use serde::{Serialize, Serializer};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProjectKind {
    Fullstack,
    RustBackend,
    StaticFrontend,
}

impl ProjectKind {
    pub(crate) fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "fullstack" => Ok(Self::Fullstack),
            "rust_backend" => Ok(Self::RustBackend),
            "static_frontend" => Ok(Self::StaticFrontend),
            _ => Err("kind must be fullstack, rust_backend, or static_frontend".to_owned()),
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Fullstack => "fullstack",
            Self::RustBackend => "rust_backend",
            Self::StaticFrontend => "static_frontend",
        }
    }

    pub(crate) const fn sort_order(self) -> u8 {
        match self {
            Self::RustBackend => 0,
            Self::Fullstack => 1,
            Self::StaticFrontend => 2,
        }
    }

    pub(crate) const fn supports_database(self) -> bool {
        matches!(self, Self::Fullstack | Self::RustBackend)
    }

    pub(crate) const fn is_fullstack(self) -> bool {
        matches!(self, Self::Fullstack)
    }

    pub(crate) const fn is_static_frontend(self) -> bool {
        matches!(self, Self::StaticFrontend)
    }

    pub(crate) const fn is_rust_backend(self) -> bool {
        matches!(self, Self::RustBackend)
    }
}

impl Serialize for ProjectKind {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TovukConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    pub(crate) kind: ProjectKind,
    pub(crate) build: BuildConfig,
    pub(crate) run: RunConfig,
    pub(crate) frontend: FrontendConfig,
    pub(crate) backend: BackendConfig,
    pub(crate) resources: ResourceConfig,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct BuildConfig {
    pub(crate) command: String,
    pub(crate) check: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RunConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<String>,
    pub(crate) port: u16,
    pub(crate) health: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct FrontendConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct BackendConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) health: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ResourceConfig {
    pub(crate) memory: String,
    pub(crate) cpu: String,
    pub(crate) idle_timeout_minutes: u16,
}

pub(crate) fn parse_tovuk_toml(
    source: &str,
    project_dir: &Path,
) -> std::result::Result<TovukConfig, String> {
    let table = source
        .parse::<toml::Table>()
        .map_err(|error| error.to_string())?;
    reject_unknown_root_keys(&table)?;
    let kind = parse_project_kind(&table)?;
    let build_table = get_section(&table, "build")?;
    let run_table = get_section(&table, "run")?;
    let frontend_table = get_section(&table, "frontend")?;
    let backend_table = get_section(&table, "backend")?;
    let resources_table = get_section(&table, "resources")?;
    reject_unknown_config_sections(
        &build_table,
        &run_table,
        &frontend_table,
        &backend_table,
        &resources_table,
    )?;

    Ok(TovukConfig {
        name: get_string(&table, "name")?,
        build: parse_build_config(&build_table, kind, project_dir)?,
        run: parse_run_config(&run_table)?,
        frontend: parse_frontend_config(&frontend_table, kind, project_dir)?,
        backend: parse_backend_config(&backend_table, kind)?,
        resources: parse_resource_config(&resources_table)?,
        kind,
    })
}

fn parse_project_kind(table: &toml::Table) -> std::result::Result<ProjectKind, String> {
    let kind = get_string(table, "kind")?.unwrap_or_else(|| "rust_backend".to_owned());
    ProjectKind::parse(&kind)
}

fn reject_unknown_config_sections(
    build: &toml::Table,
    run: &toml::Table,
    frontend: &toml::Table,
    backend: &toml::Table,
    resources: &toml::Table,
) -> std::result::Result<(), String> {
    reject_unknown_section_keys(build, "build", &["command", "check", "output"])?;
    reject_unknown_section_keys(run, "run", &["command", "port", "health"])?;
    reject_unknown_section_keys(frontend, "frontend", &["root", "check", "build", "output"])?;
    reject_unknown_section_keys(
        backend,
        "backend",
        &["root", "check", "build", "command", "port", "health"],
    )?;
    reject_unknown_section_keys(
        resources,
        "resources",
        &["memory", "cpu", "idle_timeout_minutes"],
    )
}

fn parse_build_config(
    table: &toml::Table,
    kind: ProjectKind,
    project_dir: &Path,
) -> std::result::Result<BuildConfig, String> {
    Ok(BuildConfig {
        check: get_section_string(table, "check")?
            .unwrap_or_else(|| default_check_command(kind, project_dir)),
        command: get_section_string(table, "command")?
            .unwrap_or_else(|| default_build_command(kind, project_dir)),
        output: if kind.is_static_frontend() {
            Some(get_section_string(table, "output")?.unwrap_or_else(|| "dist".to_owned()))
        } else {
            get_section_string(table, "output")?
        },
    })
}

fn parse_run_config(table: &toml::Table) -> std::result::Result<RunConfig, String> {
    Ok(RunConfig {
        command: get_section_string(table, "command")?,
        port: get_section_u16(table, "port")?.unwrap_or(3000),
        health: get_section_string(table, "health")?.unwrap_or_else(|| "/healthz".to_owned()),
    })
}

fn parse_frontend_config(
    table: &toml::Table,
    kind: ProjectKind,
    project_dir: &Path,
) -> std::result::Result<FrontendConfig, String> {
    if !kind.is_fullstack() {
        return Ok(FrontendConfig::default());
    }
    let root = get_section_string(table, "root")?;
    let frontend_dir = root
        .as_deref()
        .map_or_else(|| project_dir.to_path_buf(), |root| project_dir.join(root));
    Ok(FrontendConfig {
        root,
        check: Some(
            get_section_string(table, "check")?
                .unwrap_or_else(|| frontend_check_command(&frontend_dir)),
        ),
        build: Some(
            get_section_string(table, "build")?
                .unwrap_or_else(|| frontend_build_command(&frontend_dir)),
        ),
        output: Some(get_section_string(table, "output")?.unwrap_or_else(|| "dist".to_owned())),
    })
}

fn parse_backend_config(
    table: &toml::Table,
    kind: ProjectKind,
) -> std::result::Result<BackendConfig, String> {
    if !kind.is_fullstack() {
        return Ok(BackendConfig::default());
    }
    Ok(BackendConfig {
        root: get_section_string(table, "root")?,
        check: Some(
            get_section_string(table, "check")?
                .unwrap_or_else(|| DEFAULT_RUST_CHECK_COMMAND.to_owned()),
        ),
        build: Some(
            get_section_string(table, "build")?
                .unwrap_or_else(|| "cargo build --release".to_owned()),
        ),
        command: get_section_string(table, "command")?,
        port: Some(get_section_u16(table, "port")?.unwrap_or(3000)),
        health: Some(
            get_section_string(table, "health")?.unwrap_or_else(|| "/api/healthz".to_owned()),
        ),
    })
}

fn parse_resource_config(table: &toml::Table) -> std::result::Result<ResourceConfig, String> {
    Ok(ResourceConfig {
        memory: get_section_string(table, "memory")?.unwrap_or_else(|| "512mb".to_owned()),
        cpu: get_section_string(table, "cpu")?.unwrap_or_else(|| "0.25".to_owned()),
        idle_timeout_minutes: get_section_u16(table, "idle_timeout_minutes")?.unwrap_or(15),
    })
}

pub(crate) fn reject_unknown_root_keys(
    table: &toml::map::Map<String, toml::Value>,
) -> std::result::Result<(), String> {
    let allowed = [
        "name",
        "kind",
        "build",
        "run",
        "frontend",
        "backend",
        "resources",
    ];
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported root key {key}"));
        }
    }
    Ok(())
}

pub(crate) fn reject_unknown_section_keys(
    table: &toml::map::Map<String, toml::Value>,
    section: &str,
    allowed: &[&str],
) -> std::result::Result<(), String> {
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported [{section}] key {key}"));
        }
    }
    Ok(())
}

pub(crate) fn get_section(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<toml::map::Map<String, toml::Value>, String> {
    match table.get(key) {
        None => Ok(toml::map::Map::new()),
        Some(value) => value
            .as_table()
            .cloned()
            .ok_or_else(|| format!("[{key}] must be a table")),
    }
}

pub(crate) fn get_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<String>, String> {
    table.get(key).map_or(Ok(None), |value| {
        value
            .as_str()
            .map(|value| Some(value.to_owned()))
            .ok_or_else(|| format!("{key} must be a string"))
    })
}

pub(crate) fn get_section_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<String>, String> {
    get_string(table, key)
}

pub(crate) fn get_section_u16(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
) -> std::result::Result<Option<u16>, String> {
    table.get(key).map_or(Ok(None), |value| {
        let integer = value
            .as_integer()
            .ok_or_else(|| format!("{key} must be a number"))?;
        u16::try_from(integer)
            .ok()
            .map(Some)
            .ok_or_else(|| format!("{key} must be between 0 and 65535"))
    })
}

pub(crate) fn default_check_command(kind: ProjectKind, project_dir: &Path) -> String {
    if kind.is_static_frontend() {
        frontend_check_command(project_dir)
    } else {
        DEFAULT_RUST_CHECK_COMMAND.to_owned()
    }
}

pub(crate) fn default_build_command(kind: ProjectKind, project_dir: &Path) -> String {
    if kind.is_static_frontend() {
        frontend_build_command(project_dir)
    } else {
        "cargo build --release".to_owned()
    }
}

pub(crate) fn validate_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let name = config.name.as_deref().unwrap_or_default();
    if !is_dns_safe_name(name) {
        return Err("name must be lowercase DNS-safe text up to 48 characters".to_owned());
    }
    if config.kind.is_fullstack() {
        validate_fullstack_config(config)?;
    } else {
        validate_build_config(config)?;
        if config.kind.is_static_frontend() {
            validate_output(config.build.output.as_deref(), "[build].output")?;
        } else {
            validate_rust_backend_config(config)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_build_config(config: &TovukConfig) -> std::result::Result<(), String> {
    if config.build.command.trim().is_empty() {
        return Err("[build].command is required".to_owned());
    }
    if config.build.check.trim().is_empty() {
        return Err("[build].check is required".to_owned());
    }
    validate_check_command(config.kind, &config.build.check)?;
    if config.kind.is_rust_backend() {
        validate_rust_build_command(&config.build.command)?;
    }
    Ok(())
}

pub(crate) fn validate_rust_backend_config(
    config: &TovukConfig,
) -> std::result::Result<(), String> {
    if config.build.output.is_some() {
        return Err("[build].output is only valid for static_frontend".to_owned());
    }
    validate_rust_run_command(config.run.command.as_deref())?;
    validate_port(config.run.port, "[run].port")?;
    validate_health(&config.run.health, "[run].health")?;
    validate_resource_config(config)
}

pub(crate) fn validate_fullstack_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let backend_root = validate_root(config.backend.root.as_deref(), "[backend].root")?;
    let frontend_root = validate_root(config.frontend.root.as_deref(), "[frontend].root")?;
    if backend_root == frontend_root {
        return Err("[backend].root and [frontend].root must be different directories".to_owned());
    }
    validate_rust_check_command(require_config_command(
        config.backend.check.as_deref(),
        "[backend].check",
    )?)?;
    validate_rust_build_command(require_config_command(
        config.backend.build.as_deref(),
        "[backend].build",
    )?)?;
    validate_rust_run_command(config.backend.command.as_deref())?;
    validate_port(config.backend.port.unwrap_or(0), "[backend].port")?;
    validate_health(
        config.backend.health.as_deref().unwrap_or_default(),
        "[backend].health",
    )?;
    validate_frontend_check_command(require_config_command(
        config.frontend.check.as_deref(),
        "[frontend].check",
    )?)?;
    require_config_command(config.frontend.build.as_deref(), "[frontend].build")?;
    validate_output(config.frontend.output.as_deref(), "[frontend].output")?;
    validate_resource_config(config)
}

pub(crate) fn require_config_command<'a>(
    value: Option<&'a str>,
    field: &str,
) -> std::result::Result<&'a str, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{field} is required"))
}

pub(crate) fn validate_root<'a>(
    value: Option<&'a str>,
    field: &str,
) -> std::result::Result<&'a str, String> {
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

pub(crate) fn validate_port(value: u16, field: &str) -> std::result::Result<(), String> {
    if value == 0 {
        Err(format!("{field} must be between 1 and 65535"))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_health(value: &str, field: &str) -> std::result::Result<(), String> {
    if value.starts_with('/') {
        Ok(())
    } else {
        Err(format!("{field} must be an absolute path"))
    }
}

pub(crate) fn validate_output(value: Option<&str>, field: &str) -> std::result::Result<(), String> {
    if value.is_some_and(is_safe_relative_directory) {
        Ok(())
    } else {
        Err(format!(
            "{field} must be a safe relative directory like dist or ."
        ))
    }
}

pub(crate) fn validate_resource_config(config: &TovukConfig) -> std::result::Result<(), String> {
    let memory_mib = memory_to_mib(&config.resources.memory)?;
    if !(128..=2048).contains(&memory_mib) {
        return Err(
            "[resources].memory must be between 128mb and 2gb; use the smallest working value"
                .to_owned(),
        );
    }
    let cpu_millis = cpu_to_millis(&config.resources.cpu)?;
    if !(50..=2000).contains(&cpu_millis) {
        return Err(
            "[resources].cpu must be between 0.05 and 2; use the smallest working value".to_owned(),
        );
    }
    if !(1..=60).contains(&config.resources.idle_timeout_minutes) {
        return Err("[resources].idle_timeout_minutes must be between 1 and 60".to_owned());
    }
    Ok(())
}

pub(crate) fn validate_check_command(
    kind: ProjectKind,
    command: &str,
) -> std::result::Result<(), String> {
    if kind.is_static_frontend() {
        validate_frontend_check_command(command)
    } else {
        validate_rust_check_command(command)
    }
}

pub(crate) fn validate_rust_check_command(command: &str) -> std::result::Result<(), String> {
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

pub(crate) fn validate_rust_build_command(command: &str) -> std::result::Result<(), String> {
    if uses_javascript_backend_runtime(command) {
        return Err(
            "Rust backend build commands cannot invoke JavaScript or TypeScript runtimes; use cargo build --release"
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
        Err("Rust backend build commands must run cargo build --release".to_owned())
    }
}

pub(crate) fn validate_rust_run_command(command: Option<&str>) -> std::result::Result<(), String> {
    let value = command.unwrap_or_default();
    if uses_javascript_backend_runtime(value) {
        return Err(
            "Rust backend runtime commands cannot invoke JavaScript or TypeScript runtimes; run ./target/release/<binary> instead"
                .to_owned(),
        );
    }
    if command_tokens(value)
        .iter()
        .any(|token| token.contains("target/release/"))
    {
        Ok(())
    } else {
        Err("Rust backend runtime commands must start a binary under ./target/release/".to_owned())
    }
}

pub(crate) fn validate_frontend_check_command(command: &str) -> std::result::Result<(), String> {
    if is_noop_command(command) {
        return Ok(());
    }
    if uses_javascript_linter(command) {
        return Err("[build].check must not run JavaScript-based lint or format tooling; use oxlint, biome, or deno lint".to_owned());
    }
    let tokens = command_tokens(command);
    if has_frontend_install_command(&tokens)
        && has_frontend_script_run(&tokens, "typecheck")
        && has_frontend_script_run(&tokens, "lint")
    {
        Ok(())
    } else {
        Err("[build].check must install dependencies and run package scripts, for example `bun ci && bun run typecheck && bun run lint` or `npm ci --prefer-offline --no-audit --fund=false && npm run typecheck && npm run lint`".to_owned())
    }
}

pub(crate) fn uses_javascript_backend_runtime(command: &str) -> bool {
    command_tokens(command)
        .iter()
        .any(|token| JAVASCRIPT_BACKEND_RUNTIMES.contains(&command_name_from_token(token).as_str()))
}

pub(crate) fn is_noop_command(command: &str) -> bool {
    let command = command.trim();
    command == ":" || command == "true"
}

pub(crate) fn memory_to_mib(value: &str) -> std::result::Result<u32, String> {
    let clean = value.trim().to_ascii_lowercase();
    let amount = clean
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    let unit = clean[amount.len()..].trim();
    let amount = amount
        .parse::<u32>()
        .map_err(|_error| "[resources].memory must look like 256mb, 512mb, or 1gb".to_owned())?;
    match unit {
        "mb" | "mib" => Ok(amount),
        "gb" | "gib" => Ok(amount * 1024),
        _ => Err("[resources].memory must look like 256mb, 512mb, or 1gb".to_owned()),
    }
}

pub(crate) fn cpu_to_millis(value: &str) -> std::result::Result<u32, String> {
    let clean = value.trim();
    if clean.is_empty()
        || clean
            .chars()
            .any(|character| !character.is_ascii_digit() && character != '.')
        || clean.matches('.').count() > 1
    {
        return Err("[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned());
    }
    let mut parts = clean.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<u32>()
        .map_err(|_error| "[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned())?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 3 {
        return Err("[resources].cpu must look like 0.25, 0.5, 1, or 2".to_owned());
    }
    let mut fractional_millis = 0u32;
    for (index, digit) in fraction.bytes().enumerate() {
        fractional_millis += u32::from(digit - b'0') * [100, 10, 1][index];
    }
    Ok(whole * 1000 + fractional_millis)
}
