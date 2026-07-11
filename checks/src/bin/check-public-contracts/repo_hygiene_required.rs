use alloc::collections::BTreeSet;

use crate::{helpers::CheckResult, repo_hygiene_git::git_status_success};

use std::path::Path;

/// Contract value named `CHECK_GITHUB_ACTIONS_CHECKER_DIR`.
const CHECK_GITHUB_ACTIONS_CHECKER_DIR: &str = "checks/src/bin/check-github-actions/";

/// Contract value named `PUBLIC_CONTRACT_CHECKER_DIR`.
const PUBLIC_CONTRACT_CHECKER_DIR: &str = "checks/src/bin/check-public-contracts/";

/// Contract value named `REQUIRED_IGNORED_PATHS`.
const REQUIRED_IGNORED_PATHS: &[&str] = &[
    ".env",
    ".env.local",
    ".npmrc",
    ".pypirc",
    ".tovuk/example",
    "crates/tovuk/target/example",
    "docs/.mintlify/example",
    "node_modules/example",
    "packages/tovuk/dist/example",
    "packages/tovuk/node_modules/example",
];

/// Contract value named `REQUIRED_TRACKED_PATHS`.
const REQUIRED_TRACKED_PATHS: &[&str] = &[
    ".cargo/config.toml",
    ".editorconfig",
    ".gitattributes",
    ".githooks/pre-commit",
    ".githooks/pre-push",
    ".github/actions/setup-quality-tools/action.yml",
    ".github/dependabot.yml",
    ".github/workflows/ci.yml",
    ".github/workflows/docs-deploy.yml",
    ".github/workflows/docs-score.yml",
    ".github/workflows/docs-validate.yml",
    ".github/workflows/publish-crates.yml",
    ".github/workflows/publish-native-binaries.yml",
    ".github/workflows/publish-npm.yml",
    ".github/workflows/publish-pypi.yml",
    "native-release-targets.json",
    ".gitignore",
    ".github/actionlint.yaml",
    ".oxlintrc.json",
    ".prettierrc.json",
    ".typos.toml",
    ".vacuum.yaml",
    "AGENTS.md",
    "README.md",
    "checks/Cargo.lock",
    "checks/Cargo.toml",
    "checks/src/bin/check-dependency-policy.rs",
    "checks/src/bin/check_dependency_policy/deny.rs",
    "checks/src/bin/check_dependency_policy/graph.rs",
    "checks/src/bin/check_dependency_policy/policy.rs",
    "checks/src/bin/check_dependency_policy_tests/verification.rs",
    "checks/src/bin/check-native-release-assets.rs",
    "checks/src/bin/check-native-release-assets/checksum.rs",
    "checks/src/bin/check-native-release-assets/release.rs",
    "checks/src/bin/check_native_release_assets_tests/verification.rs",
    "checks/src/bin/check-pre-commit.rs",
    "checks/src/bin/check-release-availability.rs",
    "checks/src/bin/check_release_availability_tests/verification.rs",
    "checks/src/bin/native-release-tool.rs",
    "checks/src/bin/native_release_tool/checksum.rs",
    "checks/src/bin/native_release_tool/release_artifacts.rs",
    "checks/src/bin/native_release_tool_tests/verification.rs",
    "checks/src/bin/deploy-mintlify-docs.rs",
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
    "checks/src/bin/check-openapi/vacuum.rs",
    "checks/src/bin/check-package-artifacts.rs",
    "checks/src/bin/check-package-artifacts/archive.rs",
    "checks/src/bin/check-package-artifacts/cargo_package.rs",
    "checks/src/bin/check-package-artifacts/npm_package.rs",
    "checks/src/bin/check-package-artifacts/policy.rs",
    "checks/src/bin/check-package-artifacts/python_package.rs",
    "checks/src/bin/check-package-artifacts/zip_archive.rs",
    "checks/src/bin/check-package-artifacts/zip_directory.rs",
    "checks/src/bin/check-package-artifacts/zip_fields.rs",
    "checks/src/bin/check-package-artifacts/zip_format.rs",
    "checks/src/bin/check_package_artifacts_tests/fixtures.rs",
    "checks/src/bin/check_package_artifacts_tests/verification.rs",
    "checks/src/bin/check-prose-style.rs",
    "checks/src/bin/check-shell-style.rs",
    "checks/src/bin/check-toml-style.rs",
    "checks/src/bin/sync-native-release-targets.rs",
    "checks/src/support.rs",
    "checks/src/support/verification.rs",
    "checks/src/lib.rs",
    "checks/src/bin/check-public-contracts/agent_guidance.rs",
    "checks/src/bin/check-public-contracts/cli_contract_module.rs",
    "checks/src/bin/check-public-contracts/cli_contract_module/package.rs",
    "checks/src/bin/check-public-contracts/cli_contract_module/retired.rs",
    "checks/src/bin/check-public-contracts/docs.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/account_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/billing_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/login_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/openapi_module.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/openapi_module/examples.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/openapi_module/operations.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/openapi_module/schemas.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/paths_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/pricing_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/scraper_contract.rs",
    "checks/src/bin/check-public-contracts/docs_api_contract_module/status_contract.rs",
    "checks/src/bin/check-public-contracts/docs_navigation.rs",
    "checks/src/bin/check-public-contracts/docs_sources.rs",
    "checks/src/bin/check-public-contracts/helpers.rs",
    "checks/src/bin/check-public-contracts/helpers_io.rs",
    "checks/src/bin/check-public-contracts/helpers_public_copy.rs",
    "checks/src/bin/check-public-contracts/html_visible_copy.rs",
    "checks/src/bin/check-public-contracts/html_visible_copy_tests/verification.rs",
    "checks/src/bin/check-public-contracts/main.rs",
    "checks/src/bin/check-public-contracts/mintlify_module.rs",
    "checks/src/bin/check-public-contracts/mintlify_module/copy.rs",
    "checks/src/bin/check-public-contracts/mintlify_module/mcp.rs",
    "checks/src/bin/check-public-contracts/mintlify_module/pages.rs",
    "checks/src/bin/check-public-contracts/mintlify_fetch.rs",
    "checks/src/bin/check-public-contracts/mintlify_fetch_tests/verification.rs",
    "checks/src/bin/check-public-contracts/native_release_targets.rs",
    "checks/src/bin/check-public-contracts/native_release_targets/quality_tools.rs",
    "checks/src/bin/check-public-contracts/npm.rs",
    "checks/src/bin/check-public-contracts/npm_package.rs",
    "checks/src/bin/check-public-contracts/npm_runtime.rs",
    "checks/src/bin/check-public-contracts/package_versions.rs",
    "checks/src/bin/check-public-contracts/package_versions_tests/verification.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_git.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_paths.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_paths_tests/verification.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_required.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_text.rs",
    "checks/src/bin/check-public-contracts/retired_contracts.rs",
    "checks/src/bin/check-public-contracts/runtime_cli.rs",
    "checks/src/bin/check-public-contracts/script_contracts.rs",
    "checks/src/bin/check-public-contracts/support_contract.rs",
    "checks/src/bin/check-public-contracts/types.rs",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/LICENSE",
    "crates/tovuk/src/main.rs",
    "docs/docs.json",
    "docs/openapi.json",
    "docs/fonts/OFL-1.1.txt",
    "docs/fonts/PROVENANCE.md",
    "clippy.toml",
    "deny.toml",
    "dependency-feature-policy.json",
    "Formula/tovuk.rb",
    "packages/tovuk/package.json",
    "packages/tovuk/LICENSE",
    "packages/tovuk/install-policy.mjs",
    "packages/tovuk/native-release-targets.json",
    "packages/tovuk/tests/wrapper.test.mjs",
    "packages/tovuk-py/pyproject.toml",
    "packages/tovuk-py/LICENSE",
    "packages/tovuk-py/src/tovuk/native_release_targets.json",
    "checks/src/bin/check-all.rs",
    "checks/src/bin/check-all/package_artifacts.rs",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "skills/tovuk/SKILL.md",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&require_ignored_paths),
    size_of_val(&require_tracked_paths),
];

