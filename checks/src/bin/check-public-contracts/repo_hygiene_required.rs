use alloc::collections::BTreeSet;

use crate::{
    helpers::CheckResult,
    repo_hygiene_git::{existing_tracked_files, git_status_success},
    repo_hygiene_paths::{
        is_allowed_public_surface_path, is_forbidden_tracked_path, validate_portable_public_paths,
    },
};

use core::{fmt::Write as _, str::from_utf8};

use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};
use sha2::{Digest as _, Sha256};

use std::{
    fs::{FileType, read_to_string, symlink_metadata, write as write_file},
    path::PathBuf,
};

use tovuk_public_checks::check_support::repo_root;

/// Data-only binding for the exact reviewed tracked path set.
pub(super) const PUBLIC_TREE_POLICY_PATH: &str = "public-tree.json";

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
    "docs/.npmrc",
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

/// Durable public files that an evolvable tree policy may never remove.
const REQUIRED_PUBLIC_PATHS: &[&str] = &[
    ".cargo/config.toml",
    ".editorconfig",
    ".gitattributes",
    ".githooks/pre-commit",
    ".githooks/pre-push",
    ".github/actionlint.yaml",
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
    ".github/workflows/trusted-history.yml",
    ".gitignore",
    ".oxlintrc.json",
    ".prettierrc.json",
    ".vacuum.yaml",
    "AGENTS.md",
    "Formula/tovuk.rb",
    "LICENSE",
    PUBLIC_TREE_POLICY_PATH,
    "README.md",
    "SECURITY.md",
    "checks/Cargo.lock",
    "checks/Cargo.toml",
    "checks/src/bin/check-all.rs",
    "checks/src/bin/check-github-actions.rs",
    "checks/src/bin/check-pre-commit.rs",
    "checks/src/bin/check-public-contracts/main.rs",
    "checks/src/lib.rs",
    "clippy.toml",
    "crates/tovuk/.cargo/config.toml",
    "crates/tovuk/Cargo.lock",
    "crates/tovuk/Cargo.toml",
    "crates/tovuk/LICENSE",
    "crates/tovuk/README.md",
    "crates/tovuk/src/main.rs",
    "deny.toml",
    "dependency-feature-policy.json",
    "docs/.mintignore",
    "docs/docs.json",
    "docs/index.mdx",
    "docs/openapi.json",
    "docs/quickstart.mdx",
    "docs/robots.txt",
    "docs/style.css",
    "native-release-targets.json",
    "packages/tovuk-py/LICENSE",
    "packages/tovuk-py/README.md",
    "packages/tovuk-py/pyproject.toml",
    "packages/tovuk-py/src/tovuk/__init__.py",
    "packages/tovuk-py/src/tovuk/__main__.py",
    "packages/tovuk-py/src/tovuk/cli.py",
    "packages/tovuk-py/src/tovuk/native_release_targets.json",
    "packages/tovuk/LICENSE",
    "packages/tovuk/README.md",
    "packages/tovuk/bin/tovuk.mjs",
    "packages/tovuk/install-policy.mjs",
    "packages/tovuk/install.mjs",
    "packages/tovuk/native-release-targets.json",
    "packages/tovuk/package-lock.json",
    "packages/tovuk/package.json",
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
    "sdks/rust/src/lib.rs",
];

/// Canonical serialized public-tree policy.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PublicTreePolicy {
    /// Number of NUL-delimited tracked paths in the digest.
    path_count: u64,
    /// Lowercase SHA-256 of sorted UTF-8 paths with NUL terminators.
    paths_sha256: String,
    /// Policy schema revision.
    schema_version: u32,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0010] = [
    size_of_val(&check_current_public_tree_policy),
    size_of_val(&current_tracked_paths),
    size_of_val(&hash_paths),
    size_of_val(&is_unsafe_policy_destination),
    size_of_val(&policy_for_paths),
    size_of_val(&policy_path),
    size_of_val(&read_policy_source),
    size_of_val(&render_public_tree_policy),
    size_of_val(&require_ignored_paths),
    size_of_val(&require_public_surface),
    size_of_val(&require_public_tree_policy_bytes),
    size_of_val(&require_required_paths),
    size_of_val(&require_tracked_paths),
    size_of_val(&require_visible_paths),
    size_of_val(&reviewed_tracked_paths),
    size_of_val(&synchronize_public_tree_policy),
];

