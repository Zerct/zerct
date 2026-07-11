//! Per-workflow security and release policy checks.

use super::{
    HostedActionsCheck, PathFilters as _, ReleasePolicy as _, Workflow, WorkflowPolicy,
    require_contains,
};

/// Unfiltered event header required by the canonical continuous-integration workflow.
const CI_TRIGGER_HEADER: &str =
    "on:\n  workflow_dispatch:\n  pull_request:\n  push:\n    branches:\n      - main\n";

impl WorkflowPolicy for HostedActionsCheck {
    fn check_checkout_credentials(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if workflow.contents.contains("actions/checkout@")
            && !workflow.contents.contains("persist-credentials: false")
        {
            findings.push(format!(
                "{}: checkout must set persist-credentials: false",
                workflow.path.display()
            ));
        }
    }

    fn check_ci_trigger_coverage(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if !workflow.path.ends_with("ci.yml") {
            return;
        }
        require_contains(
            workflow.contents.as_str(),
            CI_TRIGGER_HEADER,
            "ci.yml must run on workflow_dispatch, every pull request, and every main push",
            findings,
        );
        let filters = self.workflow_path_filters(workflow.contents.as_str());
        if !filters.is_empty() {
            findings.push(format!(
                "{}: CI must run for every pull request and main push; remove paths and paths-ignore filters",
                workflow.path.display()
            ));
        }
    }

    fn check_docs_deploy_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if !workflow.path.ends_with("docs-deploy.yml") {
            return;
        }

        require_contains(
            workflow.contents.as_str(),
            "if: github.ref == 'refs/heads/main'",
            "docs-deploy.yml must reject workflow_dispatch deploys from non-main refs before exposing Mintlify secrets",
            findings,
        );
    }

    fn check_github_hosted_cargo_cache(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if self.contains_cargo_publish_command(workflow.contents.as_str())
            && !self.uses_current_cache_action(workflow.contents.as_str())
        {
            findings.push(format!(
                "{}: GitHub-hosted Rust jobs must use the current actions/cache@v6",
                workflow.path.display()
            ));
        }
    }

    fn check_secret_workflow_dispatch_policy(
        &self,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    ) {
        if !workflow.contents.contains("workflow_dispatch:")
            || !workflow.contents.contains("secrets.")
        {
            return;
        }

        require_contains(
            workflow.contents.as_str(),
            "github.ref == 'refs/heads/main'",
            format!(
                "{}: manually dispatched workflows that read repository secrets must be restricted to main",
                workflow.path.display()
            )
            .as_str(),
            findings,
        );
    }

    fn check_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        require_contains(
            workflow.contents.as_str(),
            "\npermissions:",
            format!(
                "{}: missing explicit permissions block",
                workflow.path.display()
            )
            .as_str(),
            findings,
        );
        require_contains(
            workflow.contents.as_str(),
            "\nconcurrency:",
            format!(
                "{}: missing explicit concurrency block",
                workflow.path.display()
            )
            .as_str(),
            findings,
        );
        self.check_ci_trigger_coverage(workflow, findings);
        self.check_checkout_credentials(workflow, findings);
        self.check_github_hosted_cargo_cache(workflow, findings);
        self.check_public_package_release_order(workflow, findings);
        self.check_docs_deploy_workflow(workflow, findings);
        self.check_secret_workflow_dispatch_policy(workflow, findings);
    }

    fn contains_cargo_publish_command(&self, contents: &str) -> bool {
        return contents.lines().any(|line| {
            let trimmed = line.trim();
            return trimmed.contains("cargo build")
                || trimmed.contains("cargo check")
                || trimmed.contains("cargo test")
                || trimmed.contains("cargo clippy")
                || trimmed.contains("cargo package")
                || trimmed.contains("cargo publish");
        });
    }

    fn uses_current_cache_action(&self, contents: &str) -> bool {
        return contents.lines().any(|line| {
            let trimmed = line.trim();
            return trimmed.contains("uses: actions/cache@")
                && (trimmed.contains("actions/cache@v6") || trimmed.contains("# v6"));
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{HostedActionsCheck, Workflow, WorkflowPolicy as _};

    /// Verify that unfiltered continuous-integration triggers are accepted.
    ///
    /// # Panics
    ///
    /// Panics when the full trigger coverage contract emits a finding.
    #[test]
    fn ci_trigger_coverage_accepts_unfiltered_events() {
        let workflow = Workflow {
            contents:
                "on:\n  workflow_dispatch:\n  pull_request:\n  push:\n    branches:\n      - main\n"
                    .to_owned(),
            path: PathBuf::from(".github/workflows/ci.yml"),
        };
        let mut findings = Vec::new();

        HostedActionsCheck.check_ci_trigger_coverage(&workflow, &mut findings);

        assert!(
            findings.is_empty(),
            "unfiltered CI events must cover every public change"
        );
    }

    /// Verify that a continuous-integration path filter is rejected.
    ///
    /// # Panics
    ///
    /// Panics when a filtered CI trigger is not reported.
    #[test]
    fn ci_trigger_coverage_rejects_path_filters() {
        let workflow = Workflow {
            contents: "on:\n  pull_request:\n    paths:\n      - crates/**\n".to_owned(),
            path: PathBuf::from(".github/workflows/ci.yml"),
        };
        let mut findings = Vec::new();

        HostedActionsCheck.check_ci_trigger_coverage(&workflow, &mut findings);

        assert_eq!(
            findings.len(),
            0x2,
            "CI path filters must not be able to bypass repository checks"
        );
    }

    /// Verify that the current cache major is recognized from a pinned action.
    ///
    /// # Panics
    ///
    /// Panics when the cache v6 marker is not accepted.
    #[test]
    fn current_cache_action_accepts_v6_pin() {
        let contents =
            "      - uses: actions/cache@55cc8345863c7cc4c66a329aec7e433d2d1c52a9 # v6.1.0\n";

        assert!(
            HostedActionsCheck.uses_current_cache_action(contents),
            "the exact cache v6 pin must satisfy the current-major contract"
        );
    }

    /// Verify that the former cache major no longer satisfies policy.
    ///
    /// # Panics
    ///
    /// Panics when a cache v5 marker is still accepted.
    #[test]
    fn current_cache_action_rejects_v5_pin() {
        let contents =
            "      - uses: actions/cache@668228422ae6a00e4ad889ee87cd7109ec5666a7 # v5.0.4\n";

        assert!(
            !HostedActionsCheck.uses_current_cache_action(contents),
            "cache v5 must be treated as retired after the v6 release"
        );
    }
}
