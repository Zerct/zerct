//! GitHub Actions policy checks for the public Tovuk repository.

#[path = "check-github-actions/policy.rs"]
mod github_actions_policy;
#[path = "check-github-actions/path_filters.rs"]
mod path_filters;
#[path = "check-github-actions/release_policy.rs"]
mod release_policy;

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
use release_policy::check_public_package_release_order;

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
    check_docs_deploy_workflow(workflow, findings);
    check_secret_workflow_dispatch_policy(workflow, findings);
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
        "checks/**",
        "crates/tovuk/**",
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
        && !workflow.contents.contains("actions/cache@v6")
    {
        findings.push(format!(
            "{}: GitHub-hosted Rust jobs must use actions/cache@v6",
            workflow.path.display()
        ));
    }
}

fn check_docs_deploy_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    if !workflow.path.ends_with("docs-deploy.yml") {
        return;
    }

    require_contains(
        workflow.contents.as_str(),
        "if: github.ref == 'refs/heads/main'",
        "docs-deploy.yml must reject workflow_dispatch deploys from non-main refs before exposing Mintlify secrets",
        findings,
    );
}

fn check_secret_workflow_dispatch_policy(workflow: &Workflow, findings: &mut Vec<String>) {
    if !workflow.contents.contains("workflow_dispatch:") || !workflow.contents.contains("secrets.")
    {
        return;
    }

    require_contains(
        workflow.contents.as_str(),
        "github.ref == 'refs/heads/main'",
        format!(
            "{}: manually dispatched workflows that read repository secrets must be restricted to main",
            workflow.path.display()
        )
        .as_str(),
        findings,
    );
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
