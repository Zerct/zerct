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
    /// Require every path to remain inside the complete public repository surface.
    ///
    /// # Errors
    ///
    /// Returns an error when a tree path belongs outside the public surface.
    fn require_allowed_paths(&self) -> CheckResult {
        let unapproved = self
            .paths
            .iter()
            .find(|path| return !is_allowed_public_surface_path(path));
        return unapproved.map_or(Ok(()), |path| {
            return Err(format!(
                "commit {} contains unapproved public surface path {path}",
                self.commit
            ));
        });
    }

    /// Require additions to be product files or exact authority-core paths.
    ///
    /// # Errors
    ///
    /// Returns an error when one path is introduced outside either boundary.
    fn require_introduced_paths(&self, core: &CoreAuthority) -> CheckResult {
        let introduced = self.paths.iter().find(|path| {
            let authority_core_path =
                is_security_core_path(path) && core.authority_paths.contains(path.as_str());
            return !core.base_paths.contains(path.as_str())
                && !is_approved_new_path(path)
                && !authority_core_path;
        });
        return introduced.map_or(Ok(()), |path| {
            return Err(format!(
                "commit {} introduces path outside approved product prefixes: {path}",
                self.commit
            ));
        });
    }

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

    /// Bind core identities to the base or authority and narrow new paths.
    ///
    /// # Errors
    ///
    /// Returns an error when core or product-surface policy changes.
    fn validate_public_surface(&self, base: &str, authority: &str) -> CheckResult {
        let base_entries = check_try!(git::tree_entries(self.repository.as_path(), base));
        let authority_entries =
            check_try!(git::tree_entries(self.repository.as_path(), authority,));
        let core = CoreAuthority {
            authority: authority.to_owned(),
            authority_core: core_entries(authority_entries.as_slice()),
            authority_paths: authority_entries
                .iter()
                .map(|entry| return entry.path.clone())
                .collect(),
            base: base.to_owned(),
            base_core: core_entries(base_entries.as_slice()),
            base_paths: base_entries
                .iter()
                .map(|entry| return entry.path.clone())
                .collect(),
        };
        check_try!(require_authorized_core(
            self.commit.as_str(),
            &core,
            self.entries.as_slice(),
        ));
        check_try!(self.require_allowed_paths());
        return self.require_introduced_paths(&core);
    }
}

/// Exact base and reviewed-authority trees for one core transition.
struct CoreAuthority {
    /// Reviewed authority object identity.
    authority: String,
    /// Exact enforcement-core map from the reviewed authority.
    authority_core: BTreeMap<String, String>,
    /// Complete path set from the reviewed authority.
    authority_paths: BTreeSet<String>,
    /// Pull-request base object identity.
    base: String,
    /// Exact enforcement-core map from the pull-request base.
    base_core: BTreeMap<String, String>,
    /// Complete path set from the pull-request base.
    base_paths: BTreeSet<String>,
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
    /// Trusted base for path additions and the existing enforcement core.
    public_surface_base: Option<String>,
    /// Reviewed commit whose complete enforcement core may replace the base core.
    security_core_authority: Option<String>,
}

impl PathPolicy {
    /// Build policy for a current commit graph with mandatory tree manifests.
    pub(super) const fn current() -> Self {
        return Self {
            manifest_requirement: ManifestRequirement::Required,
            public_surface_base: None,
            security_core_authority: None,
        };
    }

    /// Build strict generic policy for a fully scanned legacy rewrite.
    pub(super) const fn historical() -> Self {
        return Self {
            manifest_requirement: ManifestRequirement::OptionalLegacy,
            public_surface_base: None,
            security_core_authority: None,
        };
    }

    /// Build current policy relative to a base and exact reviewed core authority.
    pub(super) fn public_surface(base: &str, authority: &str) -> Self {
        return Self {
            manifest_requirement: ManifestRequirement::Required,
            public_surface_base: Some(base.to_owned()),
            security_core_authority: Some(authority.to_owned()),
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
    size_of_val(&require_authorized_core),
    size_of_val(&require_current_tip),
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

/// Return whether a path can change future enforcement and needs exact authority.
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

/// Require every security-core path to match the base or exact reviewed authority.
///
/// # Errors
///
/// Returns an error for a deletion, addition, rename, mode, or content change.
fn require_authorized_core(
    commit: &str,
    core: &CoreAuthority,
    entries: &[TreeEntry],
) -> CheckResult {
    let actual_core = core_entries(entries);
    if actual_core != core.base_core && actual_core != core.authority_core {
        return Err(format!(
            "commit {commit} security core matches neither base {} nor authority {}",
            core.base, core.authority,
        ));
    }
    return Ok(());
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
    match (
        path_policy.public_surface_base.as_deref(),
        path_policy.security_core_authority.as_deref(),
    ) {
        (Some(base), Some(authority)) => {
            check_try!(tree.validate_public_surface(base, authority));
        }
        (None, None) => {}
        (Some(_), None) | (None, Some(_)) => {
            return Err("public-surface policy has an incomplete core authority".to_owned());
        }
    }
    return tree.require_tree_manifest(path_policy.manifest_requirement);
}
