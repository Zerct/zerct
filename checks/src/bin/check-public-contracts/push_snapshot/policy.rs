//! Public-content policy applied to immutable pushed Git objects.

use alloc::collections::BTreeSet;

use crate::{
    helpers::CheckResult,
    repo_hygiene::MAX_SOURCE_FILE_LINES,
    repo_hygiene_paths::{is_forbidden_tracked_path, is_guarded_source_path},
    repo_hygiene_text::{
        MAX_TRACKED_TEXT_BYTES, line_contains_private_repository_marker,
        line_contains_retired_npm_runner_guidance, validate_tracked_text,
    },
};

use core::str::from_utf8;

use std::path::Path;

use tovuk_public_checks::check_support::reject_secret_signatures;

use super::{ObjectKind, TreeEntry, git, policy_paths, tree};

pub(super) use policy_paths::PathPolicy;

/// Tracked launchers and hooks that intentionally retain executable mode.
const ALLOWED_EXECUTABLE_PATHS: &[&str] = &[
    ".githooks/pre-commit",
    ".githooks/pre-push",
    "packages/tovuk/bin/tovuk.mjs",
    "packages/tovuk/install.mjs",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000a] = [
    size_of_val(&reject_forbidden_content),
    size_of_val(&require_object_size),
    size_of_val(&scan_commit_tree),
    size_of_val(&scan_objects),
    size_of_val(&scan_text_object),
    size_of_val(&scan_tree_entry),
    size_of_val(&scan_tree_object),
    size_of_val(&validate_source_policy),
    size_of_val(&PathPolicy::current),
    size_of_val(&PathPolicy::historical),
];

/// Reject secret, private-engine, and retired guidance bytes.
///
/// # Errors
///
/// Returns an error when public bytes contain a forbidden content marker.
fn reject_forbidden_content(label: &str, contents: &[u8]) -> CheckResult {
    check_try!(reject_secret_signatures(label, contents));
    let text = check_try!(
        from_utf8(contents).map_err(|error| return format!("{label} is not UTF-8: {error}"))
    );
    if text.lines().any(line_contains_private_repository_marker) {
        return Err(format!("{label} exposes a private-repository marker"));
    }
    if text.lines().any(line_contains_retired_npm_runner_guidance) {
        return Err(format!("{label} contains retired npm-runner guidance"));
    }
    return Ok(());
}

/// Require every object kind to remain within the public text-size ceiling.
///
/// # Errors
///
/// Returns an error when an object is unreadable or exceeds the size ceiling.
fn require_object_size(repository: &Path, object: &str) -> CheckResult {
    let size = check_try!(git::object_size(repository, object));
    if size > MAX_TRACKED_TEXT_BYTES {
        return Err(format!(
            "Git object {object} exceeds the {MAX_TRACKED_TEXT_BYTES}-byte public ceiling"
        ));
    }
    return Ok(());
}

/// Validate every path, mode, and blob in one newly reachable commit tree.
///
/// # Errors
///
/// Returns an error when a commit tree differs from reviewed public policy.
fn scan_commit_tree(repository: &Path, commit: &str, path_policy: &PathPolicy) -> CheckResult {
    let entries = check_try!(git::tree_entries(repository, commit));
    check_try!(policy_paths::validate_commit_paths(
        repository,
        commit,
        &entries,
        path_policy,
    ));
    for entry in &entries {
        check_try!(scan_tree_entry(repository, commit, entry));
    }
    return Ok(());
}

/// Scan all newly reachable commits, trees, blobs, and annotated tags.
///
/// # Errors
///
/// Returns an error when any reachable object violates public policy.
pub(super) fn scan_objects(
    repository: &Path,
    objects: &BTreeSet<String>,
    path_policy: &PathPolicy,
) -> CheckResult {
    for object in objects {
        check_try!(require_object_size(repository, object));
        let kind = check_try!(git::object_kind(repository, object));
        match kind {
            ObjectKind::Blob => check_try!(scan_text_object(repository, object, kind, "Git blob")),
            ObjectKind::Commit => {
                check_try!(scan_text_object(repository, object, kind, "Git commit"));
                check_try!(scan_commit_tree(repository, object, path_policy));
            }
            ObjectKind::Tag => check_try!(scan_text_object(repository, object, kind, "Git tag")),
            ObjectKind::Tree => check_try!(scan_tree_object(repository, object)),
        }
    }
    return Ok(());
}

/// Validate one canonical textual Git object.
///
/// # Errors
///
/// Returns an error when an object is noncanonical or contains forbidden bytes.
fn scan_text_object(
    repository: &Path,
    object: &str,
    kind: ObjectKind,
    category: &str,
) -> CheckResult {
    let contents = check_try!(git::read_object(repository, object, kind));
    let label = format!("{category} {object}");
    check_try!(validate_tracked_text(label.as_str(), contents.as_slice()));
    return reject_forbidden_content(label.as_str(), contents.as_slice());
}

/// Validate one exact public path and its immutable blob contents.
///
/// # Errors
///
/// Returns an error when path, mode, kind, or bytes violate public policy.
fn scan_tree_entry(repository: &Path, commit: &str, entry: &TreeEntry) -> CheckResult {
    if entry.kind != "blob" || is_forbidden_tracked_path(entry.path.as_str()) {
        return Err(format!(
            "commit {commit} contains forbidden tree entry {} {}",
            entry.kind, entry.path
        ));
    }
    let approved_mode = entry.mode == "100644"
        || (entry.mode == "100755" && ALLOWED_EXECUTABLE_PATHS.contains(&entry.path.as_str()));
    if !approved_mode {
        return Err(format!(
            "commit {commit} contains unapproved mode {} for {}",
            entry.mode, entry.path
        ));
    }
    let contents = check_try!(git::read_object(
        repository,
        entry.object.as_str(),
        ObjectKind::Blob,
    ));
    check_try!(validate_tracked_text(
        entry.path.as_str(),
        contents.as_slice()
    ));
    check_try!(validate_source_policy(
        entry.path.as_str(),
        contents.as_slice(),
    ));
    return reject_forbidden_content(entry.path.as_str(), contents.as_slice());
}

/// Read each tree object and apply size and raw secret-signature policy.
///
/// # Errors
///
/// Returns an error when a tree is unreadable or contains a secret signature.
fn scan_tree_object(repository: &Path, object: &str) -> CheckResult {
    let contents = check_try!(git::read_object(repository, object, ObjectKind::Tree));
    check_try!(reject_secret_signatures(
        format!("Git tree {object}").as_str(),
        contents.as_slice()
    ));
    return tree::validate_raw_tree(repository, object, contents.as_slice());
}

/// Enforce the source-line ceiling and reject Go source additions.
///
/// # Errors
///
/// Returns an error when public source exceeds policy or uses Go.
fn validate_source_policy(path: &str, contents: &[u8]) -> CheckResult {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| return value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("go") {
        return Err(format!("public surface must not contain Go source: {path}"));
    }
    if !is_guarded_source_path(path) {
        return Ok(());
    }
    let source = check_try!(
        from_utf8(contents).map_err(|error| return format!("source {path} is not UTF-8: {error}"))
    );
    let line_count = source.lines().count();
    if line_count > MAX_SOURCE_FILE_LINES {
        return Err(format!(
            "source {path} has {line_count} lines; maximum is {MAX_SOURCE_FILE_LINES}"
        ));
    }
    return Ok(());
}
