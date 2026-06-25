use std::collections::BTreeSet;

use crate::{helpers::CheckResult, repo_hygiene_git::git_status_success};

const REQUIRED_TRACKED_PATHS: &[&str] = &[
    ".github/workflows/ci.yml",
    ".github/workflows/docs-deploy.yml",
    ".github/workflows/docs-score.yml",
    ".github/workflows/docs-validate.yml",
    ".github/workflows/publish-crates.yml",
    ".github/workflows/publish-native-binaries.yml",
    ".github/workflows/publish-npm.yml",
    ".github/workflows/publish-pypi.yml",
    ".gitignore",
    ".github/actionlint.yaml",
    ".typos.toml",
    ".vacuum.yaml",
    "AGENTS.md",
    "README.md",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/examples/check-github-actions.rs",
    "crates/tovuk/examples/check-prose-style.rs",
    "crates/tovuk/examples/check-public-contracts/main.rs",
    "crates/tovuk/examples/check-public-contracts/agent_guidance.rs",
    "crates/tovuk/examples/check-public-contracts/repo_hygiene.rs",
    "crates/tovuk/src/main.rs",
    "docs/docs.json",
    "docs/openapi.json",
    "deny.toml",
    "Formula/tovuk.rb",
    "packages/tovuk/package.json",
    "packages/tovuk-py/pyproject.toml",
    "scripts/check-all.sh",
    "scripts/check-github-actions.sh",
    "scripts/check-openapi.sh",
    "scripts/check-prose-style.sh",
    "scripts/check-public-contracts.sh",
    "scripts/check-shell-style.sh",
    "scripts/check-toml-style.sh",
    "scripts/check-typos.sh",
    "scripts/lib/repo-root.sh",
    "scripts/lib/tool-path.sh",
    "skills/tovuk/SKILL.md",
    "crates/tovuk/examples/check-public-contracts/script_contracts.rs",
];

const REQUIRED_IGNORED_PATHS: &[&str] = &[
    ".env",
    ".env.local",
    ".npmrc",
    ".pypirc",
    ".tovuk/example",
    "crates/tovuk/target/example",
    "docs/.mintlify/example",
    "packages/tovuk/.fallow/cache.bin",
    "node_modules/example",
    "packages/tovuk/dist/example",
    "packages/tovuk/node_modules/example",
];

pub(crate) fn require_tracked_paths(tracked_set: &BTreeSet<String>) -> CheckResult {
    let missing = REQUIRED_TRACKED_PATHS
        .iter()
        .copied()
        .filter(|path| !tracked_set.contains(*path))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "These required public repo files are not tracked:\n{}",
            missing.join("\n")
        ))
    }
}

pub(crate) fn require_ignored_paths() -> CheckResult {
    for path in REQUIRED_IGNORED_PATHS {
        git_status_success(&["check-ignore", "-q", path])?
            .then_some(())
            .ok_or_else(|| format!("{path} must be ignored"))?;
    }
    Ok(())
}
