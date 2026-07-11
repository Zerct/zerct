//! Evolvable path policy for current and sanitized historical Git trees.

use alloc::collections::{BTreeMap, BTreeSet};

use crate::{
    helpers::CheckResult,
    repo_hygiene_paths::{is_allowed_public_surface_path, validate_portable_public_paths},
    repo_hygiene_required::{PUBLIC_TREE_POLICY_PATH, require_public_tree_policy_bytes},
};

use std::path::{Path, PathBuf};

use super::{ObjectKind, TreeEntry, git};

/// One recursively listed commit tree and its canonical path set.
struct CommitTree {
    /// Commit object being validated.
    commit: String,
    /// Recursively flattened Git tree entries.
    entries: Vec<TreeEntry>,
    /// Unique portable paths derived from the entries.
    paths: BTreeSet<String>,
    /// Repository containing every referenced object.
    repository: PathBuf,
}

impl CommitTree {
    /// Validate the data-only tree manifest when present or required.
    ///
    /// # Errors
    ///
    /// Returns an error when current policy is absent, malformed, or mismatched.
    fn require_tree_manifest(&self, requirement: ManifestRequirement) -> CheckResult {
        let manifest_entry = self
            .entries
            .iter()
            .find(|entry| return entry.path == PUBLIC_TREE_POLICY_PATH);
        let entry = match (manifest_entry, requirement) {
            (Some(entry), ManifestRequirement::OptionalLegacy | ManifestRequirement::Required) => {
                entry
            }
            (None, ManifestRequirement::OptionalLegacy) => return Ok(()),
            (None, ManifestRequirement::Required) => {
                return Err(format!(
                    "current commit {} lacks {PUBLIC_TREE_POLICY_PATH}",
                    self.commit
                ));
            }
        };
        let contents = check_try!(git::read_object(
            self.repository.as_path(),
            entry.object.as_str(),
            ObjectKind::Blob,
        ));
        return require_public_tree_policy_bytes(contents.as_slice(), &self.paths)
            .map_err(|error| return format!("commit {}: {error}", self.commit));
    }

    /// Preserve core identities and narrowly bound paths introduced after base.
    ///
    /// # Errors
    ///
    /// Returns an error when core or product-surface policy changes.
    fn validate_public_surface(&self, base: &str) -> CheckResult {
        let base_entries = check_try!(git::tree_entries(self.repository.as_path(), base));
        check_try!(require_immutable_core(
            base,
            self.commit.as_str(),
            &base_entries,
            self.entries.as_slice(),
        ));
        let base_paths = base_entries
            .iter()
            .map(|entry| return entry.path.as_str())
            .collect::<BTreeSet<_>>();
        let unapproved = self
            .paths
            .iter()
            .find(|path| return !is_allowed_public_surface_path(path));
        if let Some(path) = unapproved {
            return Err(format!(
                "commit {} contains unapproved public surface path {path}",
                self.commit
            ));
        }
        let introduced = self.paths.iter().find(|path| {
            return !base_paths.contains(path.as_str()) && !is_approved_new_path(path);
        });
        if let Some(path) = introduced {
            return Err(format!(
                "commit {} introduces path outside approved product prefixes: {path}",
                self.commit
            ));
        }
        return Ok(());
    }
}

/// Whether every scanned commit must carry the current tree policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManifestRequirement {
    /// Permit pre-policy commits only during a fully scanned history rewrite.
    OptionalLegacy,
    /// Require the manifest and its exact tree binding.
    Required,
}

/// Commit-tree policy selected by the enforcement boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PathPolicy {
    /// Current commits require a policy; sanitized legacy history may predate it.
    manifest_requirement: ManifestRequirement,
    /// Trusted base whose enforcement core must remain byte-identical.
    public_surface_base: Option<String>,
}

impl PathPolicy {
    /// Build policy for a current commit graph with mandatory tree manifests.
    pub(super) const fn current() -> Self {
        return Self {
            manifest_requirement: ManifestRequirement::Required,
            public_surface_base: None,
        };
    }

    /// Build strict generic policy for a fully scanned legacy rewrite.
    pub(super) const fn historical() -> Self {
        return Self {
            manifest_requirement: ManifestRequirement::OptionalLegacy,
            public_surface_base: None,
        };
    }

    /// Build current policy relative to one immutable trusted base commit.
    pub(super) fn public_surface(base: &str) -> Self {
        return Self {
            manifest_requirement: ManifestRequirement::Required,
            public_surface_base: Some(base.to_owned()),
        };
    }
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000b] = [
    size_of_val(&PathPolicy::current),
    size_of_val(&PathPolicy::historical),
    size_of_val(&PathPolicy::public_surface),
    size_of_val(&commit_has_manifest),
    size_of_val(&commit_paths),
    size_of_val(&core_entries),
    size_of_val(&is_approved_new_path),
    size_of_val(&is_security_core_path),
    size_of_val(&require_current_tip),
    size_of_val(&require_immutable_core),
    size_of_val(&validate_commit_paths),
];

