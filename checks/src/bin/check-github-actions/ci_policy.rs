//! Continuous-integration history-boundary policy.

use super::{CiPolicy, HostedActionsCheck, Workflow};

/// Complete reviewed pull-request audit and push defense-in-depth workflow.
const TRUSTED_HISTORY_WORKFLOW: &str = r#"name: Ref history audit

# The repository-scoped organization ruleset pins this workflow and checker source.
# Ordinary pull-request and push runs remain read-only defense in depth.

on:
  pull_request:
    branches:
      - main
  push:

permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: false

jobs:
  history:
    name: Git object history audit
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    steps:
      - name: Check out pinned workflow source
        uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 # v7.0.0
        with:
          fetch-depth: 0
          persist-credentials: false
          ref: ${{ github.workflow_sha }}
      - name: Build event history checker
        env:
          CARGO_TARGET_DIR: ${{ runner.temp }}/tovuk-trusted-history
        run: cargo build --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts
      - name: Check newly reachable Git objects
        env:
          TOVUK_CI_EVENT_NAME: ${{ github.event_name }}
          TOVUK_CI_EVENT_REF: ${{ github.ref }}
          TOVUK_CI_EVENT_REF_TYPE: ${{ github.ref_type }}
          TOVUK_CI_EVENT_SHA: ${{ github.sha }}
          TOVUK_CI_PULL_BASE_SHA: ${{ github.event.pull_request.base.sha }}
          TOVUK_CI_PULL_HEAD_SHA: ${{ github.event.pull_request.head.sha }}
          TOVUK_CI_PULL_MERGE_SHA: ${{ github.event.pull_request.merge_commit_sha }}
          TOVUK_CI_PULL_NUMBER: ${{ github.event.pull_request.number }}
          TOVUK_CI_PUSH_AFTER_SHA: ${{ github.event.after }}
          TOVUK_CI_PUSH_BEFORE_SHA: ${{ github.event.before }}
          TOVUK_CI_PUSH_CREATED: ${{ github.event.created }}
          TOVUK_CI_PUSH_DELETED: ${{ github.event.deleted }}
          TOVUK_CI_PUSH_FORCED: ${{ github.event.forced }}
          TOVUK_CI_WORKFLOW_SHA: ${{ github.workflow_sha }}
        run: '"$RUNNER_TEMP/tovuk-trusted-history/debug/check-public-contracts" ci-snapshot'
"#;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&check_full_ci_separation),
    size_of_val(&check_trusted_history),
];

impl CiPolicy for HostedActionsCheck {
    fn require_ci_history_gate(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if workflow.path.ends_with("ci.yml") {
            check_full_ci_separation(workflow, findings);
            return;
        }
        if workflow.path.ends_with("trusted-history.yml") {
            check_trusted_history(workflow, findings);
        }
    }
}

/// Keep contributor-controlled full checks separate from privileged event data.
fn check_full_ci_separation(workflow: &Workflow, findings: &mut Vec<String>) {
    if workflow.contents.contains("ci-snapshot") {
        findings.push(format!(
            "{}: contributor-controlled full CI must remain separate from the pinned history audit",
            workflow.path.display()
        ));
    }
}

/// Require the privileged history workflow to match the complete reviewed source.
fn check_trusted_history(workflow: &Workflow, findings: &mut Vec<String>) {
    if workflow.contents != TRUSTED_HISTORY_WORKFLOW {
        findings.push(format!(
            "{}: ref history audit must exactly match the reviewed workflow",
            workflow.path.display()
        ));
    }
}
