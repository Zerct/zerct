//! Fail-closed Git plumbing for pushed-object verification.

use alloc::collections::BTreeSet;

use crate::helpers::CheckResult;

use core::{iter::once, str::from_utf8};

use std::path::Path;

use super::{ObjectKind, PushUpdate, TreeEntry, is_zero_object};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0015] = [
    size_of_val(&advertised_exact_remote_object),
    size_of_val(&advertised_remote_objects),
    size_of_val(&git_output),
    size_of_val(&git_text),
    size_of_val(&newly_reachable_objects),
    size_of_val(&object_exists),
    size_of_val(&object_id_length),
    size_of_val(&object_kind),
    size_of_val(&object_size),
    size_of_val(&object_type_name),
    size_of_val(&parse_remote_object),
    size_of_val(&parse_tree_entry),
    size_of_val(&read_object),
    size_of_val(&require_advertised_remote),
    size_of_val(&require_commit_target),
    size_of_val(&require_complete_history),
    size_of_val(&require_head_object),
    size_of_val(&require_local_object),
    size_of_val(&require_valid_refs),
    size_of_val(&tree_entries),
    size_of_val(&valid_object_shape),
];

/// Query the destination ref directly instead of trusting a tracking ref.
///
/// # Errors
///
/// Returns an error when the live remote cannot provide one canonical exact ref.
pub(super) fn advertised_exact_remote_object(
    repository: &Path,
    push_location: &str,
    update: &PushUpdate,
) -> CheckResult<Option<String>> {
    let output = check_try!(git_output(
        repository,
        &[
            "ls-remote",
            "--refs",
            "--",
            push_location,
            update.remote_reference.as_str(),
        ],
        "git ls-remote",
    ));
    let text = check_try!(
        from_utf8(output.as_slice())
            .map_err(|error| return format!("git ls-remote returned non-UTF-8 output: {error}"))
    );
    return parse_remote_object(text, update);
}

/// Query every object currently advertised by the live destination remote.
///
/// # Errors
///
/// Returns an error when the live remote advertises malformed ref records.
pub(super) fn advertised_remote_objects(
    repository: &Path,
    push_location: &str,
) -> CheckResult<BTreeSet<String>> {
    let output = check_try!(git_output(
        repository,
        &["ls-remote", "--refs", "--", push_location],
        "git ls-remote all refs",
    ));
    let text = check_try!(from_utf8(output.as_slice()).map_err(|error| {
        return format!("git ls-remote all refs returned non-UTF-8 output: {error}");
    }));
    let expected_length = check_try!(object_id_length(repository));
    let mut objects = BTreeSet::new();
    for line in text.lines() {
        let (object, reference) = check_try!(
            line.split_once('\t')
                .ok_or_else(|| return "git ls-remote returned a malformed ref".to_owned())
        );
        if !reference.starts_with("refs/") || !valid_object_shape(object, expected_length) {
            return Err("git ls-remote returned an invalid advertised ref".to_owned());
        }
        if check_try!(object_exists(repository, object)) {
            objects.extend(once(object.to_owned()));
        }
    }
    return Ok(objects);
}

/// Run Git and return standard output bytes.
///
/// # Errors
///
/// Returns an error when Git cannot start or exits unsuccessfully.
pub(super) fn git_output(repository: &Path, args: &[&str], label: &str) -> CheckResult<Vec<u8>> {
    let output = check_try!(
        super::git_command(repository)
            .args(args)
            .output()
            .map_err(|error| return format!("run {label}: {error}"))
    );
    if !output.status.success() {
        return Err(format!("{label} failed with status {}", output.status));
    }
    return Ok(output.stdout);
}

/// Run Git and return one trimmed UTF-8 value.
///
/// # Errors
///
/// Returns an error when Git fails or emits non-UTF-8 output.
pub(super) fn git_text(repository: &Path, args: &[&str], label: &str) -> CheckResult<String> {
    let output = check_try!(git_output(repository, args, label));
    let text = check_try!(
        from_utf8(output.as_slice())
            .map_err(|error| return format!("{label} returned non-UTF-8 output: {error}"))
    );
    return Ok(text.trim_end_matches('\n').to_owned());
}

