mod layout;
mod predicates;
mod scripts;
mod source;

use super::check::{QualityCheck, quality_check};
use std::path::Path;

pub(crate) use layout::{
    frontend_build_command, frontend_check_command, frontend_package_manager, is_next_frontend,
    is_plain_static_frontend,
};
pub(crate) use predicates::{
    has_frontend_install_command, has_frontend_script_run, uses_javascript_linter,
};
use scripts::frontend_script_checks;
use source::frontend_source_checks;

pub(crate) fn static_frontend_checks(project_dir: &Path, run_scripts: bool) -> Vec<QualityCheck> {
    if is_plain_static_frontend(project_dir) {
        return Vec::new();
    }
    let mut checks = vec![frontend_lockfile_check(project_dir)];
    if let Some(check) = next_static_export_check(project_dir) {
        checks.push(check);
    }
    checks.extend(frontend_source_checks(project_dir));
    checks.extend(frontend_script_checks(project_dir, run_scripts));
    checks
}

fn frontend_lockfile_check(project_dir: &Path) -> QualityCheck {
    quality_check(
        "frontend lockfile",
        layout::frontend_lockfile_exists(project_dir),
        "found",
        "missing",
        "Commit package-lock.json, pnpm-lock.yaml, yarn.lock, bun.lock, or bun.lockb, then retry.",
    )
}

fn next_static_export_check(project_dir: &Path) -> Option<QualityCheck> {
    if !layout::is_next_frontend(project_dir) {
        return None;
    }
    let config_file = layout::next_config_file(project_dir);
    let ok = layout::next_static_export_enabled(project_dir);
    Some(QualityCheck {
        name: "Next static export".to_owned(),
        ok,
        message: next_static_export_message(config_file, ok),
        agent_instruction: if ok {
            None
        } else {
            Some(next_static_export_instruction(config_file))
        },
    })
}

fn next_static_export_message(config_file: Option<&str>, ok: bool) -> String {
    match (config_file, ok) {
        (Some(file), true) => format!("{file} sets output export"),
        (Some(file), false) => format!("{file} missing output export"),
        (None, _) => "next.config missing".to_owned(),
    }
}

fn next_static_export_instruction(config_file: Option<&str>) -> String {
    let target = config_file.unwrap_or("next.config.mjs");
    format!(
        "Create or update `{target}` with `output: \"export\"`, keep the Tovuk frontend output directory set to `out`, move Next API routes, middleware, SSR, and server logic to the Rust worker, then rerun `tovuk check --json`."
    )
}
