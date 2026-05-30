use super::{
    super::{
        constants::DEFAULT_RUST_CHECK_COMMAND,
        frontend_checks::{frontend_build_command, frontend_check_command},
        project_kind::ProjectKind,
        project_layout::{default_build_command, default_check_command},
        resource_config::parse_resource_config,
        toml_values::{get_section, get_string, get_u16, reject_unknown_section_keys},
    },
    model::{BackendConfig, BuildConfig, FrontendConfig, RunConfig, TovukConfig},
};
use std::path::Path;

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
    let backend_table = get_section(&table, "worker")?;
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
    let kind = get_string(table, "kind")?.unwrap_or_else(|| "rust_worker".to_owned());
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
        "worker",
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
        check: get_string(table, "check")?
            .unwrap_or_else(|| default_check_command(kind, project_dir)),
        command: get_string(table, "command")?
            .unwrap_or_else(|| default_build_command(kind, project_dir)),
        output: if kind.is_static_frontend() {
            Some(get_string(table, "output")?.unwrap_or_else(|| "dist".to_owned()))
        } else {
            get_string(table, "output")?
        },
    })
}

fn parse_run_config(table: &toml::Table) -> std::result::Result<RunConfig, String> {
    Ok(RunConfig {
        command: get_string(table, "command")?,
        port: get_u16(table, "port")?.unwrap_or(3000),
        health: get_string(table, "health")?.unwrap_or_else(|| "/healthz".to_owned()),
    })
}

fn parse_frontend_config(
    table: &toml::Table,
    kind: ProjectKind,
    project_dir: &Path,
) -> std::result::Result<FrontendConfig, String> {
    if !kind.is_worker_static() {
        return Ok(FrontendConfig::default());
    }
    let root = get_string(table, "root")?;
    let frontend_dir = root
        .as_deref()
        .map_or_else(|| project_dir.to_path_buf(), |root| project_dir.join(root));
    Ok(FrontendConfig {
        root,
        check: Some(
            get_string(table, "check")?.unwrap_or_else(|| frontend_check_command(&frontend_dir)),
        ),
        build: Some(
            get_string(table, "build")?.unwrap_or_else(|| frontend_build_command(&frontend_dir)),
        ),
        output: Some(get_string(table, "output")?.unwrap_or_else(|| "dist".to_owned())),
    })
}

fn parse_backend_config(
    table: &toml::Table,
    kind: ProjectKind,
) -> std::result::Result<BackendConfig, String> {
    if !kind.is_worker_static() {
        return Ok(BackendConfig::default());
    }
    Ok(BackendConfig {
        root: get_string(table, "root")?,
        check: Some(
            get_string(table, "check")?.unwrap_or_else(|| DEFAULT_RUST_CHECK_COMMAND.to_owned()),
        ),
        build: Some(
            get_string(table, "build")?.unwrap_or_else(|| "cargo build --release".to_owned()),
        ),
        command: get_string(table, "command")?,
        port: Some(get_u16(table, "port")?.unwrap_or(3000)),
        health: Some(get_string(table, "health")?.unwrap_or_else(|| "/api/healthz".to_owned())),
    })
}

fn reject_unknown_root_keys(
    table: &toml::map::Map<String, toml::Value>,
) -> std::result::Result<(), String> {
    let allowed = [
        "name",
        "kind",
        "build",
        "run",
        "frontend",
        "worker",
        "resources",
    ];
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unsupported root key {key}"));
        }
    }
    Ok(())
}
