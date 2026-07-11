//! Language wrapper release workflow policy.

use crate::{
    HostedActionsCheck, PolicyRequirement, Workflow, WrapperReleasePolicy, reject_lines,
    require_contains,
};

/// Wrapper workflow snippets rejected by the orchestrated release contract.
const REJECTED_WRAPPER_SNIPPETS: &[PolicyRequirement] = &[
    (
        "method=skip",
        "package publish workflows must fail closed instead of silently skipping publish auth",
    ),
    (
        "workflow_call:",
        "trusted package publishers must execute as top-level workflows",
    ),
    (
        "workflow_run:",
        "trusted package publishers must be dispatched directly instead of using workflow_run",
    ),
    (
        "push:",
        "package publication must not run directly from an unverified source push",
    ),
    (
        "github.actor ==",
        "package publishing must use public environment and branch protections instead of a private actor allowlist",
    ),
];

/// Required permissionless wrapper registry verification markers.
const WRAPPER_VERIFY_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "needs:\n      - prepare\n      - publish",
        "package verification must wait for prepare and publish",
    ),
    (
        "needs.publish.result == 'skipped'",
        "package verification must support already-published recovery",
    ),
    (
        "permissions: {}",
        "package verification must run without GitHub permissions",
    ),
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&check_wrapper_verify_isolation),
    size_of_val(&reject_wrapper_verify_credentials),
    size_of_val(&require_python_release_toolchain),
    size_of_val(&require_wrapper_verify_contract),
];

impl WrapperReleasePolicy for HostedActionsCheck {
    fn check_wrapper_release_assets(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        require_contains(
            workflow.contents.as_str(),
            "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-native-release-assets --",
            format!(
                "{}: package publish must verify native release assets before publishing wrappers",
                workflow.path.display()
            )
            .as_str(),
            findings,
        );
    }

