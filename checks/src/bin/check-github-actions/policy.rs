//! Repository workflow input implementation.

use std::{
    fs::{DirEntry, read_dir as read_directory, read_to_string as read_file_to_string},
    io::Result as IoResult,
    path::Path,
};

use super::{CheckResult, HostedActionsCheck, Workflow, WorkflowRepository};

/// Repository-relative directory containing workflow definitions.
const WORKFLOW_DIR: &str = ".github/workflows";

impl WorkflowRepository for HostedActionsCheck {
    fn is_workflow_file(&self, path: &Path) -> bool {
        return matches!(
            path.extension()
                .and_then(|extension| return extension.to_str()),
            Some("yml" | "yaml")
        );
    }

    fn workflow_from_entry(
        &self,
        entry_result: IoResult<DirEntry>,
    ) -> CheckResult<Option<Workflow>> {
        let path =
            check_try!(entry_result.map_err(|error| format!("read {WORKFLOW_DIR}: {error}")))
                .path();
        if !self.is_workflow_file(path.as_path()) {
            return Ok(None);
        }
        let contents = check_try!(
            read_file_to_string(path.as_path())
                .map_err(|error| format!("read {}: {error}", path.display()))
        );
        return Ok(Some(Workflow { contents, path }));
    }

    fn workflows(&self) -> CheckResult<Vec<Workflow>> {
        let workflow_directory = Path::new(WORKFLOW_DIR);
        if !workflow_directory.is_dir() {
            return Err(format!("missing {WORKFLOW_DIR}"));
        }
        let entries = check_try!(
            read_directory(workflow_directory)
                .map_err(|error| format!("read {WORKFLOW_DIR}: {error}"))
        );
        let mut workflows = Vec::new();
        for entry_result in entries {
            let workflow = check_try!(self.workflow_from_entry(entry_result));
            workflows.extend(workflow);
        }
        workflows.sort_by(|left, right| return left.path.cmp(&right.path));
        return Ok(workflows);
    }
}
