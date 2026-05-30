use super::{
    constants::{DEFAULT_RUST_CHECK_COMMAND, WORKSPACE_EXCLUDED_DIRS},
    frontend_checks::{frontend_build_command, frontend_check_command, is_plain_static_frontend},
    project_kind::ProjectKind,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

const WORKER_STATIC_WORKER_ROOTS: &[&str] = &["api", "worker", "server"];
const WORKER_STATIC_FRONTEND_ROOTS: &[&str] = &["web", "frontend", "app", "site"];
const RUST_REQUIRED_FILES: &[&str] = &["Cargo.toml", "Cargo.lock"];
const FRONTEND_PACKAGE_REQUIRED_FILES: &[&str] = &["package.json"];
const PLAIN_STATIC_REQUIRED_FILES: &[&str] = &["index.html"];

pub(crate) fn infer_project_kind(project_dir: &Path) -> ProjectKind {
    if detect_fullstack_roots(project_dir).is_some() {
        ProjectKind::WorkerStatic
    } else if project_dir.join("Cargo.toml").exists() {
        ProjectKind::RustWorker
    } else if project_dir.join("package.json").exists() || project_dir.join("index.html").exists() {
        ProjectKind::StaticFrontend
    } else {
        ProjectKind::RustWorker
    }
}

pub(crate) fn detect_fullstack_roots(project_dir: &Path) -> Option<(String, String)> {
    let backend = WORKER_STATIC_WORKER_ROOTS
        .iter()
        .find(|root| project_dir.join(root).join("Cargo.toml").exists())?;
    let frontend = WORKER_STATIC_FRONTEND_ROOTS.iter().find(|root| {
        project_dir.join(root).join("package.json").exists()
            || project_dir.join(root).join("index.html").exists()
    })?;
    Some(((*backend).to_owned(), (*frontend).to_owned()))
}

pub(crate) fn required_files(project_dir: &Path, kind: ProjectKind) -> &'static [&'static str] {
    match kind {
        ProjectKind::WorkerStatic | ProjectKind::RustWorker => RUST_REQUIRED_FILES,
        ProjectKind::StaticFrontend if is_plain_static_frontend(project_dir) => {
            PLAIN_STATIC_REQUIRED_FILES
        }
        ProjectKind::StaticFrontend => FRONTEND_PACKAGE_REQUIRED_FILES,
    }
}

pub(crate) fn fullstack_backend_required_files() -> &'static [&'static str] {
    RUST_REQUIRED_FILES
}

pub(crate) fn fullstack_frontend_required_files(project_dir: &Path) -> &'static [&'static str] {
    if is_plain_static_frontend(project_dir) {
        PLAIN_STATIC_REQUIRED_FILES
    } else {
        FRONTEND_PACKAGE_REQUIRED_FILES
    }
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

pub(crate) fn kind_order(kind: Option<ProjectKind>) -> u8 {
    kind.map_or(3, ProjectKind::sort_order)
}

pub(crate) fn discover_project_dirs(dir: &Path, project_dirs: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir()
            || entry
                .file_name()
                .to_str()
                .is_some_and(|name| WORKSPACE_EXCLUDED_DIRS.contains(&name))
        {
            continue;
        }
        if path.join("tovuk.toml").exists() {
            project_dirs.push(path);
        } else {
            discover_project_dirs(&path, project_dirs);
        }
    }
}