/// Validate the current index against its canonical data-only tree policy.
///
/// # Errors
///
/// Returns an error when tracked paths or the policy bytes drift.
pub(super) fn check_current_public_tree_policy() -> CheckResult {
    drop(check_try!(reviewed_tracked_paths()));
    return Ok(());
}

/// Read the current stage-zero tracked path set.
///
/// # Errors
///
/// Returns an error when Git cannot list canonical tracked paths.
fn current_tracked_paths() -> CheckResult<BTreeSet<String>> {
    return existing_tracked_files().map(|paths| return paths.into_iter().collect::<BTreeSet<_>>());
}

/// Encode one sorted path set as lowercase SHA-256.
///
/// # Errors
///
/// Returns an error when digest bytes cannot be formatted.
fn hash_paths(paths: &BTreeSet<String>) -> CheckResult<String> {
    let mut digest = Sha256::new();
    for path in paths {
        digest.update(path.as_bytes());
        digest.update([0x00]);
    }
    let mut encoded = String::with_capacity(0x0040);
    for byte in digest.finalize() {
        check_try!(
            write!(encoded, "{byte:02x}")
                .map_err(|error| return format!("encode public-tree SHA-256: {error}"))
        );
    }
    return Ok(encoded);
}

/// Return whether a policy destination could redirect or block a regular write.
fn is_unsafe_policy_destination(file_type: FileType) -> bool {
    if file_type.is_dir() || file_type.is_symlink() {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt as _;

        return file_type.is_block_device()
            || file_type.is_char_device()
            || file_type.is_fifo()
            || file_type.is_socket();
    }
    #[cfg(not(unix))]
    {
        return false;
    }
}

/// Construct the canonical policy value for one exact path set.
///
/// # Errors
///
/// Returns an error when path count or hashing fails.
fn policy_for_paths(paths: &BTreeSet<String>) -> CheckResult<PublicTreePolicy> {
    let path_count = check_try!(
        u64::try_from(paths.len())
            .map_err(|error| return format!("count public-tree paths: {error}"))
    );
    return Ok(PublicTreePolicy {
        path_count,
        paths_sha256: check_try!(hash_paths(paths)),
        schema_version: 0x0001,
    });
}

/// Resolve the policy file from the current repository root.
///
/// # Errors
///
/// Returns an error when the repository root cannot be found.
fn policy_path() -> CheckResult<PathBuf> {
    return repo_root().map(|root| return root.join(PUBLIC_TREE_POLICY_PATH));
}

/// Read the current data-only public-tree policy.
///
/// # Errors
///
/// Returns an error when the repository or policy file cannot be read.
fn read_policy_source() -> CheckResult<String> {
    let path = check_try!(policy_path());
    return read_to_string(path.as_path())
        .map_err(|error| return format!("read {PUBLIC_TREE_POLICY_PATH}: {error}"));
}

/// Render the exact canonical JSON policy for one tracked path set.
///
/// # Errors
///
/// Returns an error when the policy cannot be serialized.
pub(super) fn render_public_tree_policy(paths: &BTreeSet<String>) -> CheckResult<String> {
    let policy = check_try!(policy_for_paths(paths));
    let mut rendered = check_try!(
        to_string_pretty(&policy)
            .map_err(|error| return format!("serialize {PUBLIC_TREE_POLICY_PATH}: {error}"))
    );
    rendered.push('\n');
    return Ok(rendered);
}

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

