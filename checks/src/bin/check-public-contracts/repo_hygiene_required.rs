use alloc::collections::BTreeSet;

use crate::{helpers::CheckResult, repo_hygiene_git::git_status_success};

use std::path::Path;

/// Contract value named `REQUIRED_IGNORED_PATHS`.
const REQUIRED_IGNORED_PATHS: &[&str] = &[
    ".aws/credentials",
    ".cargo-home/registry/index",
    ".env",
    ".env.local",
    ".git-credentials",
    ".npmrc",
    ".pypirc",
    ".ssh/id_ed25519",
    ".tovuk/example",
    ".terraform/terraform.tfstate",
    "config.jks",
    "config.key",
    "config.keystore",
    "config.p12",
    "config.p8",
    "config.pem",
    "config.pfx",
    "config.secret",
    "crates/tovuk/.cargo/credentials",
    "crates/tovuk/.cargo/credentials.toml",
    "crates/tovuk/coverage/index.html",
    "crates/tovuk/target/example",
    "crates/tovuk/vendor/example/Cargo.toml",
    "debug.sqlite3",
    "docs/.mintlify/example",
    "docs/README.md",
    "example.auto.tfvars",
    "example.tfstate.backup",
    "example.tfvars",
    "example.tfvars.json",
    "nested/.pypirc",
    "node_modules/example",
    "package.crate",
    "package.zip",
    "packages/tovuk-py/build/example",
    "packages/tovuk-py/dist/example",
    "packages/tovuk-py/src/tovuk.egg-info/PKG-INFO",
    "packages/tovuk/.cache/example",
    "packages/tovuk/dist/example",
    "packages/tovuk/node_modules/example",
    "vendor/example/Cargo.toml",
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
    ".github/workflows/recover-publication.yml",
    "native-release-targets.json",
    ".gitignore",
    ".github/actionlint.yaml",
    ".oxlintrc.json",
    ".prettierrc.json",
    ".vacuum.yaml",
    "AGENTS.md",
    "README.md",
    "SECURITY.md",
    "checks/Cargo.lock",
    "checks/Cargo.toml",
    "checks/src/bin/check-dependency-policy.rs",
    "checks/src/bin/check_dependency_policy/active.rs",
    "checks/src/bin/check_dependency_policy/deny.rs",
    "checks/src/bin/check_dependency_policy/graph.rs",
    "checks/src/bin/check_dependency_policy/policy.rs",
    "checks/src/bin/check_dependency_policy/tree.rs",
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
    "checks/src/bin/zig-linker-proxy.rs",
    "checks/src/bin/zig_linker_proxy_tests/verification.rs",
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
    "checks/src/bin/check-github-actions/workflow_policy_publication_tests.rs",
    "checks/src/bin/check-openapi.rs",
    "checks/src/bin/check-openapi/vacuum.rs",
    "checks/src/bin/check-openapi/vacuum_download.rs",
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
    "checks/src/bin/check-public-contracts/mintlify_module/deployment.rs",
    "checks/src/bin/check-public-contracts/mintlify_module/mcp.rs",
    "checks/src/bin/check-public-contracts/mintlify_module/pages.rs",
    "checks/src/bin/check-public-contracts/mintlify_fetch.rs",
    "checks/src/bin/check-public-contracts/mintlify_fetch/cache_identity.rs",
    "checks/src/bin/check-public-contracts/mintlify_fetch_tests/verification.rs",
    "checks/src/bin/check-public-contracts/native_release_targets.rs",
    "checks/src/bin/check-public-contracts/native_release_targets/quality_tools.rs",
    "checks/src/bin/check-public-contracts/native_release_targets/zig_linker_proxy.rs",
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
    "checks/src/bin/check-public-contracts/repo_hygiene_text_tests/verification.rs",
    "checks/src/bin/check-public-contracts/repo_hygiene_tracked.rs",
    "checks/src/bin/check-public-contracts/retired_contracts.rs",
    "checks/src/bin/check-public-contracts/runtime_cli.rs",
    "checks/src/bin/check-public-contracts/script_contracts.rs",
    "checks/src/bin/check-public-contracts/support_contract.rs",
    "checks/src/bin/check-public-contracts/types.rs",
    "crates/tovuk/.cargo/config.toml",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/LICENSE",
    "crates/tovuk/src/cli/api_commands/account.rs",
    "crates/tovuk/src/cli/api_commands/account_tests.rs",
    "crates/tovuk/src/cli/api_commands/api_keys.rs",
    "crates/tovuk/src/cli/api_commands/api_keys_tests.rs",
    "crates/tovuk/src/cli/api_commands/billing.rs",
    "crates/tovuk/src/cli/api_commands/billing_tests.rs",
    "crates/tovuk/src/cli/api_commands/common.rs",
    "crates/tovuk/src/cli/api_commands/common_tests.rs",
    "crates/tovuk/src/cli/api_commands/generic.rs",
    "crates/tovuk/src/cli/api_commands/http.rs",
    "crates/tovuk/src/cli/api_commands/http/transport.rs",
    "crates/tovuk/src/cli/api_commands/http/url_policy.rs",
    "crates/tovuk/src/cli/api_commands/http_tests.rs",
    "crates/tovuk/src/cli/api_commands/http_tests/server.rs",
    "crates/tovuk/src/cli/api_commands/module_root.rs",
    "crates/tovuk/src/cli/api_commands/scrapers.rs",
    "crates/tovuk/src/cli/api_commands/scrapers_tests.rs",
    "crates/tovuk/src/cli/api_commands/support.rs",
    "crates/tovuk/src/cli/api_commands/support_tests.rs",
    "crates/tovuk/src/cli/args/flags.rs",
    "crates/tovuk/src/cli/args/module_root.rs",
    "crates/tovuk/src/cli/args/parser.rs",
    "crates/tovuk/src/cli/args/parser_tests.rs",
    "crates/tovuk/src/cli/args/values.rs",
    "crates/tovuk/src/cli/auth/auth_tests.rs",
    "crates/tovuk/src/cli/auth/keychain.rs",
    "crates/tovuk/src/cli/auth/module_root.rs",
    "crates/tovuk/src/cli/auth/output.rs",
    "crates/tovuk/src/cli/auth/output_tests.rs",
    "crates/tovuk/src/cli/auth/payload.rs",
    "crates/tovuk/src/cli/auth/payload_tests.rs",
    "crates/tovuk/src/cli/auth/token_store.rs",
    "crates/tovuk/src/cli/auth/token_store_tests.rs",
    "crates/tovuk/src/cli/constants.rs",
    "crates/tovuk/src/cli/errors.rs",
    "crates/tovuk/src/cli/help.rs",
    "crates/tovuk/src/cli/module_root.rs",
    "crates/tovuk/src/cli/runtime.rs",
    "crates/tovuk/src/cli/utils/browser.rs",
    "crates/tovuk/src/cli/utils/fields.rs",
    "crates/tovuk/src/cli/utils/module_root.rs",
    "crates/tovuk/src/cli/utils/output.rs",
    "crates/tovuk/src/cli/utils/output_tests.rs",
    "crates/tovuk/src/cli/utils/url.rs",
    "crates/tovuk/src/main.rs",
    "docs/.mintignore",
    "docs/docs.json",
    "docs/openapi.json",
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
    "checks/src/bin/check-all/package_artifacts/cargo_artifact.rs",
    "checks/src/bin/check-all/package_artifacts.rs",
    "checks/src/bin/check-all/python_runtime.rs",
    "checks/src/http_transport.rs",
    "checks/src/http_transport/config.rs",
    "checks/src/http_transport/redirect.rs",
    "checks/src/http_transport/response.rs",
    "checks/src/http_transport_tests/verification.rs",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "skills/tovuk/SKILL.md",
];