/// Enumerate every object reachable from the new target but not the live old target.
///
/// # Errors
///
/// Returns an error when Git cannot traverse the proposed object graph.
pub(super) fn newly_reachable_objects(
    repository: &Path,
    new_object: &str,
    excluded_objects: &BTreeSet<String>,
) -> CheckResult<BTreeSet<String>> {
    let mut args = vec!["rev-list", "--objects", "--no-object-names", new_object];
    if !excluded_objects.is_empty() {
        args.push("--not");
        args.extend(excluded_objects.iter().map(String::as_str));
    }
    let text = check_try!(git_text(repository, args.as_slice(), "git rev-list"));
    let objects = text.lines().map(str::to_owned).collect::<BTreeSet<_>>();
    return Ok(objects);
}

/// Return whether one remote-advertised object exists in the local database.
///
/// # Errors
///
/// Returns an error when Git cannot perform the local object query.
pub(super) fn object_exists(repository: &Path, object: &str) -> CheckResult<bool> {
    let peeled = format!("{object}^{{object}}");
    let output = check_try!(
        super::git_command(repository)
            .args(["cat-file", "-e", peeled.as_str()])
            .output()
            .map_err(|error| return format!("run git cat-file -e: {error}"))
    );
    return Ok(output.status.success());
}

/// Return the object-ID width for this repository's Git object format.
///
/// # Errors
///
/// Returns an error when Git reports an unsupported object format.
pub(super) fn object_id_length(repository: &Path) -> CheckResult<usize> {
    let format = check_try!(git_text(
        repository,
        &["rev-parse", "--show-object-format"],
        "git rev-parse --show-object-format",
    ));
    return match format.as_str() {
        "sha1" => Ok(0x0028),
        "sha256" => Ok(0x0040),
        other => Err(format!("unsupported Git object format {other:?}")),
    };
}

/// Return the verified kind of one Git object.
///
/// # Errors
///
/// Returns an error when the object is absent or has an unknown kind.
pub(super) fn object_kind(repository: &Path, object: &str) -> CheckResult<ObjectKind> {
    let kind = check_try!(git_text(
        repository,
        &["cat-file", "-t", object],
        "git cat-file -t",
    ));
    return match kind.as_str() {
        "blob" => Ok(ObjectKind::Blob),
        "commit" => Ok(ObjectKind::Commit),
        "tag" => Ok(ObjectKind::Tag),
        "tree" => Ok(ObjectKind::Tree),
        other => Err(format!("pushed object {object} has unknown kind {other:?}")),
    };
}

/// Return the byte length Git reports for one object without reading its body.
///
/// # Errors
///
/// Returns an error when Git cannot inspect the object size.
pub(super) fn object_size(repository: &Path, object: &str) -> CheckResult<u64> {
    let size = check_try!(git_text(
        repository,
        &["cat-file", "-s", object],
        "git cat-file -s",
    ));
    return size
        .parse::<u64>()
        .map_err(|error| return format!("Git object {object} has invalid size {size:?}: {error}"));
}

/// Map a checked object kind to the raw `cat-file` type selector.
const fn object_type_name(kind: ObjectKind) -> &'static str {
    return match kind {
        ObjectKind::Blob => "blob",
        ObjectKind::Commit => "commit",
        ObjectKind::Tag => "tag",
        ObjectKind::Tree => "tree",
    };
}

