//! Validate every Git object made reachable by one push.

mod ci_git;
mod ci_push;
mod continuous_integration;
mod git;
mod graph;
mod policy;
mod policy_paths;
mod tree;

use alloc::collections::BTreeSet;

use crate::helpers::{CheckResult, OutputChannel, write_line};

use std::{
    io::{Read as _, stdin},
    path::Path,
};

/// Exact immutable values that define one supported `GitHub` event.
#[derive(Debug, Eq, PartialEq)]
struct CiEnvironment {
    /// Raw ref object after a push.
    after: String,
    /// Base tip for a pull request event.
    base: String,
    /// Raw ref object before a push.
    before: String,
    /// Whether a push created its ref.
    created: String,
    /// Whether a push deleted its ref.
    deleted: String,
    /// Event kind selected by the workflow trigger.
    event: String,
    /// Commit `GitHub` associates with the event.
    event_sha: String,
    /// Whether a push force-updated its ref.
    forced: String,
    /// Head tip for a pull request event.
    head: String,
    /// Generated merge for a pull request event.
    merge: String,
    /// Pull request number.
    number: String,
    /// Fully qualified event ref.
    reference: String,
    /// Event ref kind.
    reference_type: String,
    /// Commit containing the trusted workflow source.
    workflow_sha: String,
}

/// Git object kinds accepted in a public commit graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObjectKind {
    /// File contents.
    Blob,
    /// Commit metadata and one root tree.
    Commit,
    /// Annotated tag metadata.
    Tag,
    /// Directory entries.
    Tree,
}

/// One record supplied by Git to a pre-push hook.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct PushUpdate {
    /// Local object identifier, or all zeroes for a deletion.
    local_object: String,
    /// Fully qualified local ref, or `(delete)` for a deletion.
    local_reference: String,
    /// Remote object identifier before the push, or all zeroes for a new ref.
    remote_object: String,
    /// Fully qualified destination ref.
    remote_reference: String,
}

/// One recursively listed commit-tree entry.
#[derive(Clone, Debug, Eq, PartialEq)]
struct TreeEntry {
    /// Git object kind reported by `ls-tree`.
    kind: String,
    /// Git tree mode.
    mode: String,
    /// Blob object identifier.
    object: String,
    /// Repository-relative UTF-8 path.
    path: String,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000b] = [
    size_of_val(&check),
    size_of_val(&check_ci),
    size_of_val(&check_input_in),
    size_of_val(&exclusions_for_update),
    size_of_val(&inspect_update),
    size_of_val(&is_zero_object),
    size_of_val(&parse_input),
    size_of_val(&parse_record),
    size_of_val(&path_policy_for_update),
    size_of_val(&require_object_id),
    size_of_val(&validate_record),
];

/// Scan the exact update records received by the pre-push hook.
///
/// # Errors
///
/// Returns an error for malformed input, unverifiable remote state, or any
/// newly reachable object that violates public repository policy.
pub(super) fn check(push_location: &str) -> CheckResult {
    let mut input = String::new();
    let byte_count = check_try!(
        stdin()
            .read_to_string(&mut input)
            .map_err(|error| return format!("read pre-push input: {error}"))
    );
    if byte_count != input.len() {
        return Err("pre-push input byte count changed while reading".to_owned());
    }
    return check_input_in(Path::new("."), push_location, input.as_str());
}

/// Scan the trusted `GitHub` event's newly reachable Git object delta.
///
/// # Errors
///
/// Returns an error when CI metadata, history, or reachable objects are invalid.
pub(super) fn check_ci() -> CheckResult {
    return continuous_integration::check();
}

/// Scan pre-push records in a selected repository.
///
/// # Errors
///
/// Returns an error when the input or a reachable object violates policy.
fn check_input_in(repository: &Path, push_location: &str, input: &str) -> CheckResult {
    if push_location.is_empty() || push_location.contains(['\0', '\n', '\r']) {
        return Err("push-snapshot push location is malformed".to_owned());
    }
    check_try!(graph::require_integrity(repository));
    let object_id_length = check_try!(git::object_id_length(repository));
    let updates = check_try!(parse_input(input, object_id_length));
    for update in &updates {
        check_try!(inspect_update(repository, push_location, update));
    }
    check_try!(write_line(
        OutputChannel::Regular,
        "Checked every object reachable through the proposed push.",
    ));
    return Ok(());
}

/// Select the exact remote graph excluded from one update scan.
///
/// # Errors
///
/// Returns an error when an exact base is unavailable or remote refs are malformed.
fn exclusions_for_update(
    repository: &Path,
    push_location: &str,
    actual_remote: Option<&str>,
) -> CheckResult<BTreeSet<String>> {
    let Some(object) = actual_remote else {
        return git::advertised_remote_objects(repository, push_location);
    };
    if !check_try!(git::object_exists(repository, object)) {
        return Err(format!(
            "live remote base {object} is absent locally; fetch the destination ref before pushing"
        ));
    }
    return Ok(BTreeSet::from([object.to_owned()]));
}

