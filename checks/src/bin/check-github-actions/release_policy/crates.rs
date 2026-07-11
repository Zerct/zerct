//! Crates.io release workflow policy.

use crate::{CrateReleasePolicy, HostedActionsCheck, Workflow, reject_lines, require_contains};

impl CrateReleasePolicy for HostedActionsCheck {
    fn check_crate_release_base(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        reject_lines(
            workflow,
            "method=skip",
            "package publish workflows must fail closed instead of silently skipping publish auth",
            findings,
        );
        for (needle, message) in [
            (
                "workflow_call:",
                "crates.io publication must be reusable from the native release workflow",
            ),
            (
                "github.ref == 'refs/heads/main'",
                "crates.io publication must be restricted to the main ref",
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
            "github.actor ==",
            "crates.io publication must use public environment and branch protections instead of a private actor allowlist",
            findings,
        );
        reject_lines(
            workflow,
            "workflow_dispatch:",
            "crates.io publication must not bypass the orchestrated native release gate",
            findings,
        );
        self.reject_crate_release_triggers(workflow, findings);
    }

    fn check_crate_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        self.check_crate_release_base(workflow, findings);
        self.require_crates_trusted_publishing(workflow, findings);
        for (needle, message) in [
            (
                "cargo package --locked",
                "crates.io publish prepare jobs must package the crate before exposing publish credentials",
            ),
            (
                "cargo publish --locked",
                "crates.io publish jobs must publish from the checked release source",
            ),
        ] {
            require_contains(
                workflow.contents.as_str(),
                needle,
                format!("{}: {message}", workflow.path.display()).as_str(),
                findings,
            );
        }
        for (needle, message) in [
            (
                "actions/upload-artifact@",
                "crates.io publish must not upload a .crate artifact that cargo publish cannot consume",
            ),
            (
                "actions/download-artifact@",
                "crates.io publish must not download a .crate artifact that cargo publish cannot consume",
            ),
        ] {
            reject_lines(workflow, needle, message, findings);
        }
    }

    fn reject_crate_release_triggers(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        reject_lines(
            workflow,
            "push:",
            "crates.io publication must be invoked by the native release workflow instead of a direct push trigger",
            findings,
        );
        reject_lines(
            workflow,
            "workflow_run:",
            "crates.io publication must use workflow_call instead of workflow_run",
            findings,
        );
    }

    fn require_crates_trusted_publishing(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        let message = format!(
            "{}: crates.io publishes must use Trusted Publishing OIDC",
            workflow.path.display()
        );
        for needle in [
            "runs-on: ubuntu-24.04",
            "CARGO_HOME: ${{ github.workspace }}/.cargo-home",
            "RUSTUP_HOME: ${{ github.workspace }}/.rustup-home",
            "id-token: write",
            "rust-lang/crates-io-auth-action@",
            "CARGO_REGISTRY_TOKEN: ${{ steps.crates_io_auth.outputs.token }}",
        ] {
            require_contains(
                workflow.contents.as_str(),
                needle,
                message.as_str(),
                findings,
            );
        }
        reject_lines(
            workflow,
            "secrets.CARGO_REGISTRY_TOKEN",
            "crates.io publishes must not use a long-lived Cargo registry token secret",
            findings,
        );
    }
}
