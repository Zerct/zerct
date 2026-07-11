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
        "workflow_run:",
        "package publish must use workflow_call instead of workflow_run",
    ),
    (
        "workflow_dispatch:",
        "package publication must not bypass the orchestrated native release gate",
    ),
    (
        "github.actor ==",
        "package publishing must use public environment and branch protections instead of a private actor allowlist",
    ),
];

/// Compile-time references preserve the named helper boundary.
const _: [usize; 0x0001] = [size_of_val(&require_python_release_toolchain)];

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
                "workflow_call:",
                "package publish must be reusable from the native release workflow",
            ),
            (
                "github.ref == 'refs/heads/main'",
                "package publish must be restricted to the main ref",
            ),
            (
                "ref: ${{ inputs.release_ref || github.sha }}",
                "package publish recovery must build the exact native release ref",
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
