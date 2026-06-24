use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

const WORKFLOW_DIR: &str = ".github/workflows";

#[derive(Debug)]
struct Workflow {
    path: PathBuf,
    contents: String,
}

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

fn workflows() -> Result<Vec<Workflow>, String> {
    let workflow_dir = Path::new(WORKFLOW_DIR);
    if !workflow_dir.is_dir() {
        return Err(format!("missing {WORKFLOW_DIR}"));
    }
    let mut workflows = Vec::new();
    for entry in
        fs::read_dir(workflow_dir).map_err(|error| format!("read {WORKFLOW_DIR}: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("read {WORKFLOW_DIR}: {error}"))?
            .path();
        if !is_workflow_file(&path) {
            continue;
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("read {}: {error}", path.display()))?;
        workflows.push(Workflow { path, contents });
    }
    workflows.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(workflows)
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

fn workflow_corpus() -> Result<String, String> {
    let mut corpus = String::new();
    for workflow in workflows()? {
        corpus.push_str(workflow.contents.as_str());
        corpus.push('\n');
    }
    Ok(corpus)
}

fn reject_lines(workflow: &Workflow, needle: &str, message: &str, findings: &mut Vec<String>) {
    for (line_index, line) in workflow.contents.lines().enumerate() {
        if line.contains(needle) {
            findings.push(format!(
                "{}:{}: {message}",
                workflow.path.display(),
                line_index + 1
            ));
        }
    }
}

fn reject_useblacksmith(workflow: &Workflow, findings: &mut Vec<String>) {
    for (line_index, line) in workflow.contents.lines().enumerate() {
        if line.contains("useblacksmith/cache")
            || line.contains("useblacksmith/setup-go")
            || line.contains("useblacksmith/setup-node")
            || line.contains("useblacksmith/setup-python")
            || line.contains("useblacksmith/setup-ruby")
            || line.contains("useblacksmith/setup-java")
            || line.contains("useblacksmith/rust-cache")
        {
            findings.push(format!(
                "{}:{}: Blacksmith cache forks are forbidden; use official cache-aware actions on GitHub-hosted runners",
                workflow.path.display(),
                line_index + 1
            ));
        }
    }
}

fn reject_retired_cache_action(workflow: &Workflow, findings: &mut Vec<String>) {
    for (line_index, line) in workflow.contents.lines().enumerate() {
        let Some((_prefix, version)) = line.split_once("actions/cache@") else {
            continue;
        };
        let version = version
            .split(|character: char| character.is_whitespace() || matches!(character, '"' | '\''))
            .next()
            .unwrap_or_default();
        if matches!(
            version,
            "main" | "master" | "v0" | "v1" | "v2" | "v3" | "v4"
        ) {
            findings.push(format!(
                "{}:{}: actions/cache must stay on the latest stable major",
                workflow.path.display(),
                line_index + 1
            ));
        }
    }
}

fn reject_javascript_lint_tools(workflow: &Workflow, findings: &mut Vec<String>) {
    for (line_index, line) in workflow.contents.lines().enumerate() {
        if line
            .split(|character: char| !matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-'))
            .any(|token| matches!(token, "eslint" | "prettier" | "tsc"))
        {
            findings.push(format!(
                "{}:{}: JavaScript linters and typecheckers are forbidden in CI; use Rust based checks",
                workflow.path.display(),
                line_index + 1
            ));
        }
    }
}

fn require_contains(haystack: &str, needle: &str, message: &str, findings: &mut Vec<String>) {
    if !haystack.contains(needle) {
        findings.push(message.to_owned());
    }
}

fn contains_cargo_publish_command(contents: &str) -> bool {
    contents.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.contains("cargo build")
            || trimmed.contains("cargo check")
            || trimmed.contains("cargo test")
            || trimmed.contains("cargo clippy")
            || trimmed.contains("cargo package")
            || trimmed.contains("cargo publish")
    })
}

fn is_workflow_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("yml" | "yaml")
    )
}
