use std::{fs, process::Command};

use super::github_actions_policy::{
    Workflow, reject_javascript_lint_tools, reject_lines, reject_retired_cache_action,
    reject_useblacksmith, require_contains, workflow_corpus,
};

pub(super) fn reject_global_matches(workflows: &[Workflow], findings: &mut Vec<String>) {
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

pub(super) fn require_check_all_hooks(findings: &mut Vec<String>) -> Result<(), String> {
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
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-prose-style -- --self-test",
        "scripts/check-all.sh must run the prose style checker self-test through Rust",
        findings,
    );
    require_contains(
        check_all.as_str(),
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-prose-style --",
        "scripts/check-all.sh must run the prose style checker repository scan through Rust",
        findings,
    );
    Ok(())
}

pub(super) fn require_ci_path_filter_contract(workflows: &[Workflow], findings: &mut Vec<String>) {
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

pub(super) fn require_public_trusted_ci(workflows: &[Workflow], findings: &mut Vec<String>) {
    if !workflows
        .iter()
        .any(|workflow| workflow.contents.contains("public-trusted-ci"))
    {
        findings.push(
            "no Tovuk public trusted self-hosted runner labels found in workflows".to_owned(),
        );
    }
}

pub(super) fn run_actionlint(findings: &mut Vec<String>) {
    match Command::new("actionlint").arg("-color").status() {
        Ok(status) if status.success() => {}
        Ok(status) => findings.push(format!("actionlint failed with status {status}")),
        Err(error) => findings.push(format!(
            "actionlint is required; install the native binary before checking workflows: {error}"
        )),
    }
}
