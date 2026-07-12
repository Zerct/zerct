//! Global `GitHub` Actions policy regressions.

use std::path::PathBuf;

use super::{
    GlobalPolicy as _, HostedActionsCheck, TRUSTED_MAIN_RUNNER_ASSIGNMENT, Workflow,
    require_trusted_runner_routing,
};

/// Verify alternate YAML spellings cannot bypass dangerous-trigger rejection.
///
/// # Panics
///
/// Panics when list or quoted trigger syntax is not rejected.
#[test]
fn dangerous_triggers_are_rejected_in_alternate_yaml_syntax() {
    let workflows = [
        Workflow {
            contents: "on: [push, pull_request_target]\n".to_owned(),
            path: PathBuf::from(".github/workflows/list.yml"),
        },
        Workflow {
            contents: "on:\n  \"pull_request_target\":\n".to_owned(),
            path: PathBuf::from(".github/workflows/quoted.yml"),
        },
        Workflow {
            contents: "on: [push, workflow_run]\n".to_owned(),
            path: PathBuf::from(".github/workflows/chained.yml"),
        },
    ];
    let mut findings = Vec::new();

    HostedActionsCheck.reject_global_matches(&workflows, &mut findings);

    assert_eq!(
        findings.len(),
        0x0003,
        "every alternate dangerous-trigger spelling must be rejected"
    );
}

/// Verify only the branch-guarded CI assignment can select the trusted runner.
///
/// # Panics
///
/// Panics when guarded routing fails or direct pull-request routing is accepted.
#[test]
fn trusted_ci_runner_requires_main_guard() {
    let guarded = Workflow {
        contents: format!("jobs:\n  full-check:\n    {TRUSTED_MAIN_RUNNER_ASSIGNMENT}\n"),
        path: PathBuf::from(".github/workflows/ci.yml"),
    };
    let unguarded = Workflow {
        contents: "jobs:\n  full-check:\n    runs-on: tovuk-public-linux-x64\n".to_owned(),
        path: PathBuf::from(".github/workflows/ci.yml"),
    };
    let mut guarded_findings = Vec::new();
    let mut unguarded_findings = Vec::new();

    require_trusted_runner_routing(&guarded, &mut guarded_findings);
    require_trusted_runner_routing(&unguarded, &mut unguarded_findings);

    assert!(
        guarded_findings.is_empty(),
        "guarded main routing must pass"
    );
    assert_eq!(
        unguarded_findings.len(),
        0x0001,
        "direct trusted CI routing must fail the closed assignment check"
    );
}
