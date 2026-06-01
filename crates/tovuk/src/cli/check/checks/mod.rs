use super::{
    super::{
        config::TovukConfig, frontend_checks::static_frontend_checks, project_kind::ProjectKind,
    },
    report::QualityCheck,
};
use std::path::Path;

mod files;
mod rust;
mod source;

pub(super) use files::required_file_checks;
pub(crate) use rust::first_output_line;

use files::required_files_at;
use rust::rust_backend_checks;
use source::backend_javascript_source_check;

pub(super) fn fullstack_checks(
    project_dir: &Path,
    config: &TovukConfig,
    config_valid: bool,
) -> Vec<QualityCheck> {
    let backend_root = config.backend.root.clone().unwrap_or_default();
    let frontend_root = config.frontend.root.clone().unwrap_or_default();
    let backend_dir = project_dir.join(&backend_root);
    let frontend_dir = project_dir.join(&frontend_root);
    let mut checks = Vec::new();
    checks.extend(required_files_at(
        &backend_dir,
        &backend_root,
        super::super::project_layout::fullstack_backend_required_files(),
    ));
    checks.push(backend_javascript_source_check(&backend_dir, &backend_root));
    checks.extend(rust_backend_checks(&backend_dir, config_valid));
    checks.extend(required_files_at(
        &frontend_dir,
        &frontend_root,
        super::super::project_layout::fullstack_frontend_required_files(&frontend_dir),
    ));
    checks.extend(static_frontend_checks(&frontend_dir, config_valid));
    checks
}

pub(super) fn rust_quality_checks(
    project_dir: &Path,
    kind: ProjectKind,
    config_valid: bool,
) -> Vec<QualityCheck> {
    if kind.is_static_frontend() {
        let mut checks = static_frontend_checks(project_dir, config_valid);
        checks.push(rust::unsafe_check(project_dir));
        return checks;
    }

    let mut checks = vec![backend_javascript_source_check(project_dir, "")];
    checks.extend(rust_backend_checks(project_dir, config_valid));
    checks
}
