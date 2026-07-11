//! Per-workflow security and release policy checks.

use super::{
    HostedActionsCheck, PathFilters as _, PolicyRequirement, ReleasePolicy as _, Workflow,
    WorkflowPolicy, require_contains,
};

use std::ffi::OsStr;

#[cfg(test)]
#[path = "workflow_policy_publication_tests.rs"]
mod publication_tests;

/// Unfiltered event header required by the canonical continuous-integration workflow.
const CI_TRIGGER_HEADER: &str =
    "on:\n  workflow_dispatch:\n  pull_request:\n  push:\n    branches:\n      - main\n";

/// Required guarded recovery and dispatch markers.
const PUBLICATION_RECOVERY_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "if: github.ref == 'refs/heads/main'",
        "publication recovery must reject non-main dispatches",
    ),
    (
        "ref: refs/tags/${{ inputs.release_ref }}",
        "publication recovery must check out a fully qualified release tag",
    ),
    (
        r#"release_commit="$(git rev-parse "refs/tags/$RELEASE_REF^{commit}")""#,
        "publication recovery must peel the release tag to one immutable commit",
    ),
    (
        "release_target=\"$(gh api --jq '.target_commitish'",
        "publication recovery must compare the tag commit with the GitHub release target",
    ),
    (
        "--bin check-native-release-assets -- \"$version\"",
        "publication recovery must verify complete native assets before dispatch",
    ),
    (
        "needs: gate",
        "the actions-write dispatch job must depend on the credential-free release gate",
    ),
    (
        "actions: write # Dispatch and monitor the dedicated trusted-publisher workflows.",
        "publication recovery must scope workflow dispatch permission to its dispatch job",
    ),
    (
        "gh workflow run \"$workflow\"",
        "publication recovery must directly dispatch trusted-publisher workflows",
    ),
    (
        "--ref \"$RELEASE_REF\"",
        "publication recovery must execute each publisher from the immutable release tag",
    ),
    (
        "--commit \"$RELEASE_COMMIT\"",
        "publication recovery must discover publisher runs by the immutable release commit",
    ),
    (
        "dispatch_publisher() {",
        "publication recovery must retry transient workflow dispatch failures",
    ),
    (
        "workflows=(publish-crates.yml publish-npm.yml publish-pypi.yml)",
        "publication recovery must dispatch every public registry publisher",
    ),
    (
        "if ! gh run watch \"$run_id\"",
        "publication recovery must observe every publisher without fail-fast abandonment",
    ),
    (
        "publisher_failed=false",
        "publication recovery must aggregate publisher conclusions",
    ),
];

