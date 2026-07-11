//! Registry publication workflow policy regression tests.

use std::path::PathBuf;

use super::{HostedActionsCheck, Workflow, WorkflowPolicy as _};

/// Minimal source satisfying the dedicated trusted-publisher identity contract.
const TRUSTED_PUBLISHER_SOURCE: &str = r#"workflow_dispatch:
[ "$GITHUB_REF" != "refs/tags/$release_ref" ]
[ "$GITHUB_SHA" != "$INPUT_RELEASE_COMMIT" ]
release_commit:
orchestration_id:
ref: ${{ inputs.release_commit }}
release_commit="$(git rev-parse "refs/tags/$release_ref^{commit}")"
release_target="$(gh api --jq '.target_commitish'
--bin check-native-release-assets -- "$version"
[[ ! "$INPUT_RELEASE_COMMIT" =~ ^[0-9a-f]{40}$ ]]
"#;

/// Verify that bounded transient registry retries satisfy policy.
///
/// # Panics
///
/// Panics when the exact retry and transient-status markers are rejected.
#[test]
fn registry_preflight_retry_accepts_bounded_policy() {
    let workflow = Workflow {
        contents: "for delay in 0 5 10 20 30 60; do\n429|5??)\n".to_owned(),
        path: PathBuf::from(".github/workflows/publish-crates.yml"),
    };
    let mut findings = Vec::new();

    HostedActionsCheck.check_registry_preflight_retry(&workflow, &mut findings);

    assert!(findings.is_empty(), "bounded retries must satisfy policy");
}

/// Verify that a single-shot registry probe is rejected.
///
/// # Panics
///
/// Panics when either required retry marker is not reported.
#[test]
fn registry_preflight_retry_rejects_single_shot_probe() {
    let workflow = Workflow {
        contents: "curl https://registry.example.invalid\n".to_owned(),
        path: PathBuf::from(".github/workflows/publish-crates.yml"),
    };
    let mut findings = Vec::new();

    HostedActionsCheck.check_registry_preflight_retry(&workflow, &mut findings);

    assert_eq!(findings.len(), 0x2, "both retry markers must be required");
}

/// Verify that one top-level trusted publisher satisfies identity policy.
///
/// # Panics
///
/// Panics when the exact source and dispatch contract emits a finding.
#[test]
fn registry_publisher_identity_accepts_top_level_dispatch() {
    let workflow = Workflow {
        contents: TRUSTED_PUBLISHER_SOURCE.to_owned(),
        path: PathBuf::from(".github/workflows/publish-npm.yml"),
    };
    let mut findings = Vec::new();

    HostedActionsCheck.check_registry_publisher_identity(&workflow, &mut findings);

    assert!(findings.is_empty(), "top-level dispatch must be accepted");
}

/// Verify that reusable and workflow-run publisher identities are rejected.
///
/// # Panics
///
/// Panics when either unsupported trusted-publisher trigger is accepted.
#[test]
fn registry_publisher_identity_rejects_indirect_triggers() {
    let workflow = Workflow {
        contents: format!("{TRUSTED_PUBLISHER_SOURCE}workflow_call:\nworkflow_run:\n"),
        path: PathBuf::from(".github/workflows/publish-pypi.yml"),
    };
    let mut findings = Vec::new();

    HostedActionsCheck.check_registry_publisher_identity(&workflow, &mut findings);

    assert_eq!(
        findings.len(),
        0x2,
        "both unsupported publisher identities must be rejected"
    );
}
