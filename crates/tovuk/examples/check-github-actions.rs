//! GitHub Actions policy checks for the public Tovuk repository.

#[path = "check-github-actions/policy.rs"]
mod github_actions_policy;
#[path = "check-github-actions/path_filters.rs"]
mod path_filters;

use std::{
    collections::BTreeSet,
    fs,
    process::{Command, ExitCode},
};

use github_actions_policy::{
    Workflow, contains_cargo_publish_command, reject_javascript_lint_tools, reject_lines,
    reject_retired_cache_action, reject_useblacksmith, require_contains, workflow_corpus,
    workflows,
};
use path_filters::{path_filter_matches_tracked, workflow_path_filters};

struct ReleaseWorkflowPolicy {
    suffix: &'static str,
    checks: &'static [ReleaseWorkflowCheck],
}

#[derive(Copy, Clone)]
enum ReleaseWorkflowCheck {
    NativeBinaryPublish,
    PackageVersions,
    PackagePublish,
    NativeReleaseAssets,
}

const NATIVE_BINARY_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    ReleaseWorkflowCheck::NativeBinaryPublish,
    ReleaseWorkflowCheck::PackageVersions,
];
const CRATE_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    ReleaseWorkflowCheck::PackageVersions,
    ReleaseWorkflowCheck::PackagePublish,
];
const WRAPPER_RELEASE_CHECKS: &[ReleaseWorkflowCheck] = &[
    ReleaseWorkflowCheck::PackageVersions,
    ReleaseWorkflowCheck::PackagePublish,
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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let workflows = workflows()?;
    let tracked_files = tracked_files()?;
    let mut findings = Vec::new();

    reject_global_matches(&workflows, &mut findings);
    require_check_all_hooks(&mut findings)?;
    for workflow in &workflows {
        check_workflow(workflow, &mut findings);
        check_workflow_path_filters(workflow, &tracked_files, &mut findings);
    }
    require_ci_path_filter_contract(&workflows, &mut findings);
    require_public_trusted_ci(&workflows, &mut findings);
    run_actionlint(&mut findings);

    if findings.is_empty() {
        return Ok(());
    }
    for finding in findings {
        eprintln!("{finding}");
    }
    Err("GitHub Actions policy check failed".to_owned())
}

