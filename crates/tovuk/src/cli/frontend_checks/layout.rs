use super::super::constants::{
    DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_NPM_FRONTEND_CHECK_COMMAND,
};
use std::path::Path;

pub(super) fn frontend_lockfile_exists(project_dir: &Path) -> bool {
    [
        "package-lock.json",
        "npm-shrinkwrap.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        "bun.lockb",
    ]
    .iter()
    .any(|file| project_dir.join(file).exists())
}

pub(crate) fn is_plain_static_frontend(project_dir: &Path) -> bool {
    !project_dir.join("package.json").exists() && project_dir.join("index.html").exists()
}

pub(crate) fn frontend_package_manager(project_dir: &Path) -> &'static str {
    if project_dir.join("bun.lock").exists() || project_dir.join("bun.lockb").exists() {
        "bun"
    } else {
        "npm"
    }
}

pub(crate) fn frontend_check_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND.to_owned()
    } else {
        DEFAULT_NPM_FRONTEND_CHECK_COMMAND.to_owned()
    }
}

pub(crate) fn frontend_build_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        "bun run build".to_owned()
    } else {
        "npm run build".to_owned()
    }
}
