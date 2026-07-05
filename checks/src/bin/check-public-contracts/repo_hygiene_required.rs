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
    "checks/Cargo.lock",
    "checks/Cargo.toml",
    "checks/src/bin/check-github-actions.rs",
    "checks/src/bin/check-github-actions/global_policy.rs",
    "checks/src/bin/check-github-actions/path_filter_contract.rs",
    "checks/src/bin/check-github-actions/path_filters.rs",
    "checks/src/bin/check-github-actions/policy.rs",
    "checks/src/bin/check-github-actions/release_policy.rs",
    "checks/src/bin/check-github-actions/release_policy/crates.rs",
    "checks/src/bin/check-github-actions/release_policy/native.rs",
    "checks/src/bin/check-github-actions/release_policy/wrappers.rs",
    "checks/src/bin/check-github-actions/workflow_policy.rs",
    "checks/src/bin/check-openapi.rs",
    "checks/src/bin/check-prose-style.rs",
    "checks/src/bin/check-shell-style.rs",
    "checks/src/bin/check-toml-style.rs",
    "checks/src/check_support.rs",
    "checks/src/lib.rs",
    "checks/src/bin/check-public-contracts/agent_guidance.rs",
    "checks/src/bin/check-public-contracts/cli_contract.rs",
    "checks/src/bin/check-public-contracts/cli_contract/package.rs",
    "checks/src/bin/check-public-contracts/cli_contract/retired.rs",
    "checks/src/bin/check-public-contracts/docs.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/account_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/billing_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/login_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/openapi.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/openapi/examples.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/openapi/operations.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/openapi/schemas.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/paths_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/pricing_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/scraper_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract/status_contract.rs",
    "checks/src/bin/check-public-contracts/docs_navigation.rs",
    "checks/src/bin/check-public-contracts/docs_sources.rs",
    "checks/src/bin/check-public-contracts/helpers.rs",
    "checks/src/bin/check-public-contracts/helpers_io.rs",
    "checks/src/bin/check-public-contracts/helpers_public_copy.rs",
    "checks/src/bin/check-public-contracts/html_visible_copy.rs",
    "checks/src/bin/check-public-contracts/main.rs",
    "checks/src/bin/check-public-contracts/mintlify.rs",
    "checks/src/bin/check-public-contracts/mintlify/copy.rs",
    "checks/src/bin/check-public-contracts/mintlify/mcp.rs",
    "checks/src/bin/check-public-contracts/mintlify/pages.rs",
    "checks/src/bin/check-public-contracts/mintlify_fetch.rs",
    "checks/src/bin/check-public-contracts/native_release_targets.rs",
    "checks/src/bin/check-public-contracts/npm.rs",
    "checks/src/bin/check-public-contracts/npm_package.rs",
    "checks/src/bin/check-public-contracts/npm_runtime.rs",
    "checks/src/bin/check-public-contracts/package_versions.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_git.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_paths.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_required.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_text.rs",
    "checks/src/bin/check-public-contracts/retired_contracts.rs",
    "checks/src/bin/check-public-contracts/runtime_cli.rs",
    "checks/src/bin/check-public-contracts/script_contracts.rs",
    "checks/src/bin/check-public-contracts/support_contract.rs",
    "checks/src/bin/check-public-contracts/types.rs",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/src/main.rs",
    "docs/docs.json",
    "docs/openapi.json",
    "deny.toml",
    "Formula/tovuk.rb",
    "packages/tovuk/package.json",
    "packages/tovuk-py/pyproject.toml",
    "checks/src/bin/check-all.rs",
    "scripts/sync-native-release-targets.sh",
    "scripts/lib/repo-root.sh",
    "scripts/lib/tool-path.sh",
    "skills/tovuk/SKILL.md",
];
const CHECK_GITHUB_ACTIONS_CHECKER_DIR: &str = "checks/src/bin/check-github-actions/";
const PUBLIC_CONTRACT_CHECKER_DIR: &str = "checks/src/bin/check-public-contracts/";

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

    let pinned_checker_dirs = [
        CHECK_GITHUB_ACTIONS_CHECKER_DIR,
        PUBLIC_CONTRACT_CHECKER_DIR,
    ];
    let unpinned_checker_modules = tracked_set
        .iter()
        .filter(|path| {
            pinned_checker_dirs
                .iter()
                .any(|checker_dir| path.starts_with(checker_dir))
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
