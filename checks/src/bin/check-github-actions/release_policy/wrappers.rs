use crate::github_actions_policy::{Workflow, reject_lines, require_contains};

pub(super) fn check_publish_workflow(workflow: &Workflow, findings: &mut Vec<String>) {
    check_base(workflow, findings);
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

pub(super) fn check_native_release_assets(workflow: &Workflow, findings: &mut Vec<String>) {
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

fn check_base(workflow: &Workflow, findings: &mut Vec<String>) {
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
