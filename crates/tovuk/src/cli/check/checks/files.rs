use super::super::{
    super::{project_kind::ProjectKind, project_layout::required_files},
    report::{QualityCheck, quality_check},
};
use std::path::Path;

pub(in crate::cli::check) fn required_file_checks(
    project_dir: &Path,
    kind: ProjectKind,
) -> Vec<QualityCheck> {
    required_files(project_dir, kind)
        .iter()
        .map(|file| {
            let ok = project_dir.join(file).exists();
            quality_check(
                file,
                ok,
                "found",
                "missing",
                &format!("Create and commit {file}, then retry."),
            )
        })
        .collect()
}

pub(super) fn required_files_at(
    project_dir: &Path,
    label: &str,
    files: &[&str],
) -> Vec<QualityCheck> {
    files
        .iter()
        .map(|file| {
            let display = if label.is_empty() {
                (*file).to_owned()
            } else {
                format!("{label}/{file}")
            };
            quality_check(
                &display,
                project_dir.join(file).exists(),
                "found",
                "missing",
                &format!("Create and commit {display}, then retry."),
            )
        })
        .collect()
}
