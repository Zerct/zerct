use super::github_actions_policy::{
    Workflow, reject_lines, require_contains, require_crates_trusted_publishing,
};

struct ReleaseWorkflowPolicy {
    suffix: &'static str,
    checks: &'static [ReleaseWorkflowCheck],
}

#[derive(Copy, Clone)]
enum ReleaseWorkflowCheck {
    NativeBinaryPublish,
    PackageVersions,
    CratePublish,
    ArtifactPublish,
    NativeReleaseAssets,
}

const NATIVE_BINARY_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    ReleaseWorkflowCheck::NativeBinaryPublish,
    ReleaseWorkflowCheck::PackageVersions,
];
const CRATE_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    ReleaseWorkflowCheck::PackageVersions,
    ReleaseWorkflowCheck::CratePublish,
];
const WRAPPER_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    ReleaseWorkflowCheck::PackageVersions,
    ReleaseWorkflowCheck::ArtifactPublish,
    ReleaseWorkflowCheck::NativeReleaseAssets,
];

const RELEASE_WORKFLOW_POLICIES: &[ReleaseWorkflowPolicy] = &[
    ReleaseWorkflowPolicy {
        suffix: "publish-native-binaries.yml",
        checks: NATIVE_BINARY_RELEASE_CHECKS,
    },
    ReleaseWorkflowPolicy {
        suffix: "publish-crates.yml",
        checks: CRATE_RELEASE_CHECKS,
    },
    ReleaseWorkflowPolicy {
        suffix: "publish-npm.yml",
        checks: WRAPPER_RELEASE_CHECKS,
    },
    ReleaseWorkflowPolicy {
        suffix: "publish-pypi.yml",
        checks: WRAPPER_RELEASE_CHECKS,
    },
];

pub(super) fn check_public_package_release_order(workflow: &Workflow, findings: &mut Vec<String>) {
    let Some(policy) = RELEASE_WORKFLOW_POLICIES
        .iter()
        .find(|policy| workflow.path.ends_with(policy.suffix))
    else {
        return;
    };

    for check in policy.checks {
        match check {
            ReleaseWorkflowCheck::NativeBinaryPublish => {
                check_native_binary_publish_workflow(workflow, findings);
            }
            ReleaseWorkflowCheck::PackageVersions => {
                require_contains(
                    workflow.contents.as_str(),
                    "scripts/check-public-contracts.sh package-versions",
                    format!(
                        "{}: publish workflows must verify all public package versions before publishing",
                        workflow.path.display()
                    )
                    .as_str(),
                    findings,
                );
            }
            ReleaseWorkflowCheck::CratePublish => {
                check_crate_publish_workflow(workflow, findings);
            }
            ReleaseWorkflowCheck::ArtifactPublish => {
                check_artifact_publish_workflow(workflow, findings);
            }
            ReleaseWorkflowCheck::NativeReleaseAssets => {
                require_contains(
                    workflow.contents.as_str(),
                    "./scripts/check-native-release-assets.sh",
                    format!(
                        "{}: package publish must verify native release assets before publishing wrappers",
                        workflow.path.display()
                    )
                    .as_str(),
                    findings,
                );
            }
        }
    }
}

fn check_native_binary_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    require_contains(
        workflow.contents.as_str(),
        "github.ref == 'refs/heads/main'",
        "publish-native-binaries.yml must reject workflow_dispatch release uploads from non-main refs",
        findings,
    );
    require_contains(
        workflow.contents.as_str(),
        "native-release-targets.json",
        "publish-native-binaries.yml must read the native target matrix from native-release-targets.json",
        findings,
    );
    require_contains(
        workflow.contents.as_str(),
        "fromJSON(needs.native-targets.outputs.matrix)",
        "publish-native-binaries.yml must build the native matrix generated from native-release-targets.json",
        findings,
    );
    require_contains(
        workflow.contents.as_str(),
        "needs: [native-targets, release-gate]",
        "publish-native-binaries.yml must not upload native release assets before the release gate passes",
        findings,
    );
    require_contains(
        workflow.contents.as_str(),
        "scripts/check-all.sh",
        "publish-native-binaries.yml release gate must run the full public repository check before publishing assets",
        findings,
    );
    check_blocking_docs_readiness_gate(workflow, findings);
    require_contains(
        workflow.contents.as_str(),
        "matrix.asset_ext",
        "publish-native-binaries.yml must name assets with explicit manifest asset extensions",
        findings,
    );
    require_contains(
        workflow.contents.as_str(),
        ".sha256",
        "publish-native-binaries.yml must publish SHA-256 checksum assets for native binaries",
        findings,
    );
}

fn check_blocking_docs_readiness_gate(workflow: &Workflow, findings: &mut Vec<String>) {
    const READINESS_COMMAND: &str =
        "scripts/check-public-contracts.sh mintlify-agent-readiness https://docs.tovuk.com";
    match docs_readiness_step(workflow.contents.as_str(), READINESS_COMMAND) {
        Some(DocsReadinessStep {
            start_line,
            continues_on_error: true,
        }) => findings.push(format!(
            "{}:{start_line}: live docs agent readiness must be a blocking release gate",
            workflow.path.display()
        )),
        Some(_) => {}
        None => findings.push(
            "publish-native-binaries.yml release gate must verify live docs agent readiness before publishing assets"
                .to_owned(),
        ),
    }
}