/// Return whether one commit already participates in current tree policy.
///
/// # Errors
///
/// Returns an error when Git cannot read the commit tree.
pub(super) fn commit_has_manifest(repository: &Path, commit: &str) -> CheckResult<bool> {
    let entries = check_try!(git::tree_entries(repository, commit));
    return Ok(entries
        .iter()
        .any(|entry| return entry.path == PUBLIC_TREE_POLICY_PATH));
}

/// Collect each unique flattened tree path.
///
/// # Errors
///
/// Returns an error when recursive tree output repeats a path.
fn commit_paths(commit: &str, entries: &[TreeEntry]) -> CheckResult<BTreeSet<String>> {
    let paths = entries
        .iter()
        .map(|entry| return entry.path.clone())
        .collect::<BTreeSet<_>>();
    if paths.len() != entries.len() {
        return Err(format!("commit {commit} repeats a flattened tree path"));
    }
    return Ok(paths);
}

/// Collect exact mode, kind, and object identities for security-core paths.
fn core_entries(entries: &[TreeEntry]) -> BTreeMap<String, String> {
    return entries
        .iter()
        .filter(|entry| return is_security_core_path(entry.path.as_str()))
        .map(|entry| {
            return (
                entry.path.clone(),
                format!("{} {} {}", entry.mode, entry.kind, entry.object),
            );
        })
        .collect();
}

/// Return whether a path newly introduced after base belongs to product surface.
fn is_approved_new_path(path: &str) -> bool {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| return value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let rust_source = path.starts_with("crates/tovuk/src/") && extension == "rs";
    let npm_source = (path.starts_with("packages/tovuk/bin/")
        || path.starts_with("packages/tovuk/tests/"))
        && extension == "mjs";
    let python_source = (path.starts_with("packages/tovuk-py/src/tovuk/")
        || path.starts_with("packages/tovuk-py/tests/"))
        && extension == "py";
    let docs_source = path.starts_with("docs/")
        && matches!(
            extension.as_str(),
            "css" | "json" | "md" | "mdx" | "svg" | "txt"
        );
    let skill_source = path.starts_with("skills/tovuk/") && extension == "md";
    return rust_source
        || npm_source
        || python_source
        || docs_source
        || skill_source
        || path == "docs/.mintignore";
}

/// Return whether a path can change future enforcement and must remain immutable.
fn is_security_core_path(path: &str) -> bool {
    const CORE_ROOTS: &[&str] = &[
        ".editorconfig",
        ".gitattributes",
        ".gitignore",
        ".oxlintrc.json",
        ".prettierrc.json",
        ".vacuum.yaml",
        "clippy.toml",
        "dependency-feature-policy.json",
        "deny.toml",
        "rust-toolchain.toml",
        "rustfmt.toml",
    ];
    return CORE_ROOTS.contains(&path)
        || path.starts_with(".cargo/")
        || path.starts_with(".githooks/")
        || path.starts_with(".github/")
        || path.starts_with("checks/")
        || path.starts_with("crates/tovuk/.cargo/")
        || path == "crates/tovuk/Cargo.toml";
}

/// Require one current tip to carry a valid exact tree manifest.
///
/// # Errors
///
/// Returns an error when the tip is legacy, incomplete, or malformed.
pub(super) fn require_current_tip(repository: &Path, commit: &str) -> CheckResult {
    let entries = check_try!(git::tree_entries(repository, commit));
    return validate_commit_paths(repository, commit, &entries, &PathPolicy::current());
}

/// Require every security-core path to match base mode and object.
///
/// # Errors
///
/// Returns an error for a deletion, addition, rename, mode, or content change.
fn require_immutable_core(
    base: &str,
    commit: &str,
    base_entries: &[TreeEntry],
    entries: &[TreeEntry],
) -> CheckResult {
    if core_entries(base_entries) != core_entries(entries) {
        return Err(format!(
            "commit {commit} changes immutable security core from base {base}"
        ));
    }
    return Ok(());
}

/// Validate one commit's portable paths, manifest, and optional trusted base.
///
/// # Errors
///
/// Returns an error when the commit escapes the selected path policy.
pub(super) fn validate_commit_paths(
    repository: &Path,
    commit: &str,
    entries: &[TreeEntry],
    path_policy: &PathPolicy,
) -> CheckResult {
    let paths = check_try!(commit_paths(commit, entries));
    check_try!(validate_portable_public_paths(&paths));
    let tree = CommitTree {
        commit: commit.to_owned(),
        entries: entries.to_vec(),
        paths,
        repository: repository.to_path_buf(),
    };
    if let Some(base) = path_policy.public_surface_base.as_deref() {
        check_try!(tree.validate_public_surface(base));
    }
    return tree.require_tree_manifest(path_policy.manifest_requirement);
}
