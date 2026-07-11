//! `GitHub` Actions policy checks for the public Tovuk repository.

/// Propagate an absent policy value without the question-mark operator.
macro_rules! check_some {
    ($option:expr) => {
        match $option {
            Some(value) => value,
            None => return None,
        }
    };
}

/// Propagate a failed policy check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

extern crate alloc;

#[path = "check-github-actions/policy.rs"]
mod github_actions_policy;
#[path = "check-github-actions/global_policy.rs"]
mod global_policy;
#[path = "check-github-actions/path_filter_contract.rs"]
mod path_filter_contract;
#[path = "check-github-actions/path_filters.rs"]
mod path_filters;
#[path = "check-github-actions/release_policy.rs"]
mod release_policy;
#[path = "check-github-actions/workflow_policy.rs"]
mod workflow_policy;

use alloc::collections::BTreeSet;
use flate2 as _;
use http as _;

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use tokio as _;

use serde as _;
use serde_json as _;
use sha2 as _;
use std::{
    fs::DirEntry,
    io::{Result as IoResult, Write as _, stderr},
    path::{Path, PathBuf},
    process::ExitCode,
};
use tar as _;
use tovuk_public_checks as _;
use url as _;

/// Actions maintained by Blacksmith that are forbidden in public workflows.
const BLACKSMITH_ACTIONS: &[&str] = &[
    "useblacksmith/cache",
    "useblacksmith/rust-cache",
    "useblacksmith/setup-go",
    "useblacksmith/setup-java",
    "useblacksmith/setup-node",
    "useblacksmith/setup-python",
    "useblacksmith/setup-ruby",
];

/// Executable repository policy check.
trait Check {
    /// Execute the policy check.
    ///
    /// # Errors
    ///
    /// Returns an error when repository inputs cannot be read, a diagnostic
    /// cannot be written, or at least one policy requirement is violated.
    fn execute(&self) -> CheckResult;
}

/// Result returned by `GitHub` Actions policy operations.
type CheckResult<Value = ()> = Result<Value, String>;

/// Crates.io release policy operations.
trait CrateReleasePolicy {
    /// Require the safe event and authentication foundation for crate releases.
    fn check_crate_release_base(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Apply all Crates.io release workflow requirements.
    fn check_crate_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Reject release triggers that bypass the reusable publication workflow.
    fn reject_crate_release_triggers(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require Crates.io Trusted Publishing credentials and runner isolation.
    fn require_crates_trusted_publishing(&self, workflow: &Workflow, findings: &mut Vec<String>);
}

/// Blocking behavior of a documentation readiness step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DocsReadinessBehavior {
    /// A readiness failure blocks the release.
    Blocking,
    /// A readiness failure is explicitly ignored.
    ContinuesOnError,
}

/// Location and blocking behavior of a documentation readiness step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DocsReadinessStep {
    /// Whether a readiness failure blocks the release.
    behavior: DocsReadinessBehavior,
    /// One-based line where the workflow step begins.
    start_line: usize,
}

/// Mutable state while locating a documentation readiness workflow step.
#[derive(Debug)]
struct DocsReadinessTracker {
    /// Blocking behavior accumulated for the current step.
    behavior: DocsReadinessBehavior,
    /// Presence marker for the required readiness command.
    command_seen: Option<()>,
    /// One-based line where the current step begins.
    start_line: Option<usize>,
}

/// Repository-wide policy operations.
trait GlobalPolicy {
    /// Return whether a workflow line references a Blacksmith action.
    fn line_uses_blacksmith(&self, line: &str) -> bool;

    /// Return whether a workflow line invokes a `JavaScript` lint tool.
    /// Reject forbidden action and tool references across all workflows.
    fn reject_global_matches(&self, workflows: &[Workflow], findings: &mut Vec<String>);

