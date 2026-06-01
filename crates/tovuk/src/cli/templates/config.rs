use super::super::{
    config::CapabilitiesConfig,
    constants::{DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_RUST_CHECK_COMMAND},
    errors::{Result, agent_error},
    frontend_checks::{frontend_build_command, frontend_check_command, is_plain_static_frontend},
    project::{service_name_from_cargo, service_name_from_dir, service_name_from_package},
    project_kind::ProjectKind,
    project_layout::detect_fullstack_roots,
};
use std::path::Path;

pub(super) fn init_config(project_dir: &Path, kind: ProjectKind) -> Result<String> {
    match kind {
        ProjectKind::Fullstack => {
            if let Some((backend, frontend)) = detect_fullstack_roots(project_dir) {
                return Ok(fullstack_config(project_dir, &backend, &frontend, false));
            }
            Err(agent_error(
                "fullstack_roots_missing",
                "Could not find full-stack roots.",
                "Create api/Cargo.toml and web/package.json or web/index.html, then retry.",
                false,
            ))
        }
        ProjectKind::StaticFrontend => Ok(frontend_config(project_dir, false)),
        ProjectKind::RustWorker => Ok(rust_backend_config(project_dir)),
    }
}

pub(super) fn rust_backend_config(project_dir: &Path) -> String {
    let name = service_name_from_cargo(project_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    let capabilities = capabilities_toml(ProjectKind::RustWorker);
    format!(
        "name = \"{name}\"\nkind = \"rust_worker\"\n\n{capabilities}\n[build]\ncheck = \"{DEFAULT_RUST_CHECK_COMMAND}\"\ncommand = \"cargo build --release\"\n\n[run]\ncommand = \"./target/release/{name}\"\nport = 3000\nhealth = \"/healthz\"\n\n[resources]\nmemory = \"128mb\"\ncpu = \"1\"\nidle_timeout_minutes = 15\n"
    )
}

pub(super) fn frontend_config(project_dir: &Path, prefer_bun: bool) -> String {
    let name = service_name_from_package(project_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(project_dir));
    let settings = frontend_build_settings(project_dir, prefer_bun);
    let capabilities = capabilities_toml(ProjectKind::StaticFrontend);
    format!(
        "name = \"{name}\"\nkind = \"static_frontend\"\n\n{capabilities}\n[build]\ncheck = \"{}\"\ncommand = \"{}\"\noutput = \"{}\"\n",
        settings.check, settings.build, settings.output
    )
}

pub(super) fn fullstack_config(
    project_dir: &Path,
    backend: &str,
    frontend: &str,
    prefer_bun: bool,
) -> String {
    let name = service_name_from_dir(project_dir);
    let backend_dir = project_dir.join(backend);
    let frontend_dir = project_dir.join(frontend);
    let backend_name = service_name_from_cargo(&backend_dir)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| service_name_from_dir(&backend_dir));
    let settings = frontend_build_settings(&frontend_dir, prefer_bun);
    let capabilities = capabilities_toml(ProjectKind::Fullstack);
    format!(
        "name = \"{name}\"\nkind = \"fullstack\"\n\n{capabilities}\n[worker]\nroot = \"{backend}\"\ncheck = \"{DEFAULT_RUST_CHECK_COMMAND}\"\nbuild = \"cargo build --release\"\ncommand = \"./target/release/{backend_name}\"\nport = 3000\nhealth = \"/api/healthz\"\n\n[frontend]\nroot = \"{frontend}\"\ncheck = \"{}\"\nbuild = \"{}\"\noutput = \"{}\"\n\n[resources]\nmemory = \"128mb\"\ncpu = \"1\"\nidle_timeout_minutes = 15\n",
        settings.check, settings.build, settings.output
    )
}

fn capabilities_toml(kind: ProjectKind) -> String {
    let capabilities = CapabilitiesConfig::for_kind(kind);
    let body = CapabilitiesConfig::KEYS
        .iter()
        .map(|key| format!("{key} = {}", capabilities.value_for_key(key)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("[capabilities]\n{body}\n\n")
}

struct FrontendBuildSettings {
    check: String,
    build: String,
    output: String,
}

fn frontend_build_settings(project_dir: &Path, prefer_bun: bool) -> FrontendBuildSettings {
    let output = if is_plain_static_frontend(project_dir) {
        "."
    } else {
        "dist"
    };
    let check = if prefer_bun && output != "." {
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND.to_owned()
    } else {
        frontend_check_command(project_dir)
    };
    let build = if prefer_bun && output != "." {
        "bun run build".to_owned()
    } else {
        frontend_build_command(project_dir)
    };
    FrontendBuildSettings {
        check,
        build,
        output: output.to_owned(),
    }
}