/// Require every current path to remain inside the public repository surface.
///
/// # Errors
///
/// Returns an error for an unsafe, forbidden, or unapproved path.
fn require_public_surface(paths: &BTreeSet<String>) -> CheckResult {
    check_try!(validate_portable_public_paths(paths));
    if let Some(path) = paths.iter().find(|path| {
        return !is_allowed_public_surface_path(path) || is_forbidden_tracked_path(path);
    }) {
        return Err(format!(
            "public-tree policy contains an unapproved path: {path}"
        ));
    }
    return Ok(());
}

/// Verify canonical policy bytes bind one exact sorted tracked path set.
///
/// # Errors
///
/// Returns an error for malformed policy data, drift, or an unsafe path set.
pub(super) fn require_public_tree_policy_bytes(
    source: &[u8],
    paths: &BTreeSet<String>,
) -> CheckResult {
    check_try!(require_public_surface(paths));
    check_try!(require_required_paths(paths));
    let text = check_try!(
        from_utf8(source)
            .map_err(|error| return format!("{PUBLIC_TREE_POLICY_PATH} is not UTF-8: {error}"))
    );
    let actual = check_try!(
        from_str::<PublicTreePolicy>(text)
            .map_err(|error| return format!("parse {PUBLIC_TREE_POLICY_PATH}: {error}"))
    );
    let expected = check_try!(policy_for_paths(paths));
    if actual != expected {
        return Err(format!(
            "{PUBLIC_TREE_POLICY_PATH} does not bind the exact tracked path set"
        ));
    }
    let canonical = check_try!(render_public_tree_policy(paths));
    if canonical.as_bytes() != source {
        return Err(format!(
            "{PUBLIC_TREE_POLICY_PATH} must use canonical generated JSON"
        ));
    }
    return Ok(());
}

/// Require the durable public repository contract to remain present.
///
/// # Errors
///
/// Returns an error when any mandatory public path is absent.
fn require_required_paths(paths: &BTreeSet<String>) -> CheckResult {
    let missing = REQUIRED_PUBLIC_PATHS
        .iter()
        .copied()
        .filter(|path| return !paths.contains(*path))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "These required public repo files are not tracked:\n{}",
        missing.join("\n")
    ));
}

/// Contract implementation for `require_tracked_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_tracked_paths(tracked_set: &BTreeSet<String>) -> CheckResult {
    let source = check_try!(read_policy_source());
    return require_public_tree_policy_bytes(source.as_bytes(), tracked_set);
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

/// Return the exact file set bound by the current data-only public-tree policy.
///
/// # Errors
///
/// Returns an error when the index or policy file is inconsistent.
pub(super) fn reviewed_tracked_paths() -> CheckResult<BTreeSet<String>> {
    let paths = check_try!(current_tracked_paths());
    let source = check_try!(read_policy_source());
    check_try!(require_public_tree_policy_bytes(source.as_bytes(), &paths));
    return Ok(paths);
}

/// Regenerate the data-only policy from the current Git index.
///
/// # Errors
///
/// Returns an error when the destination is unsafe or cannot be written.
pub(super) fn synchronize_public_tree_policy() -> CheckResult {
    let mut paths = check_try!(current_tracked_paths());
    if !paths.contains(PUBLIC_TREE_POLICY_PATH) {
        paths.extend([PUBLIC_TREE_POLICY_PATH.to_owned()]);
    }
    check_try!(require_public_surface(&paths));
    check_try!(require_required_paths(&paths));
    let destination = PUBLIC_TREE_POLICY_PATH;
    let destination_path = check_try!(policy_path());
    if check_try!(
        destination_path
            .try_exists()
            .map_err(|error| return format!("inspect {destination}: {error}"))
    ) {
        let metadata = check_try!(
            symlink_metadata(destination_path.as_path())
                .map_err(|error| return format!("inspect {destination}: {error}"))
        );
        if is_unsafe_policy_destination(metadata.file_type()) {
            return Err(format!("{destination} must be a regular file"));
        }
    }
    let rendered = check_try!(render_public_tree_policy(&paths));
    return write_file(destination_path.as_path(), rendered)
        .map_err(|error| return format!("write {destination}: {error}"));
}

#[cfg(test)]
#[path = "repo_hygiene_required_tests/verification.rs"]
mod tests;