struct DocsReadinessStep {
    start_line: usize,
    continues_on_error: bool,
}

fn docs_readiness_step(contents: &str, command: &str) -> Option<DocsReadinessStep> {
    let mut step_start_line = None;
    let mut step_has_command = false;
    let mut step_continues_on_error = false;

    for (line_index, line) in contents.lines().enumerate() {
        if line.trim_start().starts_with("- name:") {
            if step_has_command {
                return Some(DocsReadinessStep {
                    start_line: step_start_line.unwrap_or(line_index + 1),
                    continues_on_error: step_continues_on_error,
                });
            }
            step_start_line = Some(line_index + 1);
            step_has_command = false;
            step_continues_on_error = false;
        }

        if step_start_line.is_some() {
            step_has_command |= line.contains(command);
            step_continues_on_error |= line.trim_start().starts_with("continue-on-error:");
        }
    }

    step_has_command.then(|| DocsReadinessStep {
        start_line: step_start_line.unwrap_or(1),
        continues_on_error: step_continues_on_error,
    })
}

fn check_package_publish_workflow_base(workflow: &Workflow, findings: &mut Vec<String>) {
    reject_lines(
        workflow,
        "method=skip",
        "package publish workflows must fail closed instead of silently skipping publish auth",
        findings,
    );
    for (needle, message) in [
        (
            "workflow_run:",
            "package publish must wait for native binary workflow completion",
        ),
        (
            "Publish native binaries",
            "package publish must depend on the native binary workflow",
        ),
        (
            "github.event.workflow_run.conclusion == 'success'",
            "package publish must reject failed native binary workflow runs",
        ),
        (
            "github.event.workflow_run.event == 'push'",
            "package publish must only trust native binary workflow_run events created by main pushes",
        ),
        (
            "github.event.workflow_run.head_branch == 'main'",
            "package publish must only trust native binary workflow_run events from main",
        ),
        (
            "github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'",
            "manual package publishes must be restricted to the main ref",
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

fn check_crate_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    check_crate_publish_workflow_base(workflow, findings);
    require_crates_trusted_publishing(workflow, findings);
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
            "actions/upload-artifact@v6",
            "crates.io publish must not upload a .crate artifact that cargo publish cannot consume",
        ),
        (
            "actions/download-artifact@v6",
            "crates.io publish must not download a .crate artifact that cargo publish cannot consume",
        ),
    ] {
        reject_lines(workflow, needle, message, findings);
    }
}

fn check_crate_publish_workflow_base(workflow: &Workflow, findings: &mut Vec<String>) {
    reject_lines(
        workflow,
        "method=skip",
        "package publish workflows must fail closed instead of silently skipping publish auth",
        findings,
    );
    for (needle, message) in [
        (
            "push:",
            "crates.io trusted publishing must use a crates.io-supported push trigger",
        ),
        (
            "branches: [main]",
            "crates.io trusted publishing must run only for main pushes",
        ),
        (
            "github.event_name == 'push' && github.ref == 'refs/heads/main'",
            "crates.io trusted publishing must explicitly guard main push events",
        ),
        (
            "github.event_name == 'workflow_dispatch' && github.ref == 'refs/heads/main'",
            "manual package publishes must be restricted to the main ref",
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
        "workflow_run:",
        "crates.io trusted publishing cannot use the workflow_run event",
        findings,
    );
}

fn check_artifact_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    check_package_publish_workflow_base(workflow, findings);
    for (needle, message) in [
        (
            "needs: prepare",
            "package publish credentials must be isolated to a post-prepare job",
        ),
        (
            "actions/upload-artifact@v6",
            "package publish prepare jobs must upload verified artifacts",
        ),
        (
            "actions/download-artifact@v6",
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

#[cfg(test)]
mod tests {
    use super::docs_readiness_step;

    const COMMAND: &str =
        "scripts/check-public-contracts.sh mintlify-agent-readiness https://docs.tovuk.com";

    #[test]
    fn docs_readiness_step_detects_non_blocking_gate() -> Result<(), String> {
        let step = docs_readiness_step(
            r"
jobs:
  release-gate:
    steps:
      - name: Check public agent readiness
        continue-on-error: true
        run: ./scripts/check-public-contracts.sh mintlify-agent-readiness https://docs.tovuk.com
      - name: Run full repository check
        run: scripts/check-all.sh
",
            COMMAND,
        )
        .ok_or_else(|| "readiness step missing".to_owned())?;

        if !step.continues_on_error {
            return Err("readiness step should continue on error".to_owned());
        }
        if step.start_line != 5 {
            return Err(format!(
                "unexpected readiness step line {}",
                step.start_line
            ));
        }
        Ok(())
    }

    #[test]
    fn docs_readiness_step_accepts_blocking_gate() -> Result<(), String> {
        let step = docs_readiness_step(
            r"
jobs:
  release-gate:
    steps:
      - name: Check public agent readiness
        run: ./scripts/check-public-contracts.sh mintlify-agent-readiness https://docs.tovuk.com
      - name: Run full repository check
        run: scripts/check-all.sh
",
            COMMAND,
        )
        .ok_or_else(|| "readiness step missing".to_owned())?;

        if step.continues_on_error {
            return Err("readiness step should block release".to_owned());
        }
        if step.start_line != 5 {
            return Err(format!(
                "unexpected readiness step line {}",
                step.start_line
            ));
        }
        Ok(())
    }
}
