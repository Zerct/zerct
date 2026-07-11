//! Crates.io release workflow policy.

use crate::{
    CrateReleasePolicy, HostedActionsCheck, PolicyRequirement, Workflow, reject_lines,
    require_contains,
};

/// Exact minimal job allowed to hold crates.io Trusted Publishing permission.
const CRATE_PUBLISH_JOB: &str = "    name: publish tovuk to crates.io
    needs: prepare
    if: needs.prepare.outputs.exists == 'false'
    runs-on: ubuntu-24.04
    timeout-minutes: 10
    environment: crates
    permissions:
      contents: read
      id-token: write # Required for crates.io Trusted Publishing.
    steps:
      - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 # v7.0.0
        with:
          persist-credentials: false
      - name: Verify pinned Rust toolchain
        env:
          CARGO_HOME: ${{ runner.temp }}/cargo-home
          RUSTUP_HOME: ${{ runner.temp }}/rustup-home
        run: |
          rustup show active-toolchain
          rustc --version --verbose
      - name: Authenticate with crates.io trusted publishing
        id: crates_io_auth
        uses: rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18 # v1.0.5
      - name: Publish the package without re-executing package code
        run: cargo publish --locked --no-verify
        working-directory: crates/tovuk
        env:
          CARGO_HOME: ${{ runner.temp }}/cargo-home
          CARGO_REGISTRY_TOKEN: ${{ steps.crates_io_auth.outputs.token }}
          RUSTUP_HOME: ${{ runner.temp }}/rustup-home
";

/// Sensitive publication capabilities forbidden outside the minimal publish job.
const CREDENTIAL_SNIPPETS: &[PolicyRequirement] = &[
    (
        "environment:",
        "only the minimal crates.io publish job may use the protected environment",
    ),
    (
        "id-token: write",
        "only the minimal crates.io publish job may request an OIDC token",
    ),
    (
        "rust-lang/crates-io-auth-action@",
        "only the minimal crates.io publish job may authenticate to crates.io",
    ),
    (
        "CARGO_REGISTRY_TOKEN",
        "only the minimal crates.io publish job may receive the registry token",
    ),
];

/// Required recovery behavior in the credential-free verification job.
const VERIFY_JOB_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "needs:\n      - prepare\n      - publish",
        "crates.io verification must wait for both prepare and publish",
    ),
    (
        "needs.publish.result == 'skipped'",
        "crates.io verification must support already-published recovery",
    ),
    (
        "permissions: {}",
        "crates.io verification must run without GitHub permissions",
    ),
    (
        "VERSION: ${{ needs.prepare.outputs.version }}",
        "crates.io verification must check the prepared version",
    ),
    (
        "https://crates.io/api/v1/crates/tovuk/$VERSION",
        "crates.io verification must query the exact published version",
    ),
];

/// Compile-time references preserve the named policy helper boundaries.
const _: [usize; 0x3] = [
    size_of_val(&check_crate_release_job_isolation),
    size_of_val(&require_minimal_publish_job),
    size_of_val(&require_verify_job),
];

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
                "workflow_dispatch:",
                "crates.io publication must expose its guarded top-level dispatch entrypoint",
            ),
            (
                "[ \"$GITHUB_REF\" != \"refs/tags/$release_ref\" ]",
                "crates.io publication must fail outside the exact release tag",
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
        self.reject_crate_release_triggers(workflow, findings);
    }

    fn check_crate_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        self.check_crate_release_base(workflow, findings);
        self.require_crates_trusted_publishing(workflow, findings);
        check_crate_release_job_isolation(workflow, findings);
        for (needle, message) in [
            (
                "cargo package --locked",
                "crates.io publish prepare jobs must package the crate before exposing publish credentials",
            ),
            (
                "cargo publish --locked --no-verify",
                "crates.io publish must avoid re-executing package code after authentication",
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
            "crates.io Trusted Publishing rejects workflow_run events",
            findings,
        );
        reject_lines(
            workflow,
            "workflow_call:",
            "crates.io Trusted Publishing must execute as the configured top-level workflow",
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
            "cargo publish --locked --no-verify",
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

/// Apply the three-job credential-isolation and recovery contract.
fn check_crate_release_job_isolation(workflow: &Workflow, findings: &mut Vec<String>) {
    let source = workflow.contents.as_str();
    let Some((prepare_source, publish_and_verify_source)) = source.split_once("\n  publish:\n")
    else {
        findings.push(format!(
            "{}: crates.io release must use ordered prepare, publish, and verify jobs",
            workflow.path.display()
        ));
        return;
    };
    let Some((_, prepare_job_source)) = prepare_source.rsplit_once("\n  prepare:\n") else {
        findings.push(format!(
            "{}: crates.io release is missing its prepare job",
            workflow.path.display()
        ));
        return;
    };
    let Some((publish_job_source, verify_job_source)) =
        publish_and_verify_source.split_once("\n  verify:\n")
    else {
        findings.push(format!(
            "{}: crates.io release is missing its verification job",
            workflow.path.display()
        ));
        return;
    };
    reject_credential_snippets(workflow, "prepare", prepare_job_source, findings);
    require_minimal_publish_job(workflow, publish_job_source, findings);
    reject_credential_snippets(workflow, "verify", verify_job_source, findings);
    require_verify_job(workflow, verify_job_source, findings);
}

/// Reject publication credentials from one unprivileged job section.
fn reject_credential_snippets(
    workflow: &Workflow,
    job_name: &str,
    job_source: &str,
    findings: &mut Vec<String>,
) {
    for &(needle, message) in CREDENTIAL_SNIPPETS {
        if job_source.contains(needle) {
            findings.push(format!(
                "{}: crates.io {job_name} job: {message}",
                workflow.path.display()
            ));
        }
    }
}

/// Require the complete OIDC job to stay at its audited minimal shape.
fn require_minimal_publish_job(
    workflow: &Workflow,
    publish_source: &str,
    findings: &mut Vec<String>,
) {
    if publish_source != CRATE_PUBLISH_JOB {
        findings.push(format!(
            "{}: crates.io OIDC job must match the minimal audited publish job",
            workflow.path.display()
        ));
    }
}

/// Require credential-free verification after a fresh or recovered publication.
fn require_verify_job(workflow: &Workflow, verify_source: &str, findings: &mut Vec<String>) {
    for &(needle, message) in VERIFY_JOB_REQUIREMENTS {
        if !verify_source.contains(needle) {
            findings.push(format!("{}: {message}", workflow.path.display()));
        }
    }
}
