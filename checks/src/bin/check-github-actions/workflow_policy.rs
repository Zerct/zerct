//! Per-workflow security and release policy checks.

use super::{HostedActionsCheck, ReleasePolicy as _, Workflow, WorkflowPolicy, require_contains};

impl WorkflowPolicy for HostedActionsCheck {
    fn check_checkout_credentials(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if workflow.contents.contains("actions/checkout@")
            && !workflow.contents.contains("persist-credentials: false")
        {
            findings.push(format!(
                "{}: checkout must set persist-credentials: false",
                workflow.path.display()
            ));
        }
    }

    fn check_docs_deploy_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
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

    fn check_github_hosted_cargo_cache(&self, workflow: &Workflow, findings: &mut Vec<String>) {
        if self.contains_cargo_publish_command(workflow.contents.as_str())
            && !self.uses_current_cache_action(workflow.contents.as_str())
        {
            findings.push(format!(
                "{}: GitHub-hosted Rust jobs must use the current actions/cache@v5",
                workflow.path.display()
            ));
        }
    }

    fn check_secret_workflow_dispatch_policy(
        &self,
        workflow: &Workflow,
        findings: &mut Vec<String>,
    ) {
        if !workflow.contents.contains("workflow_dispatch:")
            || !workflow.contents.contains("secrets.")
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

    fn check_workflow(&self, workflow: &Workflow, findings: &mut Vec<String>) {
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
        self.check_checkout_credentials(workflow, findings);
        self.check_github_hosted_cargo_cache(workflow, findings);
        self.check_public_package_release_order(workflow, findings);
        self.check_docs_deploy_workflow(workflow, findings);
        self.check_secret_workflow_dispatch_policy(workflow, findings);
    }

    fn contains_cargo_publish_command(&self, contents: &str) -> bool {
        return contents.lines().any(|line| {
            let trimmed = line.trim();
            return trimmed.contains("cargo build")
                || trimmed.contains("cargo check")
                || trimmed.contains("cargo test")
                || trimmed.contains("cargo clippy")
                || trimmed.contains("cargo package")
                || trimmed.contains("cargo publish");
        });
    }

    fn uses_current_cache_action(&self, contents: &str) -> bool {
        return contents.lines().any(|line| {
            let trimmed = line.trim();
            return trimmed.contains("uses: actions/cache@")
                && (trimmed.contains("actions/cache@v5") || trimmed.contains("# v5"));
        });
    }
}