/// Contract implementation for `require_ignored_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_ignored_paths() -> CheckResult {
    for path in REQUIRED_IGNORED_PATHS {
        check_try!(
            check_try!(git_status_success(&["check-ignore", "-q", path]))
                .then_some(())
                .ok_or_else(|| format!("{path} must be ignored"))
        );
    }
    return Ok(());
}

/// Contract implementation for `require_tracked_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_tracked_paths(tracked_set: &BTreeSet<String>) -> CheckResult {
    let required_set = REQUIRED_TRACKED_PATHS
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let missing = REQUIRED_TRACKED_PATHS
        .iter()
        .copied()
        .filter(|path| return !tracked_set.contains(*path))
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
            return pinned_checker_dirs
                .iter()
                .any(|checker_dir| return path.starts_with(checker_dir))
                && Path::new(path)
                    .extension()
                    .is_some_and(|extension| return extension.eq_ignore_ascii_case("rs"))
                && !required_set.contains(path.as_str());
        })
        .cloned()
        .collect::<Vec<_>>();
    if !unpinned_checker_modules.is_empty() {
        return Err(format!(
            "These public contract checker modules must be added to REQUIRED_TRACKED_PATHS:\n{}",
            unpinned_checker_modules.join("\n")
        ));
    }

    return Ok(());
}
