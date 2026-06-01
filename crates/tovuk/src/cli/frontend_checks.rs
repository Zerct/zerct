mod layout;
mod predicates;
mod scripts;
mod source;

use super::check::{QualityCheck, quality_check};
use std::path::Path;

pub(crate) use layout::{frontend_build_command, frontend_check_command, is_plain_static_frontend};
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