/// Required top-level trusted-publisher identity markers.
const REGISTRY_PUBLISHER_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "workflow_dispatch:",
        "registry publisher must support guarded manual recovery",
    ),
    (
        "[ \"$GITHUB_REF\" != \"refs/tags/$release_ref\" ]",
        "registry publisher must fail when the dispatch ref is not the release tag",
    ),
    (
        "[ \"$GITHUB_SHA\" != \"$INPUT_RELEASE_COMMIT\" ]",
        "registry publisher must fail when the dispatch commit is not the release commit",
    ),
    (
        "release_commit:",
        "registry publisher must require an immutable release commit input",
    ),
    (
        "orchestration_id:",
        "registry publisher must require a unique orchestration identifier",
    ),
    (
        "ref: ${{ inputs.release_commit }}",
        "publication must check out the exact resolved release commit",
    ),
    (
        r#"release_commit="$(git rev-parse "refs/tags/$release_ref^{commit}")""#,
        "publication must peel the release tag to one immutable commit",
    ),
    (
        "release_target=\"$(gh api --jq '.target_commitish'",
        "publication must compare the tag commit with the GitHub release target",
    ),
    (
        "--bin check-native-release-assets -- \"$version\"",
        "publication must verify the complete native release before registry access",
    ),
    (
        "[[ ! \"$INPUT_RELEASE_COMMIT\" =~ ^[0-9a-f]{40}$ ]]",
        "publication must validate the immutable commit input",
    ),
];

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

    fn check_publication_recovery_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if !workflow.path.ends_with("recover-publication.yml") {
            return;
        }
        if workflow.contents.contains("id-token: write") {
            findings.push(format!(
                "{}: recovery orchestration must never receive registry OIDC permission",
                workflow.path.display()
            ));
        }
        require_workflow_requirements(workflow, PUBLICATION_RECOVERY_REQUIREMENTS, findings);
    }

    fn check_registry_preflight_retry(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if !is_registry_publisher(workflow) {
            return;
        }
        for (needle, message) in [
            (
                "for delay in 0 5 10 20 30 60; do",
                "registry preflight must retry on a bounded schedule",
            ),
            (
                "429|5??)",
                "registry preflight must classify rate limits and server errors as transient",
            ),
        ] {
            require_contains(
                workflow.contents.as_str(),
                needle,
                format!("{}: {message}", workflow.path.display()).as_str(),
                findings,
            );
        }
    }

    fn check_registry_publisher_identity(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if !is_registry_publisher(workflow) {
            return;
        }
        let forbidden_triggers = ["workflow_call:", "workflow_run:"];
        for forbidden_trigger in forbidden_triggers
            .into_iter()
            .filter(|trigger| return workflow.contents.contains(trigger))
        {
            findings.push(format!(
                "{}: trusted registry publishers must not use {forbidden_trigger}",
                workflow.path.display(),
            ));
        }
        require_workflow_requirements(workflow, REGISTRY_PUBLISHER_REQUIREMENTS, findings);
    }

    fn check_registry_workflow_concurrency(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        let required_group = match workflow.path.file_name().and_then(OsStr::to_str) {
            Some("publish-crates.yml") => "  group: crates-publication",
            Some("publish-npm.yml") => "  group: npm-publication",
            Some("publish-pypi.yml") => "  group: pypi-publication",
            None | Some(_) => return,
        };
        require_contains(
            workflow.contents.as_str(),
            required_group,
            format!(
                "{}: registry release concurrency must be registry-scoped",
                workflow.path.display()
            )
            .as_str(),
            findings,
        );
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
        self.check_publication_recovery_workflow(workflow, findings);
        self.check_registry_preflight_retry(workflow, findings);
        self.check_registry_publisher_identity(workflow, findings);
        self.check_registry_workflow_concurrency(workflow, findings);
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

/// Return whether one workflow owns a public registry trusted-publisher identity.
fn is_registry_publisher(workflow: &Workflow) -> bool {
    return matches!(
        workflow.path.file_name().and_then(OsStr::to_str),
        Some("publish-crates.yml" | "publish-npm.yml" | "publish-pypi.yml")
    );
}

/// Require every named marker in one workflow.
fn require_workflow_requirements(
    workflow: &Workflow,
    requirements: &[PolicyRequirement],
    findings: &mut Vec<String>,
) {
    for &(needle, message) in requirements {
        require_contains(
            workflow.contents.as_str(),
            needle,
            format!("{}: {message}", workflow.path.display()).as_str(),
            findings,
        );
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

    /// Verify that one publisher accepts its registry-scoped group.
    ///
    /// # Panics
    ///
    /// Panics when the non-colliding group emits a finding.
    #[test]
    fn registry_workflow_concurrency_accepts_scoped_group() {
        let workflow = Workflow {
            contents: "concurrency:\n  group: crates-publication\n".to_owned(),
            path: PathBuf::from(".github/workflows/publish-crates.yml"),
        };
        let mut findings = Vec::new();

        HostedActionsCheck.check_registry_workflow_concurrency(&workflow, &mut findings);

        assert!(
            findings.is_empty(),
            "registry-scoped concurrency must be accepted"
        );
    }

    /// Verify that one publisher rejects a non-registry group.
    ///
    /// # Panics
    ///
    /// Panics when the colliding group is not reported.
    #[test]
    fn registry_workflow_concurrency_rejects_unscoped_group() {
        let workflow = Workflow {
            contents: "concurrency:\n  group: ${{ github.workflow }}-${{ github.ref }}\n"
                .to_owned(),
            path: PathBuf::from(".github/workflows/publish-crates.yml"),
        };
        let mut findings = Vec::new();

        HostedActionsCheck.check_registry_workflow_concurrency(&workflow, &mut findings);

        assert_eq!(
            findings.len(),
            0x1,
            "non-registry concurrency must be rejected for publishers"
        );
    }
}
