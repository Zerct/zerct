use super::{
    github_actions_policy::{Workflow, contains_cargo_publish_command, require_contains},
    release_policy::check_public_package_release_order,
};

pub(super) fn check_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
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
