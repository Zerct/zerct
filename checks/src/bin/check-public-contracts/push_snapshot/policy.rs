//! Public-content policy applied to immutable pushed Git objects.

use alloc::collections::BTreeSet;

use crate::{
    helpers::CheckResult,
    repo_hygiene::MAX_SOURCE_FILE_LINES,
    repo_hygiene_paths::{is_forbidden_tracked_path, is_guarded_source_path},
    repo_hygiene_text::{
        MAX_TRACKED_TEXT_BYTES, line_contains_private_repository_marker,
        line_contains_retired_npm_runner_guidance, reject_private_implementation_terms,
        validate_tracked_text,
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

/// Successful scans shared across all ref updates in one proposed push.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct ScanState {
    /// Blob paths already checked under the layout facet that affects them.
    entries: BTreeSet<ScannedTreeEntry>,
    /// Objects already checked under the exact policy that governed their tree.
    objects: BTreeSet<ScannedObject>,
}

/// Exact object and path-policy identity already checked during one invocation.
type ScannedObject = (PathPolicy, String);

/// One tree-entry identity already checked under one layout policy.
type ScannedTreeEntry = (bool, String, String, String, String);

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000d] = [
    size_of_val(&reject_forbidden_content),
    size_of_val(&require_object_size),
    size_of_val(&scan_commit_tree),
    size_of_val(&scan_objects),
    size_of_val(&scan_objects_with_state),
    size_of_val(&scan_text_object),
    size_of_val(&scan_tree_entry),
    size_of_val(&scan_tree_object),
    size_of_val(&validate_commit_header),
    size_of_val(&validate_commit_text),
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
    check_try!(reject_private_implementation_terms(label, contents));
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
fn scan_commit_tree(
    repository: &Path,
    commit: &str,
    path_policy: &PathPolicy,
    state: &mut ScanState,
) -> CheckResult {
    let entries = check_try!(git::tree_entries(repository, commit));
    check_try!(policy_paths::validate_commit_paths(
        repository,
        commit,
        &entries,
        path_policy,
    ));
    for entry in &entries {
        let identity = (
            path_policy.requires_current_layout(),
            entry.mode.clone(),
            entry.kind.clone(),
            entry.object.clone(),
            entry.path.clone(),
        );
        if state.entries.insert(identity) {
            check_try!(scan_tree_entry(repository, commit, entry, path_policy));
        }
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
    return scan_objects_with_state(repository, objects, path_policy, &mut ScanState::default());
}

/// Scan objects while reusing successful work across all refs in one push.
///
/// # Errors
///
/// Returns an error when any reachable object violates public policy.
pub(super) fn scan_objects_with_state(
    repository: &Path,
    objects: &BTreeSet<String>,
    path_policy: &PathPolicy,
    state: &mut ScanState,
) -> CheckResult {
    for object in objects {
        let identity = (path_policy.clone(), object.clone());
        if state.objects.contains(&identity) {
            continue;
        }
        check_try!(require_object_size(repository, object));
        let kind = check_try!(git::object_kind(repository, object));
        match kind {
            ObjectKind::Blob => check_try!(scan_text_object(repository, object, kind, "Git blob")),
            ObjectKind::Commit => {
                check_try!(scan_text_object(repository, object, kind, "Git commit"));
                check_try!(scan_commit_tree(repository, object, path_policy, state,));
            }
            ObjectKind::Tag => check_try!(scan_text_object(repository, object, kind, "Git tag")),
            ObjectKind::Tree => check_try!(scan_tree_object(repository, object)),
        }
        let inserted = state.objects.insert(identity);
        if !inserted {
            return Err(format!("Git object {object} was cached concurrently"));
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
    match kind {
        ObjectKind::Blob | ObjectKind::Tag => {
            check_try!(validate_tracked_text(label.as_str(), contents.as_slice()));
        }
        ObjectKind::Commit => {
            check_try!(validate_commit_text(label.as_str(), contents.as_slice()));
        }
        ObjectKind::Tree => return Err("a Git tree cannot be scanned as text".to_owned()),
    }
    return reject_forbidden_content(label.as_str(), contents.as_slice());
}

/// Validate one exact public path and its immutable blob contents.
///
/// # Errors
///
/// Returns an error when path, mode, kind, or bytes violate public policy.
fn scan_tree_entry(
    repository: &Path,
    commit: &str,
    entry: &TreeEntry,
    path_policy: &PathPolicy,
) -> CheckResult {
    if entry.kind != "blob" || is_forbidden_tracked_path(entry.path.as_str()) {
        return Err(format!(
            "commit {commit} contains forbidden tree entry {} {}",
            entry.kind, entry.path
        ));
    }
    let require_current_layout = path_policy.requires_current_layout();
    let approved_mode = entry.mode == "100644"
        || (entry.mode == "100755"
            && (!require_current_layout
                || ALLOWED_EXECUTABLE_PATHS.contains(&entry.path.as_str())));
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
        path_policy,
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
    check_try!(reject_private_implementation_terms(
        format!("Git tree {object}").as_str(),
        contents.as_slice()
    ));
    check_try!(reject_secret_signatures(
        format!("Git tree {object}").as_str(),
        contents.as_slice()
    ));
    return tree::validate_raw_tree(repository, object, contents.as_slice());
}

/// Validate canonical whitespace in a raw commit header.
///
/// Git stores each continuation line with one leading space. An empty line in
/// a multiline signature is therefore encoded as exactly one space; that byte
/// is structural rather than trailing message whitespace.
///
/// # Errors
///
/// Returns an error when a header starts with an orphan continuation or when a
/// primary or nonempty continuation line has trailing horizontal whitespace.
fn validate_commit_header(label: &str, header: &str) -> CheckResult {
    let mut has_primary = false;
    for line in header.lines() {
        let continuation = line.starts_with(' ');
        if continuation && !has_primary {
            return Err(format!("{label} starts with an orphan header continuation"));
        }
        let trailing = line
            .as_bytes()
            .last()
            .is_some_and(|byte| return matches!(byte, b' ' | b'\t'));
        if trailing && line != " " {
            return Err(format!("{label} header contains trailing whitespace"));
        }
        has_primary |= !continuation;
    }
    if !has_primary {
        return Err(format!("{label} has no primary header"));
    }
    return Ok(());
}

/// Validate one raw Git commit without misclassifying signature framing.
///
/// # Errors
///
/// Returns an error for binary, non-UTF-8, CRLF, malformed-header, or
/// message-trailing-whitespace content.
fn validate_commit_text(label: &str, contents: &[u8]) -> CheckResult {
    if contents.contains(&0x00) {
        return Err(format!("{label} contains a NUL byte"));
    }
    let source = check_try!(
        from_utf8(contents).map_err(|error| return format!("{label} is not UTF-8: {error}"))
    );
    if source.contains('\r') {
        return Err(format!(
            "{label} contains a carriage return; Git text must use LF"
        ));
    }
    let (header, message) = check_try!(
        source
            .split_once("\n\n")
            .ok_or_else(|| return format!("{label} has no header terminator"))
    );
    check_try!(validate_commit_header(label, header));

    // GitHub's automatically signed `createCommitOnBranch` commits can leave
    // the final message line unterminated. That is legal commit framing, so
    // normalize only for message-whitespace validation. Blobs and tags remain
    // subject to the unchanged terminal-LF requirement.
    let mut canonical_message = message.as_bytes().to_vec();
    if !canonical_message.is_empty() && !canonical_message.ends_with(b"\n") {
        canonical_message.push(b'\n');
    }
    return validate_tracked_text(
        format!("{label} message").as_str(),
        canonical_message.as_slice(),
    );
}

/// Enforce the source-line ceiling and reject Go source additions.
///
/// # Errors
///
/// Returns an error when public source exceeds policy or uses Go.
fn validate_source_policy(path: &str, contents: &[u8], path_policy: &PathPolicy) -> CheckResult {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| return value.to_str())
        .unwrap_or_default();
    if extension.eq_ignore_ascii_case("go") {
        return Err(format!("public surface must not contain Go source: {path}"));
    }
    if !path_policy.requires_current_layout() || !is_guarded_source_path(path) {
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

#[cfg(test)]
#[path = "policy_tests/object_text.rs"]
mod tests;