/// Source-like paths that broad ignore patterns must never conceal.
const REQUIRED_VISIBLE_PATHS: &[&str] = &[
    ".env.example",
    "Cargo.lock",
    "SECURITY.md",
    "crates/tovuk/.cargo/config.toml",
    "crates/tovuk/src/build/module.rs",
    "crates/tovuk/src/dist/schema.rs",
    "docs/build/guide.mdx",
    "docs/sdks/rust.mdx",
    "packages/tovuk/.npmrc",
    "sdks/rust/src/lib.rs",
];

/// Rust source trees whose tracked modules require explicit public review.
const REVIEWED_RUST_SOURCE_DIRS: &[&str] = &["checks/src/", "crates/tovuk/src/"];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0003] = [
    size_of_val(&require_ignored_paths),
    size_of_val(&require_tracked_paths),
    size_of_val(&require_visible_paths),
];

/// Contract implementation for `require_ignored_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_ignored_paths() -> CheckResult {
    for path in REQUIRED_IGNORED_PATHS {
        check_try!(
            check_try!(git_status_success(&[
                "check-ignore",
                "-q",
                "--no-index",
                path
            ]))
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

    let unpinned_rust_modules = tracked_set
        .iter()
        .filter(|path| {
            return REVIEWED_RUST_SOURCE_DIRS
                .iter()
                .any(|source_dir| return path.starts_with(source_dir))
                && Path::new(path)
                    .extension()
                    .is_some_and(|extension| return extension.eq_ignore_ascii_case("rs"))
                && !required_set.contains(path.as_str());
        })
        .cloned()
        .collect::<Vec<_>>();
    if !unpinned_rust_modules.is_empty() {
        return Err(format!(
            "These public Rust modules must be added to REQUIRED_TRACKED_PATHS:\n{}",
            unpinned_rust_modules.join("\n")
        ));
    }

    return Ok(());
}

/// Require source-like paths to remain visible to Git.
///
/// # Errors
///
/// Returns an error when a broad ignore rule conceals a source-like path.
pub(super) fn require_visible_paths() -> CheckResult {
    for path in REQUIRED_VISIBLE_PATHS {
        if check_try!(git_status_success(&[
            "check-ignore",
            "-q",
            "--no-index",
            path,
        ])) {
            return Err(format!("{path} must not be ignored"));
        }
    }
    return Ok(());
}