fn tracked_files() -> Result<BTreeSet<String>, String> {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .map_err(|error| format!("git ls-files failed: {error}"))?;
    if !output.status.success() {
        return Err("git ls-files failed".to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

fn reject_global_matches(workflows: &[Workflow], findings: &mut Vec<String>) {
    for workflow in workflows {
        reject_lines(
            workflow,
            "blacksmith-",
            "Blacksmith runners are forbidden; use Tovuk trusted self-hosted runners or GitHub-hosted runners",
            findings,
        );
        reject_useblacksmith(workflow, findings);
        reject_retired_cache_action(workflow, findings);
        reject_lines(
            workflow,
            "pull_request_target:",
            "pull_request_target is forbidden for this public repository",
            findings,
        );
        reject_javascript_lint_tools(workflow, findings);
    }
}

fn require_check_all_hooks(findings: &mut Vec<String>) -> Result<(), String> {
    let check_all = fs::read_to_string("scripts/check-all.sh")
        .map_err(|error| format!("read scripts/check-all.sh: {error}"))?;
    let all_workflows = workflow_corpus()?;
    require_contains(
        all_workflows.as_str(),
        "scripts/check-all.sh",
        "workflows must run scripts/check-all.sh so local and CI checks stay aligned",
        findings,
    );
    require_contains(
        check_all.as_str(),
        "./scripts/check-prose-style.sh --self-test",
        "scripts/check-all.sh must run the prose style checker self-test",
        findings,
    );
    require_contains(
        check_all.as_str(),
        "./scripts/check-prose-style.sh",
        "scripts/check-all.sh must run the prose style checker repository scan",
        findings,
    );
    Ok(())
}

fn check_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
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
    check_checkout_credentials(workflow, findings);
    check_self_hosted_policy(workflow, findings);
    check_github_hosted_cargo_cache(workflow, findings);
    check_public_package_release_order(workflow, findings);
}

fn check_workflow_path_filters(
    workflow: &Workflow,
    tracked_files: &BTreeSet<String>,
    findings: &mut Vec<String>,
) {
    for path_filter in workflow_path_filters(workflow.contents.as_str()) {
        if !path_filter_matches_tracked(path_filter.as_str(), tracked_files) {
            findings.push(format!(
                "{}: workflow path filter {path_filter:?} does not match tracked files",
                workflow.path.display()
            ));
        }
    }
}

fn require_ci_path_filter_contract(workflows: &[Workflow], findings: &mut Vec<String>) {
    let Some(ci) = workflows
        .iter()
        .find(|workflow| workflow.path.ends_with("ci.yml"))
    else {
        findings.push("missing .github/workflows/ci.yml".to_owned());
        return;
    };
    for path in [
        ".gitignore",
        ".typos.toml",
        ".vacuum.yaml",
        "AGENTS.md",
        "deny.toml",
        "native-release-targets.json",
    ] {
        require_contains(
            ci.contents.as_str(),
            format!("- \"{path}\"").as_str(),
            format!("{}: CI path filters must include {path}", ci.path.display()).as_str(),
            findings,
        );
    }
}

fn check_checkout_credentials(workflow: &Workflow, findings: &mut Vec<String>) {
    if workflow.contents.contains("actions/checkout@")
        && !workflow.contents.contains("persist-credentials: false")
    {
        findings.push(format!(
            "{}: checkout must set persist-credentials: false",
            workflow.path.display()
        ));
    }
}

fn check_self_hosted_policy(workflow: &Workflow, findings: &mut Vec<String>) {
    if !workflow.contents.contains("self-hosted") {
        return;
    }
    for (needle, message) in [
        (
            "public-trusted-ci",
            "public self-hosted jobs must use the public-trusted-ci label",
        ),
        (
            "github.actor == 'kriptoburak'",
            "public self-hosted jobs must require github.actor == kriptoburak",
        ),
        (
            "github.ref == 'refs/heads/main'",
            "public self-hosted push and workflow_dispatch jobs must require refs/heads/main",
        ),
    ] {
        require_contains(
            workflow.contents.as_str(),
            needle,
            format!("{}: {message}", workflow.path.display()).as_str(),
            findings,
        );
    }
    if workflow.contents.contains("pull_request:") {
        for (needle, message) in [
            (
                "github.event.pull_request.head.repo.full_name == github.repository",
                "public self-hosted pull_request jobs must require same-repository heads",
            ),
            (
                "github.event.pull_request.base.ref == 'main'",
                "public self-hosted pull_request jobs must require base branch main",
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

fn check_github_hosted_cargo_cache(workflow: &Workflow, findings: &mut Vec<String>) {
    if contains_cargo_publish_command(workflow.contents.as_str())
        && !workflow.contents.contains("public-trusted-ci")
        && !workflow.contents.contains("actions/cache@v5")
    {
        findings.push(format!(
            "{}: GitHub-hosted Rust jobs must use actions/cache@v5",
            workflow.path.display()
        ));
    }
}

fn check_public_package_release_order(workflow: &Workflow, findings: &mut Vec<String>) {
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
            ReleaseWorkflowCheck::PackagePublish => {
                check_package_publish_workflow(workflow, findings);
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
    require_contains(
        workflow.contents.as_str(),
        "mintlify-agent-readiness https://docs.tovuk.com",
        "publish-native-binaries.yml release gate must verify live docs agent readiness before publishing assets",
        findings,
    );
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

fn check_package_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
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

fn require_public_trusted_ci(workflows: &[Workflow], findings: &mut Vec<String>) {
    if !workflows
        .iter()
        .any(|workflow| workflow.contents.contains("public-trusted-ci"))
    {
        findings.push(
            "no Tovuk public trusted self-hosted runner labels found in workflows".to_owned(),
        );
    }
}

fn run_actionlint(findings: &mut Vec<String>) {
    match Command::new("actionlint").arg("-color").status() {
        Ok(status) if status.success() => {}
        Ok(status) => findings.push(format!("actionlint failed with status {status}")),
        Err(error) => findings.push(format!(
            "actionlint is required; install the native binary before checking workflows: {error}"
        )),
    }
}
