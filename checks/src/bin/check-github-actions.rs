//! GitHub Actions policy checks for the public Tovuk repository.

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

use std::process::ExitCode;

use github_actions_policy::workflows;
use global_policy::{
    reject_global_matches, require_check_all_hooks, require_ci_path_filter_contract,
    require_public_trusted_ci, run_actionlint,
};
use path_filter_contract::{check_workflow_path_filters, tracked_files};
use workflow_policy::check_workflow;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let workflows = workflows()?;
    let tracked_files = tracked_files()?;
    let mut findings = Vec::new();

    reject_global_matches(&workflows, &mut findings);
    require_check_all_hooks(&mut findings)?;
    for workflow in &workflows {
        check_workflow(workflow, &mut findings);
        check_workflow_path_filters(workflow, &tracked_files, &mut findings);
    }
    require_ci_path_filter_contract(&workflows, &mut findings);
    require_public_trusted_ci(&workflows, &mut findings);
    run_actionlint(&mut findings);

    if findings.is_empty() {
        return Ok(());
    }
    for finding in findings {
        eprintln!("{finding}");
    }
    Err("GitHub Actions policy check failed".to_owned())
}
