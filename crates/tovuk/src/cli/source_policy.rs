use super::{
    constants::{FRONTEND_JAVASCRIPT_EXTENSIONS, FRONTEND_SOURCE_ROOTS},
    project::walk_project_files,
};
use std::path::Path;

#[derive(Default)]
pub(crate) struct FrontendSourceReport {
    pub(crate) typescript: Vec<String>,
    pub(crate) javascript: Vec<String>,
    pub(crate) server_routes: Vec<String>,
}

pub(crate) fn frontend_source_report(project_dir: &Path) -> FrontendSourceReport {
    let mut report = FrontendSourceReport::default();
    walk_project_files(project_dir, |_file, relative| {
        if is_frontend_server_route(relative) {
            report.server_routes.push(relative.to_owned());
        }
        if !is_frontend_source_path(relative) {
            return;
        }
        if is_frontend_typescript_source(relative) {
            report.typescript.push(relative.to_owned());
        } else if is_frontend_javascript_source(relative) {
            report.javascript.push(relative.to_owned());
        }
    });
    report
}

pub(crate) fn backend_javascript_or_typescript_sources(
    project_dir: &Path,
    label: &str,
) -> Vec<String> {
    let mut matches = Vec::new();
    walk_project_files(project_dir, |_file, relative| {
        if is_backend_javascript_or_typescript_source(relative) {
            matches.push(if label.is_empty() {
                relative.to_owned()
            } else {
                format!("{label}/{relative}")
            });
        }
    });
    matches
}

fn is_frontend_source_path(relative: &str) -> bool {
    relative
        .split('/')
        .next()
        .is_some_and(|root| FRONTEND_SOURCE_ROOTS.contains(&root))
}

fn is_frontend_typescript_source(relative: &str) -> bool {
    !relative.ends_with(".d.ts")
        && Path::new(relative)
            .extension()
            .is_some_and(|extension| matches_ignore_ascii_case(extension, &["ts", "tsx"]))
}

fn matches_ignore_ascii_case(value: &std::ffi::OsStr, candidates: &[&str]) -> bool {
    candidates
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn is_frontend_javascript_source(relative: &str) -> bool {
    FRONTEND_JAVASCRIPT_EXTENSIONS
        .iter()
        .any(|extension| relative.ends_with(extension))
}

fn is_frontend_server_route(relative: &str) -> bool {
    if !is_frontend_typescript_source(relative) && !is_frontend_javascript_source(relative) {
        return false;
    }
    let parts = relative
        .to_ascii_lowercase()
        .split('/')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let file = parts.last().map_or("", String::as_str);
    file.starts_with("+server.")
        || file.starts_with("middleware.")
        || path_starts_with(&parts, &["pages", "api"])
        || path_starts_with(&parts, &["src", "pages", "api"])
        || (file.starts_with("route.")
            && (path_starts_with(&parts, &["app", "api"])
                || path_starts_with(&parts, &["src", "app", "api"])))
}

fn path_starts_with(path_parts: &[String], prefix: &[&str]) -> bool {
    path_parts.len() >= prefix.len()
        && prefix
            .iter()
            .enumerate()
            .all(|(index, part)| path_parts[index] == *part)
}

fn is_backend_javascript_or_typescript_source(relative: &str) -> bool {
    if relative.ends_with(".d.ts") || !is_javascript_or_typescript_path(relative) {
        return false;
    }
    relative
        .split('/')
        .any(|component| ["api", "app", "pages", "routes", "server", "src"].contains(&component))
}

fn is_javascript_or_typescript_path(relative: &str) -> bool {
    [".cjs", ".js", ".jsx", ".mjs", ".ts", ".tsx"]
        .iter()
        .any(|extension| relative.ends_with(extension))
}