/// Inspect one destination ref against its live remote object.
///
/// # Errors
///
/// Returns an error when the ref state or newly reachable objects are invalid.
fn inspect_update(repository: &Path, push_location: &str, update: &PushUpdate) -> CheckResult {
    check_try!(git::require_valid_refs(repository, update));
    let actual_remote = check_try!(git::advertised_exact_remote_object(
        repository,
        push_location,
        update,
    ));
    check_try!(git::require_advertised_remote(
        update,
        actual_remote.as_deref()
    ));
    if is_zero_object(update.local_object.as_str()) {
        return Ok(());
    }
    check_try!(git::require_local_object(repository, update));
    check_try!(git::require_commit_target(repository, update));
    let excluded = check_try!(exclusions_for_update(
        repository,
        push_location,
        actual_remote.as_deref(),
    ));
    let path_policy = check_try!(path_policy_for_update(
        repository,
        update,
        actual_remote.as_deref(),
    ));
    let objects = check_try!(git::newly_reachable_objects(
        repository,
        update.local_object.as_str(),
        &excluded,
    ));
    return policy::scan_objects(repository, &objects, &path_policy);
}

/// Return whether an object field represents Git's all-zero sentinel.
fn is_zero_object(object: &str) -> bool {
    return object.bytes().all(|byte| return byte == b'0');
}

/// Parse all complete pre-push lines and reject duplicate destinations.
///
/// # Errors
///
/// Returns an error when any record is malformed or repeats a destination.
fn parse_input(input: &str, object_id_length: usize) -> CheckResult<Vec<PushUpdate>> {
    if input.contains('\r') || (!input.is_empty() && !input.ends_with('\n')) {
        return Err("pre-push input must contain complete LF-terminated records".to_owned());
    }
    let mut destinations = BTreeSet::new();
    let mut updates = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let line_number = index.saturating_add(0x0001);
        let update = check_try!(parse_record(line, line_number, object_id_length));
        if !destinations.insert(update.remote_reference.clone()) {
            return Err(format!(
                "pre-push input repeats destination {}",
                update.remote_reference
            ));
        }
        updates.push(update);
    }
    return Ok(updates);
}

/// Parse one four-field pre-push record.
///
/// # Errors
///
/// Returns an error when the record does not have four coherent fields.
fn parse_record(
    line: &str,
    line_number: usize,
    object_id_length: usize,
) -> CheckResult<PushUpdate> {
    let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
    let [
        local_reference,
        local_object,
        remote_reference,
        remote_object,
    ] = *fields.as_slice()
    else {
        return Err(format!(
            "pre-push line {line_number} must contain exactly four fields"
        ));
    };
    let canonical = format!("{local_reference} {local_object} {remote_reference} {remote_object}");
    if line != canonical {
        return Err(format!(
            "pre-push line {line_number} must use canonical single-space separators"
        ));
    }
    check_try!(require_object_id(local_object, object_id_length, "local"));
    check_try!(require_object_id(remote_object, object_id_length, "remote"));
    let update = PushUpdate {
        local_object: local_object.to_owned(),
        local_reference: local_reference.to_owned(),
        remote_object: remote_object.to_owned(),
        remote_reference: remote_reference.to_owned(),
    };
    check_try!(validate_record(&update));
    return Ok(update);
}

/// Select current or fully scanned historical policy for one local update.
///
/// # Errors
///
/// Returns an error when ancestry or a rewritten branch tip cannot be verified.
fn path_policy_for_update(
    repository: &Path,
    update: &PushUpdate,
    actual_remote: Option<&str>,
) -> CheckResult<policy::PathPolicy> {
    let Some(base) = actual_remote else {
        return Ok(policy::PathPolicy::current());
    };
    if !update.remote_reference.starts_with("refs/heads/") {
        return Ok(policy::PathPolicy::historical());
    }
    let fast_forward = check_try!(graph::is_ancestor(
        repository,
        base,
        update.local_object.as_str(),
    ));
    let current_base = check_try!(policy_paths::commit_has_manifest(repository, base));
    if fast_forward && current_base {
        return Ok(policy::PathPolicy::public_surface(
            base,
            update.local_object.as_str(),
        ));
    }
    check_try!(policy_paths::require_current_tip(
        repository,
        update.local_object.as_str(),
    ));
    return Ok(policy::PathPolicy::historical());
}

/// Require a lowercase hexadecimal object identifier of the repository's width.
///
/// # Errors
///
/// Returns an error when the object identifier has the wrong width or alphabet.
fn require_object_id(object: &str, expected_length: usize, label: &str) -> CheckResult {
    let valid_hex = object
        .bytes()
        .all(|byte| return byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
    if object.len() != expected_length || !valid_hex {
        return Err(format!(
            "pre-push {label} object must be a {expected_length}-digit lowercase hexadecimal ID"
        ));
    }
    return Ok(());
}

/// Require coherent creation, update, or deletion sentinels.
///
/// # Errors
///
/// Returns an error when deletion names and all-zero objects disagree.
fn validate_record(update: &PushUpdate) -> CheckResult {
    let deleting = is_zero_object(update.local_object.as_str());
    let local_deletion_name = update.local_reference == "(delete)";
    if deleting != local_deletion_name {
        return Err("only an all-zero local object may use the `(delete)` ref".to_owned());
    }
    if deleting && is_zero_object(update.remote_object.as_str()) {
        return Err("cannot delete a remote ref whose old object is all zeroes".to_owned());
    }
    return Ok(());
}

#[cfg(test)]
#[path = "test_fixture.rs"]
mod tests;