    fn check_wrapper_release_base(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        for &(needle, message) in REJECTED_WRAPPER_SNIPPETS {
            reject_lines(workflow, needle, message, findings);
        }
        for (needle, message) in [
            (
                "workflow_dispatch:",
                "package publish must expose only its guarded top-level dispatch entrypoint",
            ),
            (
                "[ \"$GITHUB_REF\" != \"refs/tags/$release_ref\" ]",
                "package publish must fail outside the exact release tag",
            ),
            (
                "ref: ${{ inputs.release_commit }}",
                "package publish must build the resolved native release commit",
            ),
            (
                "release_target=\"$(gh api --jq '.target_commitish'",
                "package publish must bind its source to the GitHub release target",
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

    fn check_wrapper_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        self.check_wrapper_release_base(workflow, findings);
        if workflow.path.ends_with("publish-pypi.yml") {
            require_python_release_toolchain(workflow, findings);
        }
        for (needle, message) in [
            (
                "needs: prepare",
                "package publish credentials must be isolated to a post-prepare job",
            ),
            (
                "actions/upload-artifact@",
                "package publish prepare jobs must upload verified artifacts",
            ),
            (
                "actions/download-artifact@",
                "package publish jobs must publish downloaded verified artifacts",
            ),
        ] {
            require_contains(
                workflow.contents.as_str(),
                needle,
                format!("{}: {message}", workflow.path.display()).as_str(),
                findings,
            );
        }
        check_wrapper_verify_isolation(workflow, findings);
    }
}

/// Require wrapper registry verification to run without publication credentials.
fn check_wrapper_verify_isolation(workflow: &Workflow, findings: &mut Vec<String>) {
    let source = workflow.contents.as_str();
    let Some((prepare_source, publish_and_verify_source)) = source.split_once("\n  publish:\n")
    else {
        findings.push(format!(
            "{}: package release must use prepare, publish, and verify jobs",
            workflow.path.display()
        ));
        return;
    };
    let Some((publish_source, verify_source)) =
        publish_and_verify_source.split_once("\n  verify:\n")
    else {
        findings.push(format!(
            "{}: package release is missing its permissionless verification job",
            workflow.path.display()
        ));
        return;
    };
    reject_wrapper_verify_credentials(
        workflow,
        (prepare_source, publish_source, verify_source),
        findings,
    );
    require_wrapper_verify_contract(workflow, verify_source, findings);
}

/// Reject OIDC permission outside the wrapper upload job.
fn reject_wrapper_verify_credentials(
    workflow: &Workflow,
    job_sources: (&str, &str, &str),
    findings: &mut Vec<String>,
) {
    let (prepare_source, publish_source, verify_source) = job_sources;
    if prepare_source.contains("id-token: write") || verify_source.contains("id-token: write") {
        findings.push(format!(
            "{}: only the package upload job may request an OIDC token",
            workflow.path.display()
        ));
    }
    if publish_source.contains("name: Verify ") {
        findings.push(format!(
            "{}: post-publish registry verification must not retain OIDC permission",
            workflow.path.display()
        ));
    }
}

/// Require the pinned Python build and validation toolchain.
fn require_python_release_toolchain(workflow: &Workflow, findings: &mut Vec<String>) {
    for (needle, message) in [
        (
            "python-version: \"3.14.6\"",
            "PyPI packaging must use the pinned Python release version",
        ),
        (
            "build==1.5.1",
            "PyPI packaging must use the pinned build frontend",
        ),
        (
            "pyproject-build --installer uv",
            "PyPI packaging must build artifacts through pinned Rust-based uv isolation",
        ),
        (
            "ruff==0.15.21",
            "PyPI packaging must run the pinned Ruff quality gate",
        ),
        (
            "ty==0.0.58",
            "PyPI packaging must run the pinned strict type checker",
        ),
        (
            "python3 -m unittest discover -s packages/tovuk-py/tests",
            "PyPI packaging must run the Python unit-test suite",
        ),
    ] {
        require_contains(
            workflow.contents.as_str(),
            needle,
            format!("{}: {message}", workflow.path.display()).as_str(),
            findings,
        );
    }
    reject_lines(
        workflow,
        "python -m pip install",
        "PyPI packaging must not bootstrap release tooling through mutable pip state",
        findings,
    );
}

/// Require the complete permissionless wrapper verification contract.
fn require_wrapper_verify_contract(
    workflow: &Workflow,
    verify_source: &str,
    findings: &mut Vec<String>,
) {
    for &(needle, message) in WRAPPER_VERIFY_REQUIREMENTS {
        require_contains(
            verify_source,
            needle,
            format!("{}: {message}", workflow.path.display()).as_str(),
            findings,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Workflow, check_wrapper_verify_isolation};

    /// Minimal three-job wrapper release with permissionless verification.
    const ISOLATED_RELEASE: &str = concat!(
        "jobs:\n  prepare:\n    permissions:\n      contents: read\n",
        "\n  publish:\n    permissions:\n      id-token: write\n",
        "\n  verify:\n    needs:\n      - prepare\n      - publish\n",
        "    if: needs.publish.result == 'skipped'\n    permissions: {}\n",
    );

    /// Verify that permissionless post-publish verification is accepted.
    ///
    /// # Panics
    ///
    /// Panics when the isolated three-job contract emits a finding.
    #[test]
    fn wrapper_verify_isolation_accepts_permissionless_job() {
        let workflow = Workflow {
            contents: ISOLATED_RELEASE.to_owned(),
            path: PathBuf::from(".github/workflows/publish-npm.yml"),
        };
        let mut findings = Vec::new();

        check_wrapper_verify_isolation(&workflow, &mut findings);

        assert!(findings.is_empty(), "permissionless verification must pass");
    }

    /// Verify that verification cannot retain the publication OIDC permission.
    ///
    /// # Panics
    ///
    /// Panics when an OIDC-capable verification job is accepted.
    #[test]
    fn wrapper_verify_isolation_rejects_oidc_permission() {
        let workflow = Workflow {
            contents: ISOLATED_RELEASE.replace(
                "    permissions: {}",
                "    permissions:\n      id-token: write",
            ),
            path: PathBuf::from(".github/workflows/publish-pypi.yml"),
        };
        let mut findings = Vec::new();

        check_wrapper_verify_isolation(&workflow, &mut findings);

        assert_eq!(findings.len(), 0x2, "OIDC verification must fail policy");
    }
}