/// Parse one exact remote ref result.
///
/// # Errors
///
/// Returns an error when the remote record is ambiguous or malformed.
fn parse_remote_object(text: &str, update: &PushUpdate) -> CheckResult<Option<String>> {
    if text.is_empty() {
        return Ok(None);
    }
    let mut lines = text.lines();
    let line = check_try!(
        lines
            .next()
            .ok_or_else(|| return "missing remote ref".to_owned())
    );
    if lines.next().is_some() {
        return Err("git ls-remote returned more than one exact ref".to_owned());
    }
    let (object, reference) = check_try!(
        line.split_once('\t')
            .ok_or_else(|| return "git ls-remote returned a malformed ref".to_owned())
    );
    if reference != update.remote_reference
        || !valid_object_shape(object, update.remote_object.len())
    {
        return Err("git ls-remote returned an invalid exact ref".to_owned());
    }
    return Ok(Some(object.to_owned()));
}

/// Parse one NUL-delimited recursive `ls-tree` entry.
///
/// # Errors
///
/// Returns an error when tree metadata or path bytes are malformed.
fn parse_tree_entry(entry: &[u8], commit: &str) -> CheckResult<TreeEntry> {
    let separator = check_try!(
        entry
            .iter()
            .position(|byte| return *byte == b'\t')
            .ok_or_else(|| return format!("commit {commit} has malformed tree metadata"))
    );
    let metadata = check_try!(
        from_utf8(entry.get(..separator).unwrap_or_default())
            .map_err(|error| return format!("commit {commit} has non-UTF-8 metadata: {error}"))
    );
    let path_bytes = entry
        .get(separator.saturating_add(0x0001)..)
        .unwrap_or_default();
    let path = check_try!(
        from_utf8(path_bytes)
            .map_err(|error| return format!("commit {commit} has non-UTF-8 path: {error}"))
    );
    let fields = metadata.split_ascii_whitespace().collect::<Vec<_>>();
    let [mode, kind, object] = *fields.as_slice() else {
        return Err(format!("commit {commit} has malformed tree entry {path:?}"));
    };
    if !valid_object_shape(object, commit.len()) {
        return Err(format!(
            "commit {commit} has invalid object ID for {path:?}"
        ));
    }
    return Ok(TreeEntry {
        kind: kind.to_owned(),
        mode: mode.to_owned(),
        object: object.to_owned(),
        path: path.to_owned(),
    });
}

/// Read one object exactly as stored by Git.
///
/// # Errors
///
/// Returns an error when Git cannot read the object as the verified kind.
pub(super) fn read_object(
    repository: &Path,
    object: &str,
    kind: ObjectKind,
) -> CheckResult<Vec<u8>> {
    return git_output(
        repository,
        &["cat-file", object_type_name(kind), object],
        "git cat-file object",
    );
}

/// Require the hook's old object to equal the destination's live old object.
///
/// # Errors
///
/// Returns an error when the advertised and live destination objects differ.
pub(super) fn require_advertised_remote(update: &PushUpdate, actual: Option<&str>) -> CheckResult {
    return match actual {
        Some(object) if object == update.remote_object => Ok(()),
        Some(object) => Err(format!(
            "remote ref {} changed from advertised {} to {object}",
            update.remote_reference, update.remote_object
        )),
        None if is_zero_object(update.remote_object.as_str()) => Ok(()),
        None => Err(format!(
            "remote ref {} no longer exists at advertised object {}",
            update.remote_reference, update.remote_object
        )),
    };
}

/// Require branches to be commits and tags to peel to commits.
///
/// # Errors
///
/// Returns an error when a destination would point to a noncommit target.
pub(super) fn require_commit_target(repository: &Path, update: &PushUpdate) -> CheckResult {
    let kind = check_try!(object_kind(repository, update.local_object.as_str()));
    if update.remote_reference.starts_with("refs/heads/") {
        return (kind == ObjectKind::Commit)
            .then_some(())
            .ok_or_else(|| return "a pushed branch target must be a commit".to_owned());
    }
    if !matches!(kind, ObjectKind::Commit | ObjectKind::Tag) {
        return Err("a pushed tag must point or peel to a commit".to_owned());
    }
    let peeled = format!("{}^{{commit}}", update.local_object);
    drop(check_try!(git_text(
        repository,
        &["rev-parse", "--verify", "--end-of-options", peeled.as_str()],
        "git rev-parse tag commit",
    )));
    return Ok(());
}

