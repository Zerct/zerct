//! Workflow path-filter contract checks against tracked repository files.

use alloc::collections::BTreeSet;
use std::process::Command;

use super::{CheckResult, HostedActionsCheck, PathFilterContract, PathFilters as _, Workflow};

impl PathFilterContract for HostedActionsCheck {
    fn check_workflow_path_filters(
        &self,
        workflow: &Workflow,
        tracked_files: &BTreeSet<String>,
        findings: &mut Vec<String>,
    ) {
        self.workflow_path_filters(workflow.contents.as_str())
            .into_iter()
            .filter(|path_filter| {
                return !self.path_filter_matches_tracked(path_filter.as_str(), tracked_files);
            })
            .for_each(|path_filter| {
                findings.push(format!(
                    "{}: workflow path filter {path_filter:?} does not match tracked files",
                    workflow.path.display()
                ));
            });
    }

    fn tracked_files(&self) -> CheckResult<BTreeSet<String>> {
        let output = check_try!(
            Command::new("git")
                .args(["ls-files"])
                .output()
                .map_err(|error| format!("git ls-files failed: {error}"))
        );
        if !output.status.success() {
            return Err("git ls-files failed".to_owned());
        }
        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(ToOwned::to_owned)
            .collect());
    }
}
