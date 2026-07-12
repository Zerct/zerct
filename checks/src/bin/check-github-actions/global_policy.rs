//! Repository-wide `GitHub` Actions policy checks.

use std::{fs::read_to_string as read_file_to_string, process::Command};

use super::{
    BLACKSMITH_ACTIONS, CheckResult, GlobalPolicy, HostedActionsCheck, PolicyRequirement, Workflow,
    reject_lines, reject_matching_lines, require_contains,
};

#[cfg(test)]
#[path = "global_policy_tests.rs"]
mod tests;

/// Full-check source requirements enforced for every local and CI run.
const CHECK_ALL_SOURCE_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "actionlint",
        "Rust check-all must run the GitHub Actions syntax checker",
    ),
    (
        "CARGO_AUDIT_LOCKFILES",
        "Rust check-all must audit every public Cargo lockfile",
    ),
    (
        "cargo-machete",
        "Rust check-all must reject unused Rust dependencies",
    ),
    (
        "check-dependency-policy",
        "Rust check-all must run the Rust dependency policy checker",
    ),
    (
        "check-package-artifacts",
        "Rust check-all must validate exact publishable package archives",
    ),
    (
        "check-openapi",
        "Rust check-all must validate the public OpenAPI contract",
    ),
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
        "RUSTDOCFLAGS\", \"-D warnings",
        "Rust check-all must deny every rustdoc warning",
    ),
    (
        "ty==0.0.58",
        "Rust check-all must run the pinned strict Python type checker",
    ),
    (
        "packages/tovuk-py/tests",
        "Rust check-all must run the Python unit-test suite",
    ),
    (
        "build==1.5.1",
        "Rust check-all must build Python artifacts with the pinned frontend",
    ),
    (
        "mint@4.2.578",
        "Rust check-all must run the pinned Mintlify documentation gates",
    ),
    (
        "MINTLIFY_TELEMETRY_DISABLED",
        "Rust check-all must disable Mintlify telemetry",
    ),
    (
        "UV_NO_CACHE",
        "Rust check-all must isolate Python tools from mutable user cache state",
    ),
    (
        "--package=node@24.18.0",
        "Rust check-all must run Mintlify with the pinned supported Node runtime",
    ),
    (
        "zizmor",
        "Rust check-all must run the GitHub Actions security checker",
    ),
    (
        "--no-index",
        "Rust check-all must smoke-test built package artifacts without registry access",
    ),
    (
        "--keep-going",
        "Rust check-all must report all independent compiler and Clippy findings",
    ),
    (
        "--no-fail-fast",
        "Rust check-all must report all independent test failures",
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

/// Exact native quality-tool versions installed by continuous integration.
const QUALITY_TOOL_REQUIREMENTS: &[PolicyRequirement] = &[
    (
        "cargo-audit@0.22.2",
        "quality-tool setup must pin the reviewed cargo-audit release",
    ),
    (
        "cargo-deny@0.20.2",
        "quality-tool setup must pin the cargo-deny CLI used by policy arguments",
    ),
    (
        "cargo-machete@0.9.2",
        "quality-tool setup must pin the reviewed cargo-machete release",
    ),
    (
        "taplo-cli@0.10.0",
        "quality-tool setup must pin the reviewed Taplo release",
    ),
    (
        "typos-cli@1.48.0",
        "quality-tool setup must pin the reviewed typos release",
    ),
    (
        "zizmor@1.26.1",
        "quality-tool setup must pin the reviewed zizmor release",
    ),
];

/// Exact repository-scoped Linux runner label for trusted `main` jobs.
const TRUSTED_LINUX_RUNNER_LABEL: &str = "tovuk-public-linux-x64";
/// Exact repository-scoped macOS ARM runner label used only by the native matrix.
const TRUSTED_MACOS_ARM_RUNNER_LABEL: &str = "tovuk-public-macos-arm64";
/// Complete guarded CI assignment that keeps pull requests and non-main refs hosted.
const TRUSTED_MAIN_RUNNER_ASSIGNMENT: &str = "runs-on: ${{ github.event_name != 'pull_request' && github.ref == 'refs/heads/main' && 'tovuk-public-linux-x64' || 'ubuntu-24.04' }}";

/// Compile-time references preserve the named helper boundary.
const _: [usize; 0x0006] = [
    size_of_val(&expected_trusted_linux_assignments),
    size_of_val(&reject_dangerous_triggers),
    size_of_val(&require_node_before_check_all),
    size_of_val(&require_trusted_runner_routing),
    size_of_val(&read_check_all_sources),
    size_of_val(&runner_assignment_allowed),
];

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
            reject_dangerous_triggers(workflow, findings);
            reject_lines(
                workflow,
                "self-hosted",
                "broad self-hosted runner labels are forbidden; use one exact repository-scoped runner label",
                findings,
            );
            require_trusted_runner_routing(workflow, findings);
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
        let retired_pin = ["# v0", "# v1", "# v2", "# v3", "# v4", "# v5"]
            .iter()
            .any(|marker| return line.trim_end().ends_with(marker));
        if retired_pin
            || matches!(
                stable_major,
                "main" | "master" | "v0" | "v1" | "v2" | "v3" | "v4" | "v5"
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
        let check_all = check_try!(read_check_all_sources());
        let npm_package = check_try!(
            read_file_to_string("packages/tovuk/package.json")
                .map_err(|error| format!("read packages/tovuk/package.json: {error}"))
        );
        let quality_tools = check_try!(
            read_file_to_string(".github/actions/setup-quality-tools/action.yml").map_err(
                |error| format!("read .github/actions/setup-quality-tools/action.yml: {error}")
            )
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
        for &(needle, message) in QUALITY_TOOL_REQUIREMENTS {
            require_contains(quality_tools.as_str(), needle, message, findings);
        }
        return Ok(());
    }

    fn run_actionlint(&self, findings: &mut Vec<String>) {
        match Command::new("actionlint")
            .args(["-config-file", ".github/actionlint.yaml", "-color"])
            .status()
        {
            Ok(status) if status.success() => {}
            Ok(status) => findings.push(format!("actionlint failed with status {status}")),
            Err(error) => findings.push(format!(
                "actionlint is required; install the native binary before checking workflows: {error}"
            )),
        }
    }
}
/// Return the exact trusted Linux assignment count permitted in one workflow.
fn expected_trusted_linux_assignments(workflow: &Workflow) -> usize {
    if workflow.path.ends_with("ci.yml") || workflow.path.ends_with("docs-deploy.yml") {
        return 0x0001;
    }
    if workflow.path.ends_with("publish-native-binaries.yml") {
        return 0x0004;
    }
    return 0x0000;
}

/// Read the root runner and its isolated package artifact implementation.
///
/// # Errors
///
/// Returns an error when either canonical source cannot be read.
fn read_check_all_sources() -> CheckResult<String> {
    let mut corpus = String::new();
    for path in [
        "checks/src/bin/check-all.rs",
        "checks/src/bin/check-all/package_artifacts.rs",
    ] {
        let source = check_try!(
            read_file_to_string(path).map_err(|error| return format!("read {path}: {error}"))
        );
        corpus.push_str(source.as_str());
        corpus.push('\n');
    }
    return Ok(corpus);
}

/// Reject target-context and chained privileged triggers in every workflow.
fn reject_dangerous_triggers(workflow: &Workflow, findings: &mut Vec<String>) {
    reject_lines(
        workflow,
        "pull_request_target",
        "pull_request_target is forbidden; use a read-only pull_request ruleset workflow",
        findings,
    );
    reject_lines(
        workflow,
        "workflow_run",
        "workflow_run is forbidden; use direct or reusable workflow composition",
        findings,
    );
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

/// Require every runner assignment to match the closed hosted/trusted policy.
fn require_trusted_runner_routing(workflow: &Workflow, findings: &mut Vec<String>) {
    let assignments = workflow
        .contents
        .lines()
        .map(str::trim)
        .filter(|line| return line.starts_with("runs-on:"))
        .collect::<Vec<_>>();
    for assignment in &assignments {
        if !runner_assignment_allowed(workflow, assignment) {
            findings.push(format!(
                "{}: runner assignment {assignment:?} is outside the closed public runner policy",
                workflow.path.display()
            ));
        }
    }
    let expected_linux_assignments = expected_trusted_linux_assignments(workflow);
    let actual_linux_assignments = assignments
        .iter()
        .filter(|assignment| return assignment.contains(TRUSTED_LINUX_RUNNER_LABEL))
        .count();
    if actual_linux_assignments != expected_linux_assignments {
        findings.push(format!(
            "{}: expected {expected_linux_assignments} trusted Linux runner assignments, found {actual_linux_assignments}",
            workflow.path.display()
        ));
    }
    if assignments
        .iter()
        .any(|assignment| return assignment.contains(TRUSTED_MACOS_ARM_RUNNER_LABEL))
    {
        findings.push(format!(
            "{}: the macOS ARM runner must be selected only through the reviewed native matrix",
            workflow.path.display()
        ));
    }
}

/// Return whether one complete `runs-on` assignment is approved for its workflow.
fn runner_assignment_allowed(workflow: &Workflow, assignment: &str) -> bool {
    if assignment == "runs-on: ubuntu-24.04" {
        return true;
    }
    if workflow.path.ends_with("ci.yml") {
        return assignment == TRUSTED_MAIN_RUNNER_ASSIGNMENT;
    }
    if workflow.path.ends_with("docs-deploy.yml") {
        return assignment == "runs-on: tovuk-public-linux-x64";
    }
    if workflow.path.ends_with("publish-native-binaries.yml") {
        return matches!(
            assignment,
            "runs-on: ${{ matrix.runner }}" | "runs-on: tovuk-public-linux-x64"
        );
    }
    return false;
}
