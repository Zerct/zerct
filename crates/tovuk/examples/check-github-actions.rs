//! GitHub Actions policy checks for the public Tovuk repository.

#[path = "check-github-actions/policy.rs"]
mod github_actions_policy;

use std::{
    fs,
    process::{Command, ExitCode},
};

use github_actions_policy::{
    Workflow, contains_cargo_publish_command, reject_javascript_lint_tools, reject_lines,
    reject_retired_cache_action, reject_useblacksmith, require_contains, workflow_corpus,
    workflows,
};

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
    let mut findings = Vec::new();

    reject_global_matches(&workflows, &mut findings);
    require_check_all_hooks(&mut findings)?;
    for workflow in &workflows {
        check_workflow(workflow, &mut findings);
    }
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
            "github.event.pull_request.head.repo.full_name == github.repository",
            "public self-hosted pull_request jobs must require same-repository heads",
        ),
        (
            "github.event.pull_request.base.ref == 'main'",
            "public self-hosted pull_request jobs must require base branch main",
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
    let path = workflow.path.to_string_lossy();
    if path.ends_with("publish-native-binaries.yml") {
        require_contains(
            workflow.contents.as_str(),
            "aarch64-unknown-linux-gnu",
            "publish-native-binaries.yml must build every native target used by public package wrappers",
            findings,
        );
    }
    if path.ends_with("publish-npm.yml") || path.ends_with("publish-pypi.yml") {
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