/// Require an unshallow repository before computing an event history delta.
///
/// # Errors
///
/// Returns an error when Git cannot prove that the local history is complete.
pub(super) fn require_complete_history(repository: &Path) -> CheckResult {
    let state = check_try!(git_text(
        repository,
        &["rev-parse", "--is-shallow-repository"],
        "git rev-parse --is-shallow-repository",
    ));
    return match state.as_str() {
        "false" => Ok(()),
        "true" => Err("CI snapshot requires a complete non-shallow Git history".to_owned()),
        other => Err(format!("Git returned invalid shallow state {other:?}")),
    };
}

/// Require the checked-out `HEAD` to equal the trusted event commit.
///
/// # Errors
///
/// Returns an error when `HEAD` is absent or differs from the event identity.
pub(super) fn require_head_object(repository: &Path, expected: &str) -> CheckResult {
    let head = check_try!(git_text(
        repository,
        &["rev-parse", "--verify", "HEAD"],
        "git rev-parse --verify HEAD",
    ));
    if head != expected {
        return Err(format!(
            "checked-out HEAD {head} differs from trusted event commit {expected}"
        ));
    }
    return Ok(());
}

/// Require the local ref to resolve to the exact object supplied by Git.
///
/// # Errors
///
/// Returns an error when the local ref is absent or resolves differently.
pub(super) fn require_local_object(repository: &Path, update: &PushUpdate) -> CheckResult {
    let object = check_try!(git_text(
        repository,
        &[
            "show-ref",
            "--verify",
            "--hash",
            update.local_reference.as_str(),
        ],
        "git show-ref local ref",
    ));
    if object != update.local_object {
        return Err(format!(
            "local ref {} does not resolve to proposed object {}",
            update.local_reference, update.local_object
        ));
    }
    return Ok(());
}

/// Require local and destination refs to use reviewed public namespaces.
///
/// # Errors
///
/// Returns an error when either ref is malformed or uses an unknown namespace.
pub(super) fn require_valid_refs(repository: &Path, update: &PushUpdate) -> CheckResult {
    let remote_known = update.remote_reference.starts_with("refs/heads/")
        || update.remote_reference.starts_with("refs/tags/");
    let local_known = update.local_reference == "(delete)"
        || update.local_reference.starts_with("refs/heads/")
        || update.local_reference.starts_with("refs/tags/");
    if !remote_known || !local_known {
        return Err("pre-push refs must use refs/heads/* or refs/tags/*".to_owned());
    }
    drop(check_try!(git_output(
        repository,
        &["check-ref-format", update.remote_reference.as_str()],
        "git check-ref-format remote ref",
    )));
    if update.local_reference != "(delete)" {
        drop(check_try!(git_output(
            repository,
            &["check-ref-format", update.local_reference.as_str()],
            "git check-ref-format local ref",
        )));
    }
    return Ok(());
}

/// Recursively list every path in one commit tree without quoting names.
///
/// # Errors
///
/// Returns an error when Git cannot list or parse the commit tree.
pub(super) fn tree_entries(repository: &Path, commit: &str) -> CheckResult<Vec<TreeEntry>> {
    let output = check_try!(git_output(
        repository,
        &["ls-tree", "-r", "-z", "--full-tree", commit],
        "git ls-tree",
    ));
    let mut entries = Vec::new();
    for raw_entry in output.split(|byte| return *byte == 0x00) {
        if raw_entry.is_empty() {
            continue;
        }
        entries.push(check_try!(parse_tree_entry(raw_entry, commit)));
    }
    return Ok(entries);
}

/// Return whether Git output has one canonical lowercase object-ID shape.
pub(super) fn valid_object_shape(object: &str, expected_length: usize) -> bool {
    return object.len() == expected_length
        && object
            .bytes()
            .all(|byte| return byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
}
