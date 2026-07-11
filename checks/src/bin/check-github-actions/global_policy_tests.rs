//! Global `GitHub` Actions policy regressions.

use std::path::PathBuf;

use super::{GlobalPolicy as _, HostedActionsCheck, Workflow};

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
