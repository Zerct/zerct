use std::{collections::BTreeSet, process::Command};

use super::{
    github_actions_policy::Workflow,
    path_filters::{path_filter_matches_tracked, workflow_path_filters},
};

pub(super) fn tracked_files() -> Result<BTreeSet<String>, String> {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .map_err(|error| format!("git ls-files failed: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed".to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

pub(super) fn check_workflow_path_filters(
    workflow: &Workflow,
    tracked_files: &BTreeSet<String>,
    findings: &mut Vec<String>,
) {
    for path_filter in workflow_path_filters(workflow.contents.as_str()) {
        if !path_filter_matches_tracked(path_filter.as_str(), tracked_files) {
            findings.push(format!(
                "{}: workflow path filter {path_filter:?} does not match tracked files",
                workflow.path.display()
            ));
        }
    }
}