    /// Reject JavaScript-based lint and type-check commands.
    /// Reject obsolete or floating cache action versions.
    fn reject_retired_cache_action(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Reject an obsolete cache action version on one indexed workflow line.
    fn reject_retired_cache_line(
        &self,
        workflow: &Workflow,
        indexed_line: (usize, &str),
        findings: &mut Vec<String>,
    );

    /// Reject Blacksmith action forks.
    fn reject_useblacksmith(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require workflows and the local aggregate checker to use the same gates.
    ///
    /// # Errors
    ///
    /// Returns an error when the local aggregate checker cannot be read.
    fn require_check_all_hooks(
        &self,
        workflows: &[Workflow],
        findings: &mut Vec<String>,
    ) -> CheckResult;

    /// Run the native workflow linter and record failures.
    fn run_actionlint(&self, findings: &mut Vec<String>);
}

/// `GitHub` Actions repository policy checker.
#[derive(Clone, Copy, Debug)]
struct HostedActionsCheck;

impl Check for HostedActionsCheck {
    fn execute(&self) -> CheckResult {
        let workflows = check_try!(self.workflows());
        let tracked_files = check_try!(self.tracked_files());
        let mut findings = Vec::new();

        self.reject_global_matches(workflows.as_slice(), &mut findings);
        check_try!(self.require_check_all_hooks(workflows.as_slice(), &mut findings));
        for workflow in &workflows {
            self.check_workflow(workflow, &mut findings);
            self.check_workflow_path_filters(workflow, &tracked_files, &mut findings);
        }
        self.run_actionlint(&mut findings);

        if findings.is_empty() {
            return Ok(());
        }
        let report_result = findings.into_iter().try_for_each(|finding| {
            return write_stderr(finding.as_str());
        });
        if let Err(error) = report_result {
            return Err(error);
        }
        return Err("GitHub Actions policy check failed".to_owned());
    }
}

/// Native binary release policy operations.
trait NativeReleasePolicy {
    /// Require live documentation readiness to block native publication.
    fn check_blocking_docs_readiness_gate(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require the exact binary-affecting push trigger paths for native releases.
    fn check_native_release_path_filter_contract(
        &self,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    );

    /// Apply all native binary release workflow requirements.
    fn check_native_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Find the workflow step that executes the documentation readiness check.
    fn docs_readiness_step(&self, contents: &str) -> Option<DocsReadinessStep>;

    /// Process one indexed workflow line while locating the readiness step.
    fn process_docs_readiness_line(
        &self,
        indexed_line: (usize, &str),
        tracker: &mut DocsReadinessTracker,
    ) -> Option<DocsReadinessStep>;
}

/// State accumulated while parsing a workflow path-filter block.
#[derive(Debug)]
struct PathFilterBlock {
    /// Indentation of the active paths or paths-ignore key.
    block_indent: Option<usize>,
    /// Filters collected from the active and completed blocks.
    filters: Vec<String>,
}

/// Path-filter contract operations.
trait PathFilterContract {
    /// Check that every workflow path filter matches a tracked file.
    fn check_workflow_path_filters(
        &self,
        workflow: &Workflow,
        tracked_files: &BTreeSet<String>,
        findings: &mut Vec<String>,
    );

    /// Return repository paths tracked by Git.
    ///
    /// # Errors
    ///
    /// Returns an error when Git cannot enumerate tracked files.
    fn tracked_files(&self) -> CheckResult<BTreeSet<String>>;
}

/// Workflow path-filter parsing and matching operations.
trait PathFilters {
    /// Advance a glob cursor across one wildcard-delimited pattern part.
    fn advance_glob_cursor(&self, cursor: usize, part: &str, path: &str) -> Option<usize>;

    /// Return whether a simplified workflow glob matches a tracked path.
    fn glob_matches(&self, pattern: &str, path: &str) -> bool;

    /// Count the leading ASCII spaces on a workflow source line.
    fn leading_spaces(&self, line: &str) -> usize;

    /// Return whether a path filter matches at least one tracked file.
    fn path_filter_matches_tracked(&self, filter: &str, tracked_files: &BTreeSet<String>) -> bool;

    /// Process one workflow source line for path-filter extraction.
    fn process_path_filter_line(&self, line: &str, state: &mut PathFilterBlock);

    /// Remove matching single or double quotes from a YAML scalar.
    fn unquote_yaml_string<'value>(&self, value: &'value str) -> &'value str;

    /// Extract path and path-ignore filters from workflow source.
    fn workflow_path_filters(&self, contents: &str) -> Vec<String>;
}

/// Source snippet paired with the policy diagnostic emitted when it is absent or present.
type PolicyRequirement = (&'static str, &'static str);

/// A release policy operation in its required execution order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReleaseCheck {
    /// Validate the Crates.io publication workflow.
    CratePublishing,
    /// Validate native binary publication.
    NativePublishing,
    /// Validate wrapper consumption of native release assets.
    NativeReleaseAssets,
    /// Validate synchronized public package versions.
    PackageVersions,
    /// Validate language wrapper publication.
    WrapperPublishing,
}

/// Public release workflow policy operations.
trait ReleasePolicy {
    /// Apply the ordered release policy selected for a workflow.
    fn check_public_package_release_order(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Execute one release policy operation.
    fn execute_release_check(
        &self,
        check: ReleaseCheck,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    );

    /// Require synchronized package-version verification before publication.
    fn require_package_versions(&self, workflow: &Workflow, findings: &mut Vec<String>);
}

/// A workflow source loaded from the repository.
#[derive(Debug)]
struct Workflow {
    /// Complete workflow source text.
    contents: String,
    /// Repository-relative workflow path.
    path: PathBuf,
}

/// Per-workflow policy operations.
trait WorkflowPolicy {
    /// Require checkout steps to discard persisted credentials.
    fn check_checkout_credentials(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require continuous integration to run for every pull request and main push.
    fn check_ci_trigger_coverage(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require the documentation deploy workflow to protect secret-bearing runs.
    fn check_docs_deploy_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require GitHub-hosted Cargo jobs to use the approved cache action.
    fn check_github_hosted_cargo_cache(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require manually dispatched secret-bearing workflows to target main.
    fn check_secret_workflow_dispatch_policy(
        &self,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    );

    /// Apply all policies relevant to one workflow.
    fn check_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Return whether workflow source contains a Cargo build or publish command.
    fn contains_cargo_publish_command(&self, contents: &str) -> bool;

    /// Return whether workflow source uses the current cache action major.
    fn uses_current_cache_action(&self, contents: &str) -> bool;
}

/// Repository workflow input operations.
trait WorkflowRepository {
    /// Return whether a path identifies a supported workflow file.
    fn is_workflow_file(&self, path: &Path) -> bool;

    /// Load one workflow directory entry when it is a supported workflow file.
    ///
    /// # Errors
    ///
    /// Returns an error when the entry or its workflow source cannot be read.
    fn workflow_from_entry(
        &self,
        entry_result: IoResult<DirEntry>,
    ) -> CheckResult<Option<Workflow>>;

    /// Load all repository workflows in deterministic path order.
    ///
    /// # Errors
    ///
    /// Returns an error when the workflow directory is absent or unreadable, or
    /// when a workflow source cannot be read.
    fn workflows(&self) -> CheckResult<Vec<Workflow>>;
}

/// Language wrapper release policy operations.
trait WrapperReleasePolicy {
    /// Require wrapper publication to verify native release assets.
    fn check_wrapper_release_assets(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Require the safe event and authentication foundation for wrapper releases.
    fn check_wrapper_release_base(&self, workflow: &Workflow, findings: &mut Vec<String>);

    /// Apply all language wrapper release workflow requirements.
    fn check_wrapper_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>);
}

fn main() -> ExitCode {
    match HostedActionsCheck.execute() {
        Ok(()) => return ExitCode::SUCCESS,
        Err(message) => {
            return match write_stderr(message.as_str()) {
                Ok(()) | Err(_) => ExitCode::FAILURE,
            };
        }
    }
}

/// Reject workflow lines containing a prohibited fragment.
fn reject_lines(workflow: &Workflow, needle: &str, message: &str, findings: &mut Vec<String>) {
    reject_matching_lines(workflow, message, findings, |line| {
        return line.contains(needle);
    });
}

/// Reject workflow lines accepted by a caller-provided predicate.
fn reject_matching_lines(
    workflow: &Workflow,
    message: &str,
    findings: &mut Vec<String>,
    line_matches: impl Fn(&str) -> bool,
) {
    for (line_index, line) in workflow.contents.lines().enumerate() {
        if line_matches(line) {
            findings.push(format!(
                "{}:{}: {message}",
                workflow.path.display(),
                line_index.saturating_add(0x1)
            ));
        }
    }
}

/// Require a source fragment and record a finding when it is absent.
fn require_contains(haystack: &str, needle: &str, message: &str, findings: &mut Vec<String>) {
    if !haystack.contains(needle) {
        findings.push(message.to_owned());
    }
}

/// Write one diagnostic line to standard error.
///
/// # Errors
///
/// Returns an error when the process standard error stream cannot be written.
fn write_stderr(message: &str) -> CheckResult {
    let mut writer = stderr().lock();
    return match writeln!(writer, "{message}") {
        Ok(()) => Ok(()),
        Err(error) => Err(format!("write stderr: {error}")),
    };
}
