//! Native binary release workflow policy.

use alloc::collections::BTreeSet;

use crate::{
    DocsReadinessBehavior, DocsReadinessStep, DocsReadinessTracker, HostedActionsCheck,
    NativeReleasePolicy, PathFilters as _, PolicyRequirement, Workflow, reject_lines,
    require_contains,
};

/// Required documentation deployment and synchronization command.
const DOCS_DEPLOY_COMMAND: &str =
    "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin deploy-mintlify-docs --";

/// Binary-affecting paths that must trigger the native release workflow.
const NATIVE_RELEASE_PATH_FILTERS: &[&str] = &[
    ".cargo/config.toml",
    ".github/workflows/publish-native-binaries.yml",
    "native-release-targets.json",
    "checks/Cargo.toml",
    "checks/Cargo.lock",
    "checks/src/bin/zig-linker-proxy.rs",
    "crates/tovuk/.cargo/config.toml",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/src/**",
    "rust-toolchain.toml",
];

/// Event and branch header that must own the native release path filters.
const NATIVE_RELEASE_TRIGGER_HEADER: &str =
    "on:\n  workflow_dispatch:\n  push:\n    branches:\n      - main\n    paths:\n";

/// Native release workflow snippets that are forbidden.
const REJECTED_NATIVE_RELEASE_SNIPPETS: &[PolicyRequirement] = &[
    (
        "def build_strategy",
        "inline Python matrix generation is forbidden; use native-release-tool",
    ),
    (
        "node - <<'NODE'",
        "inline Node checksum generation is forbidden; use native-release-tool",
    ),
    (
        "steps.release.outputs",
        "platform build jobs must not own release state",
    ),
    (
        "status.env",
        "native release state must not use mutable status.env handoffs",
    ),
    (
        "github.actor ==",
        "native publishing must use repository protections instead of a private actor gate",
    ),
    (
        "uses: ./.github/workflows/publish-",
        "native publishing must dispatch top-level trusted-publisher workflows through recovery",
    ),
    (
        "2>/dev/null",
        "native release API failures must not be hidden as missing state",
    ),
    (
        "|| true",
        "native release state probes must fail closed on unexpected errors",
    ),
];

/// Native release workflow snippets required by the public publication contract.
const REQUIRED_NATIVE_RELEASE_SNIPPETS: &[PolicyRequirement] = &[
    (
        "github.ref == 'refs/heads/main'",
        "publish-native-binaries.yml must reject workflow_dispatch release uploads from non-main refs",
    ),
    (
        "native-release-targets.json",
        "publish-native-binaries.yml must read the native target matrix from native-release-targets.json",
    ),
    (
        "fromJSON(needs.native-targets.outputs.matrix)",
        "publish-native-binaries.yml must build the native matrix generated from native-release-targets.json",
    ),
    (
        "needs: [native-targets, release-gate]",
        "publish-native-binaries.yml must not upload native release assets before the release gate passes",
    ),
    (
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --",
        "publish-native-binaries.yml release gate must run the full public repository check before publishing assets",
    ),
    (
        "--bin check-release-availability -- \"$version\"",
        "push releases must prove every registry version is unpublished before mutating GitHub state",
    ),
    (
        "--bin native-release-tool -- matrix native-release-targets.json crates/tovuk/Cargo.toml",
        "publish-native-binaries.yml must generate its release matrix with the tracked Rust utility",
    ),
    (
        "matrix.asset_name",
        "publish-native-binaries.yml must use the Rust-generated release asset name",
    ),
    (
        "--bin native-release-tool -- tag crates/tovuk/Cargo.toml",
        "publish-native-binaries.yml must derive its release tag with the tracked Rust utility",
    ),
    (
        "--bin native-release-tool -- prepare-release native-artifact native-release-targets.json crates/tovuk/Cargo.toml",
        "publish-native-binaries.yml must validate the exact artifact set and generate checksums with Rust",
    ),
    (
        "needs: [native-targets, release-gate, build]",
        "native release state and uploads must run on the GitHub-hosted upload job after builds",
    ),
    (
        "runs-on: ${{ matrix.runner }}",
        "native builds must use each tracked GitHub-hosted matrix runner",
    ),
    (
        "cargo build --locked --release --manifest-path checks/Cargo.toml --bin zig-linker-proxy",
        "Linux ARM64 releases must build the tested Rust Zig linker proxy",
    ),
    (
        "CARGO_ZIGBUILD_ZIG_PATH: ${{ github.workspace }}/checks/target/release/zig-linker-proxy",
        "cargo-zigbuild must delegate through the tracked Rust Zig linker proxy",
    ),
    (
        "TOVUK_REAL_ZIG_PATH: ${{ runner.temp }}/zig-0.16.0/zig",
        "the Zig proxy must delegate to the exact pinned Zig executable",
    ),
    (
        "TOVUK_REAL_ZIG_PATH=\"$zig_root/zig\" \"$proxy\" version",
        "the release workflow must smoke test delegation through the Zig proxy",
    ),
    (
        "actions/upload-artifact@",
        "platform build jobs must hand immutable artifacts to the upload jobs",
    ),
    (
        "merge-multiple: true",
        "the central native publisher must merge every immutable matrix artifact",
    ),
    (
        "[ \"$GITHUB_EVENT_NAME\" = \"push\" ]",
        "push reruns must fail closed when an exact release asset already exists",
    ),
    (
        "upload+=(\"native-artifact/$asset_name.sha256\")",
        "manual reruns must resume by uploading only missing checksum assets",
    ),
    (
        "--bin check-native-release-assets -- \"${RELEASE_TAG#v}\" 0 --allow-draft",
        "the central publisher must verify the exact draft asset set before publication",
    ),
    (
        "--bin check-native-release-assets -- \"${RELEASE_TAG#v}\"",
        "the central publisher must verify the complete remote native release",
    ),
    (
        "cmp -- \"$RUNNER_TEMP/$asset_name.built\" \"native-artifact/$asset_name\"",
        "checksum recovery must prove an existing asset matches the rebuilt immutable artifact",
    ),
    (
        "gh release create \"$RELEASE_TAG\"",
        "one central job must create the native release",
    ),
    (
        "--draft=false --latest",
        "a newly created draft must become public only after full asset verification",
    ),
    (
        "actions: write # Dispatch and monitor the credential-isolated recovery workflow.",
        "native publication must isolate workflow-dispatch permission after asset publication",
    ),
    (
        "gh workflow run recover-publication.yml",
        "native publication must dispatch guarded registry recovery after asset publication",
    ),
    (
        "gh run watch \"$recovery_run_id\"",
        "native publication must wait for registry recovery to complete",
    ),
];

impl NativeReleasePolicy for HostedActionsCheck {
    fn check_blocking_docs_readiness_gate(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        match self.docs_readiness_step(workflow.contents.as_str()) {
            Some(DocsReadinessStep {
                behavior: DocsReadinessBehavior::ContinuesOnError,
                start_line,
            }) => findings.push(format!(
                "{}:{start_line}: Mintlify synchronization must be a blocking release gate",
                workflow.path.display()
            )),
            Some(_) => {}
            None => findings.push(
                "publish-native-binaries.yml release gate must deploy docs and wait for Mintlify synchronization before publishing assets"
                    .to_owned(),
            ),
        }
    }

    fn check_native_release_path_filter_contract(
        &self,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    ) {
        require_contains(
            workflow.contents.as_str(),
            NATIVE_RELEASE_TRIGGER_HEADER,
            "publish-native-binaries.yml must use path filters on main pushes while retaining manual recovery",
            findings,
        );
        let actual = self.workflow_path_filters(workflow.contents.as_str());
        let actual_filters = actual.iter().map(String::as_str).collect::<BTreeSet<_>>();
        let required_filters = NATIVE_RELEASE_PATH_FILTERS
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if actual.len() != NATIVE_RELEASE_PATH_FILTERS.len() || actual_filters != required_filters {
            let workflow_path = workflow.path.display();
            findings.push(format!(
                "{workflow_path}: native release push paths must be exactly {NATIVE_RELEASE_PATH_FILTERS:?}; found {actual:?}"
            ));
        }
    }

    fn check_native_release_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        for &(needle, message) in REQUIRED_NATIVE_RELEASE_SNIPPETS {
            require_contains(workflow.contents.as_str(), needle, message, findings);
        }
        for &(needle, message) in REJECTED_NATIVE_RELEASE_SNIPPETS {
            reject_lines(workflow, needle, message, findings);
        }
        self.check_native_release_path_filter_contract(workflow, findings);
        self.check_blocking_docs_readiness_gate(workflow, findings);
    }

    fn docs_readiness_step(&self, contents: &str) -> Option<DocsReadinessStep> {
        let mut tracker = DocsReadinessTracker {
            behavior: DocsReadinessBehavior::Blocking,
            command_seen: None,
            start_line: None,
        };
        let completed_step = contents.lines().enumerate().find_map(|indexed_line| {
            return self.process_docs_readiness_line(indexed_line, &mut tracker);
        });
        if completed_step.is_some() {
            return completed_step;
        }
        if tracker.command_seen.is_none() {
            return None;
        }
        return Some(DocsReadinessStep {
            behavior: tracker.behavior,
            start_line: tracker.start_line.unwrap_or(0x1),
        });
    }

    fn process_docs_readiness_line(
        &self,
        indexed_line: (usize, &str),
        tracker: &mut DocsReadinessTracker,
    ) -> Option<DocsReadinessStep> {
        let (line_index, line) = indexed_line;
        let fallback_line = line_index.saturating_add(0x1);
        let starts_step = line.trim_start().starts_with("- name:");
        if starts_step && tracker.command_seen.is_some() {
            return Some(DocsReadinessStep {
                behavior: tracker.behavior,
                start_line: tracker.start_line.unwrap_or(fallback_line),
            });
        }
        if starts_step {
            tracker.behavior = DocsReadinessBehavior::Blocking;
            tracker.command_seen = None;
            tracker.start_line = Some(fallback_line);
        }
        if tracker.start_line.is_none() {
            return None;
        }
        if line.contains(DOCS_DEPLOY_COMMAND) {
            tracker.command_seen = Some(());
        }
        if line.trim_start().starts_with("continue-on-error:") {
            tracker.behavior = DocsReadinessBehavior::ContinuesOnError;
        }
        return None;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        DocsReadinessBehavior, DocsReadinessStep, HostedActionsCheck, NativeReleasePolicy as _,
        Workflow,
    };

    /// Verify that the blocking Mintlify synchronization step is accepted.
    ///
    /// # Panics
    ///
    /// Panics when the blocking synchronization step is not parsed correctly.
    #[test]
    fn docs_readiness_step_accepts_blocking_gate() {
        let step = HostedActionsCheck.docs_readiness_step(
            "
jobs:
  release-gate:
    steps:
      - name: Deploy docs and wait for sync
        run: cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin deploy-mintlify-docs --
      - name: Run full repository check
        run: cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --
",
        );

        assert_eq!(
            step,
            Some(DocsReadinessStep {
                behavior: DocsReadinessBehavior::Blocking,
                start_line: 0x5,
            }),
            "the live readiness command should be found as a blocking step"
        );
    }

    /// Verify that a non-blocking Mintlify synchronization step is rejected.
    ///
    /// # Panics
    ///
    /// Panics when continue-on-error is not detected on the synchronization step.
    #[test]
    fn docs_readiness_step_detects_non_blocking_gate() {
        let step = HostedActionsCheck.docs_readiness_step(
            "
jobs:
  release-gate:
    steps:
      - name: Deploy docs and wait for sync
        continue-on-error: true
        run: cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin deploy-mintlify-docs --
      - name: Run full repository check
        run: cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --
",
        );

        assert_eq!(
            step,
            Some(DocsReadinessStep {
                behavior: DocsReadinessBehavior::ContinuesOnError,
                start_line: 0x5,
            }),
            "continue-on-error should mark the readiness step as non-blocking"
        );
    }

    /// Verify that the exact binary-affecting release trigger set is accepted.
    ///
    /// # Panics
    ///
    /// Panics when the native release path contract emits a finding.
    #[test]
    fn native_release_paths_accept_exact_contract() {
        let workflow = native_workflow(
            r#"      - ".cargo/config.toml"
      - ".github/workflows/publish-native-binaries.yml"
      - "native-release-targets.json"
      - "checks/Cargo.toml"
      - "checks/Cargo.lock"
      - "checks/src/bin/zig-linker-proxy.rs"
      - "crates/tovuk/.cargo/config.toml"
      - "crates/tovuk/Cargo.toml"
      - "crates/tovuk/Cargo.lock"
      - "crates/tovuk/src/**"
      - "rust-toolchain.toml""#,
        );
        let mut findings = Vec::new();

        HostedActionsCheck.check_native_release_path_filter_contract(&workflow, &mut findings);

        assert!(
            findings.is_empty(),
            "the complete native release trigger contract must be accepted"
        );
    }

    /// Verify that omitting a binary-affecting release trigger is rejected.
    ///
    /// # Panics
    ///
    /// Panics when an incomplete native release path contract is accepted.
    #[test]
    fn native_release_paths_reject_incomplete_contract() {
        let workflow = native_workflow(
            r#"      - ".cargo/config.toml"
      - ".github/workflows/publish-native-binaries.yml"
      - "native-release-targets.json"
      - "checks/Cargo.toml"
      - "checks/Cargo.lock"
      - "checks/src/bin/zig-linker-proxy.rs"
      - "crates/tovuk/Cargo.toml"
      - "crates/tovuk/Cargo.lock"
      - "crates/tovuk/src/**"
      - "rust-toolchain.toml""#,
        );
        let mut findings = Vec::new();

        HostedActionsCheck.check_native_release_path_filter_contract(&workflow, &mut findings);

        assert_eq!(
            findings.len(),
            0x1,
            "an incomplete native release trigger must be rejected"
        );
    }

    /// Build a native-release workflow fixture with the selected path list.
    fn native_workflow(paths: &str) -> Workflow {
        return Workflow {
            contents: format!(
                "on:\n  workflow_dispatch:\n  push:\n    branches:\n      - main\n    paths:\n{paths}\n"
            ),
            path: PathBuf::from(".github/workflows/publish-native-binaries.yml"),
        };
    }
}
