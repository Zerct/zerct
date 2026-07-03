use std::{
    fs,
    path::{Path, PathBuf},
};

const WORKFLOW_DIR: &str = ".github/workflows";

#[derive(Debug)]
pub(crate) struct Workflow {
    pub(crate) path: PathBuf,
    pub(crate) contents: String,
}

pub(crate) fn workflows() -> Result<Vec<Workflow>, String> {
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

pub(crate) fn workflow_corpus() -> Result<String, String> {
    let mut corpus = String::new();
    for workflow in workflows()? {
        corpus.push_str(workflow.contents.as_str());
        corpus.push('\n');
    }
    Ok(corpus)
}

pub(crate) fn reject_lines(
    workflow: &Workflow,
    needle: &str,
    message: &str,
    findings: &mut Vec<String>,
) {
    reject_matching_lines(workflow, message, findings, |line| line.contains(needle));
}

pub(crate) fn reject_useblacksmith(workflow: &Workflow, findings: &mut Vec<String>) {
    reject_matching_lines(
        workflow,
        "Blacksmith cache forks are forbidden; use official cache-aware actions on GitHub-hosted runners",
        findings,
        |line| {
            BLACKSMITH_ACTIONS
                .iter()
                .any(|needle| line.contains(needle))
        },
    );
}

pub(crate) fn reject_retired_cache_action(workflow: &Workflow, findings: &mut Vec<String>) {
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
            "main" | "master" | "v0" | "v1" | "v2" | "v3" | "v4" | "v5"
        ) {
            findings.push(format!(
                "{}:{}: actions/cache must stay on the latest stable major",
                workflow.path.display(),
                line_index + 1
            ));
        }
    }
}

pub(crate) fn reject_javascript_lint_tools(workflow: &Workflow, findings: &mut Vec<String>) {
    reject_matching_lines(
        workflow,
        "JavaScript linters and typecheckers are forbidden in CI; use Rust based checks",
        findings,
        |line| {
            line
            .split(|character: char| {
                !matches!(character, 'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-')
            })
            .any(|token| matches!(token, "eslint" | "prettier" | "tsc"))
        },
    );
}

pub(crate) fn require_contains(
    haystack: &str,
    needle: &str,
    message: &str,
    findings: &mut Vec<String>,
) {
    if !haystack.contains(needle) {
        findings.push(message.to_owned());
    }
}

pub(crate) fn require_crates_trusted_publishing(workflow: &Workflow, findings: &mut Vec<String>) {
    let message = format!(
        "{}: crates.io publishes must use Trusted Publishing OIDC",
        workflow.path.display()
    );
    for needle in [
        "runs-on: ubuntu-24.04",
        "CARGO_HOME: ${{ github.workspace }}/.cargo-home",
        "RUSTUP_HOME: ${{ github.workspace }}/.rustup-home",
        "id-token: write",
        "rust-lang/crates-io-auth-action@v1",
        "CARGO_REGISTRY_TOKEN: ${{ steps.crates_io_auth.outputs.token }}",
    ] {
        require_contains(
            workflow.contents.as_str(),
            needle,
            message.as_str(),
            findings,
        );
    }
    reject_lines(
        workflow,
        "secrets.CARGO_REGISTRY_TOKEN",
        "crates.io publishes must not use a long-lived Cargo registry token secret",
        findings,
    );
}

pub(crate) fn contains_cargo_publish_command(contents: &str) -> bool {
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

const BLACKSMITH_ACTIONS: &[&str] = &[
    "useblacksmith/cache",
    "useblacksmith/setup-go",
    "useblacksmith/setup-node",
    "useblacksmith/setup-python",
    "useblacksmith/setup-ruby",
    "useblacksmith/setup-java",
    "useblacksmith/rust-cache",
];

fn reject_matching_lines(
    workflow: &Workflow,
    message: &str,
    findings: &mut Vec<String>,
    line_matches: impl Fn(&str) -> bool,
) {
    for (line_index, line) in workflow.contents.lines().enumerate() {
        if line_matches(line) {
            findings.push(format!(
                "{}:{}: {message}",
                workflow.path.display(),
                line_index + 1
            ));
        }
    }
}
