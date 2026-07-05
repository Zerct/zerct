use crate::github_actions_policy::{
    Workflow, reject_lines, require_contains, require_crates_trusted_publishing,
};

pub(super) fn check_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    check_base(workflow, findings);
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

fn check_base(workflow: &Workflow, findings: &mut Vec<String>) {
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
