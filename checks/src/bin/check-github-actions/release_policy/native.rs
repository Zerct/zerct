use crate::github_actions_policy::{Workflow, require_contains};

const READINESS_COMMAND: &str = "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- mintlify-agent-readiness https://docs.tovuk.com";

pub(super) fn check_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    for (needle, message) in [
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
            "scripts/check-all.sh",
            "publish-native-binaries.yml release gate must run the full public repository check before publishing assets",
        ),
        (
            "matrix.asset_ext",
            "publish-native-binaries.yml must name assets with explicit manifest asset extensions",
        ),
        (
            ".sha256",
            "publish-native-binaries.yml must publish SHA-256 checksum assets for native binaries",
        ),
    ] {
        require_contains(workflow.contents.as_str(), needle, message, findings);
    }
    check_blocking_docs_readiness_gate(workflow, findings);
}

fn check_blocking_docs_readiness_gate(workflow: &Workflow, findings: &mut Vec<String>) {
    match docs_readiness_step(workflow.contents.as_str()) {
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

fn docs_readiness_step(contents: &str) -> Option<DocsReadinessStep> {
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
            step_has_command |= line.contains(READINESS_COMMAND);
            step_continues_on_error |= line.trim_start().starts_with("continue-on-error:");
        }
    }

    step_has_command.then(|| DocsReadinessStep {
        start_line: step_start_line.unwrap_or(1),
        continues_on_error: step_continues_on_error,
    })
}

#[cfg(test)]
mod tests {
    use super::docs_readiness_step;

    #[test]
    fn docs_readiness_step_detects_non_blocking_gate() -> Result<(), String> {
        let step = docs_readiness_step(
            r"
jobs:
  release-gate:
    steps:
      - name: Check public agent readiness
        continue-on-error: true
        run: cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- mintlify-agent-readiness https://docs.tovuk.com
      - name: Run full repository check
        run: scripts/check-all.sh
",
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
        run: cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-public-contracts -- mintlify-agent-readiness https://docs.tovuk.com
      - name: Run full repository check
        run: scripts/check-all.sh
",
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
