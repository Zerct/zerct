use std::{collections::BTreeSet, path::Path};

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
    "scripts/deploy-mintlify-docs.sh",
    "scripts/check-native-release-assets.sh",
    "native-release-targets.json",
    ".gitignore",
    ".github/actionlint.yaml",
    ".typos.toml",
    ".vacuum.yaml",
    "AGENTS.md",
    "README.md",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk-public-checks/Cargo.lock",
    "crates/tovuk-public-checks/Cargo.toml",
    "crates/tovuk-public-checks/src/bin/check-github-actions.rs",
    "crates/tovuk-public-checks/src/bin/check-github-actions/path_filters.rs",
    "crates/tovuk-public-checks/src/bin/check-github-actions/policy.rs",
    "crates/tovuk-public-checks/src/bin/check-prose-style.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/agent_guidance.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/cli_contract.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/cli_contract/package.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/cli_contract/retired.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/docs.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/docs_api_contract.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/docs_api_contract/login_contract.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/docs_api_contract/openapi.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/docs_navigation.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/docs_sources.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/helpers.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/helpers_io.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/helpers_public_copy.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/main.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/mintlify.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/mintlify_fetch.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/native_release_targets.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/npm.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/npm_package.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/npm_runtime.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/package_versions.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/repo_hygiene.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/repo_hygiene_git.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/repo_hygiene_paths.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/repo_hygiene_required.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/repo_hygiene_text.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/retired_contracts.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/runtime_cli.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/script_contracts.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/support_contract.rs",
    "crates/tovuk-public-checks/src/bin/check-public-contracts/types.rs",
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
    "scripts/sync-native-release-targets.sh",
    "scripts/lib/repo-root.sh",
    "scripts/lib/tool-path.sh",
    "skills/tovuk/SKILL.md",
];
const PUBLIC_CONTRACT_CHECKER_DIR: &str =
    "crates/tovuk-public-checks/src/bin/check-public-contracts/";

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
    "packages/tovuk/native-release-targets.json",
    "packages/tovuk-py/src/tovuk/native_release_targets.json",
];

pub(crate) fn require_tracked_paths(tracked_set: &BTreeSet<String>) -> CheckResult {
    let required_set = REQUIRED_TRACKED_PATHS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let missing = REQUIRED_TRACKED_PATHS
        .iter()
        .copied()
        .filter(|path| !tracked_set.contains(*path))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "These required public repo files are not tracked:\n{}",
            missing.join("\n")
        ));
    }

    let unpinned_checker_modules = tracked_set
        .iter()
        .filter(|path| {
            path.starts_with(PUBLIC_CONTRACT_CHECKER_DIR)
                && Path::new(path)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
                && !required_set.contains(path.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    if !unpinned_checker_modules.is_empty() {
        return Err(format!(
            "These public contract checker modules must be added to REQUIRED_TRACKED_PATHS:\n{}",
            unpinned_checker_modules.join("\n")
        ));
    }

    Ok(())
}

pub(crate) fn require_ignored_paths() -> CheckResult {
    for path in REQUIRED_IGNORED_PATHS {
        git_status_success(&["check-ignore", "-q", path])?
            .then_some(())
            .ok_or_else(|| format!("{path} must be ignored"))?;
    }
    Ok(())
}
