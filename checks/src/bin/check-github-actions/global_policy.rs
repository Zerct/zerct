//! Repository-wide `GitHub` Actions policy checks.

use std::{fs::read_to_string as read_file_to_string, process::Command};

use super::{
    BLACKSMITH_ACTIONS, CheckResult, GlobalPolicy, HostedActionsCheck, PolicyRequirement, Workflow,
    reject_lines, reject_matching_lines, require_contains,
};

/// Full-check source requirements enforced for every local and CI run.
const CHECK_ALL_SOURCE_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "check-prose-style\", &[\"--self-test\"]",
        "Rust check-all must run the prose style checker self-test",
    ),
    (
        "check-prose-style\", &[]",
        "Rust check-all must run the prose style checker repository scan",
    ),
    (
        "ruff==0.15.21",
        "Rust check-all must run the pinned Python formatter and linter",
    ),
    (
        "ty==0.0.58",
        "Rust check-all must run the pinned strict Python type checker",
    ),
    (
        "packages/tovuk-py/tests",
        "Rust check-all must run the Python unit-test suite",
    ),
];

/// npm package requirements that the canonical Rust check delegates to.
const NPM_CHECK_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "oxlint@1.73.0",
        "the npm package must run the pinned Oxlint wrapper gate",
    ),
    (
        "prettier@3.9.5",
        "the npm package must run the pinned Prettier wrapper gate",
    ),
    (
        "tests/wrapper.test.mjs",
        "the npm package must run the wrapper test suite",
    ),
];

/// Compile-time references preserve the named helper boundary.
const _: [usize; 0x0001] = [size_of_val(&require_node_before_check_all)];

impl GlobalPolicy for HostedActionsCheck {
    fn line_uses_blacksmith(&self, line: &str) -> bool {
        return BLACKSMITH_ACTIONS
            .iter()
            .any(|needle| return line.contains(needle));
    }

    fn reject_global_matches(&self, workflows: &[Workflow], findings: &mut Vec<String>) {
        for workflow in workflows {
            reject_lines(
                workflow,
                "blacksmith-",
                "Blacksmith runners are forbidden; use GitHub-hosted runners",
                findings,
            );
            self.reject_retired_cache_action(workflow, findings);
            self.reject_useblacksmith(workflow, findings);
            reject_lines(
                workflow,
                "pull_request_target:",
                "pull_request_target is forbidden for this public repository",
                findings,
            );
            reject_lines(
                workflow,
                "self-hosted",
                "private and self-hosted runner labels are forbidden; use GitHub-hosted runners",
                findings,
            );
        }
    }

    fn reject_retired_cache_action(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        for indexed_line in workflow.contents.lines().enumerate() {
            self.reject_retired_cache_line(workflow, indexed_line, findings);
        }
    }

    fn reject_retired_cache_line(
        &self,
        workflow: &Workflow,
        indexed_line: (usize, &str),
        findings: &mut Vec<String>,
    ) {
        let (line_index, line) = indexed_line;
        let Some((_prefix, version_suffix)) = line.split_once("actions/cache@") else {
            return;
        };
        let stable_major = version_suffix
            .split(|character: char| {
                return character.is_whitespace() || matches!(character, '"' | '\'');
            })
            .next()
            .unwrap_or_default();
        let retired_pin = ["# v0", "# v1", "# v2", "# v3", "# v4"]
            .iter()
            .any(|marker| return line.trim_end().ends_with(marker));
        if retired_pin
            || matches!(
                stable_major,
                "main" | "master" | "v0" | "v1" | "v2" | "v3" | "v4"
            )
        {
            findings.push(format!(
                "{}:{}: actions/cache must stay on the latest stable major",
                workflow.path.display(),
                line_index.saturating_add(0x1)
            ));
        }
    }

    fn reject_useblacksmith(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        reject_matching_lines(
            workflow,
            "Blacksmith cache forks are forbidden; use official cache-aware actions on GitHub-hosted runners",
            findings,
            |line| return self.line_uses_blacksmith(line),
        );
    }

    fn require_check_all_hooks(
        &self,
        workflows: &[Workflow],
        findings: &mut Vec<String>,
    ) -> CheckResult {
        let check_all = check_try!(
            read_file_to_string("checks/src/bin/check-all.rs")
                .map_err(|error| format!("read checks/src/bin/check-all.rs: {error}"))
        );
        let npm_package = check_try!(
            read_file_to_string("packages/tovuk/package.json")
                .map_err(|error| format!("read packages/tovuk/package.json: {error}"))
        );
        let workflow_corpus = workflows
            .iter()
            .fold(String::new(), |mut corpus, workflow| {
                corpus.push_str(workflow.contents.as_str());
                corpus.push('\n');
                return corpus;
            });
        require_contains(
            workflow_corpus.as_str(),
            "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --",
            "workflows must run the Rust check-all binary so local and CI checks stay aligned",
            findings,
        );
        require_node_before_check_all(workflows, findings);
        for &(needle, message) in CHECK_ALL_SOURCE_REQUIREMENTS {
            require_contains(check_all.as_str(), needle, message, findings);
        }
        for &(needle, message) in NPM_CHECK_REQUIREMENTS {
            require_contains(npm_package.as_str(), needle, message, findings);
        }
        return Ok(());
    }

    fn require_ci_path_filter_contract(&self, workflows: &[Workflow], findings: &mut Vec<String>) {
        let Some(ci_workflow) = workflows
            .iter()
            .find(|workflow| return workflow.path.ends_with("ci.yml"))
        else {
            findings.push("missing .github/workflows/ci.yml".to_owned());
            return;
        };
        for path in [
            ".gitignore",
            ".oxlintrc.json",
            ".prettierrc.json",
            ".typos.toml",
            ".vacuum.yaml",
            "AGENTS.md",
            "checks/**",
            "crates/tovuk/**",
            "deny.toml",
            "native-release-targets.json",
        ] {
            require_contains(
                ci_workflow.contents.as_str(),
                format!("- \"{path}\"").as_str(),
                format!(
                    "{}: CI path filters must include {path}",
                    ci_workflow.path.display()
                )
                .as_str(),
                findings,
            );
        }
    }

    fn run_actionlint(&self, findings: &mut Vec<String>) {
        match Command::new("actionlint").arg("-color").status() {
            Ok(status) if status.success() => {}
            Ok(status) => findings.push(format!("actionlint failed with status {status}")),
            Err(error) => findings.push(format!(
                "actionlint is required; install the native binary before checking workflows: {error}"
            )),
        }
    }
}
/// Require a pinned Node runtime before every full repository check.
fn require_node_before_check_all(workflows: &[Workflow], findings: &mut Vec<String>) {
    let check_command =
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --";
    for workflow in workflows {
        let Some(check_position) = workflow.contents.find(check_command) else {
            continue;
        };
        let setup_position = workflow.contents.find("actions/setup-node@");
        let pinned_version = workflow.contents.find("node-version: \"24.18.0\"");
        let setup_before_check = setup_position
            .is_some_and(|position| return position < check_position)
            && pinned_version.is_some_and(|position| return position < check_position);
        if !setup_before_check {
            findings.push(format!(
                "{}: Node 24.18.0 must be configured before check-all runs JavaScript gates",
                workflow.path.display()
            ));
        }
    }
}
