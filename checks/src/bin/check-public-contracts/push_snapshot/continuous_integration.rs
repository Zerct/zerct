//! Fail-closed Git object verification for trusted `GitHub` event identities.

use alloc::collections::BTreeSet;

use crate::helpers::{CheckResult, OutputChannel, write_line};

use core::str::from_utf8;

use std::{env, path::Path};

use super::{CiEnvironment, ObjectKind, ci_git, ci_push, git, graph, is_zero_object, policy};

/// Trusted event name supplied directly from the `GitHub` expression context.
const EVENT_NAME_VARIABLE: &str = "TOVUK_CI_EVENT_NAME";

/// Event ref kind supplied directly from the `GitHub` expression context.
const EVENT_REFERENCE_TYPE_VARIABLE: &str = "TOVUK_CI_EVENT_REF_TYPE";

/// Fully qualified event ref supplied directly from the `GitHub` expression context.
const EVENT_REFERENCE_VARIABLE: &str = "TOVUK_CI_EVENT_REF";

/// Event commit supplied directly from the `GitHub` expression context.
const EVENT_SHA_VARIABLE: &str = "TOVUK_CI_EVENT_SHA";

/// Pull request's base tip supplied directly from the `GitHub` expression context.
const PULL_BASE_VARIABLE: &str = "TOVUK_CI_PULL_BASE_SHA";

/// Pull request's head tip supplied directly from the `GitHub` expression context.
const PULL_HEAD_VARIABLE: &str = "TOVUK_CI_PULL_HEAD_SHA";

/// Pull request's generated merge supplied directly from the event payload.
const PULL_MERGE_VARIABLE: &str = "TOVUK_CI_PULL_MERGE_SHA";

/// Pull request number supplied directly from the event payload.
const PULL_NUMBER_VARIABLE: &str = "TOVUK_CI_PULL_NUMBER";

/// Push event's raw new ref object supplied directly from the event payload.
const PUSH_AFTER_VARIABLE: &str = "TOVUK_CI_PUSH_AFTER_SHA";

/// Push event's raw former ref object supplied directly from the event payload.
const PUSH_BEFORE_VARIABLE: &str = "TOVUK_CI_PUSH_BEFORE_SHA";

/// Push event's creation state supplied directly from the event payload.
const PUSH_CREATED_VARIABLE: &str = "TOVUK_CI_PUSH_CREATED";

/// Push event's deletion state supplied directly from the event payload.
const PUSH_DELETED_VARIABLE: &str = "TOVUK_CI_PUSH_DELETED";

/// Push event's forced-update state supplied directly from the event payload.
const PUSH_FORCED_VARIABLE: &str = "TOVUK_CI_PUSH_FORCED";

/// Commit containing the trusted workflow and checker source.
const WORKFLOW_SHA_VARIABLE: &str = "TOVUK_CI_WORKFLOW_SHA";

/// Whether one event field may use Git's all-zero sentinel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ZeroPolicy {
    /// Accept the sentinel used for an absent side of a ref update.
    Allow,
    /// Require an object identity rather than a sentinel.
    Reject,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000c] = [
    size_of_val(&check),
    size_of_val(&check_environment_in),
    size_of_val(&check_pull_request),
    size_of_val(&commit_parents),
    size_of_val(&read_environment_with::<fn(&str) -> CheckResult<String>>),
    size_of_val(&require_commit),
    size_of_val(&require_empty),
    size_of_val(&require_ordered_parents),
    size_of_val(&require_pull_envelope),
    size_of_val(&require_sha),
    size_of_val(&scan_delta),
    size_of_val(&scan_pull_delta),
];

/// Scan the Git object delta described by the trusted `GitHub` event environment.
///
/// # Errors
///
/// Returns an error when an environment value, local object, history boundary,
/// or newly reachable object cannot be verified.
pub(super) fn check() -> CheckResult {
    let mut read_variable = |name: &str| {
        return env::var(name)
            .map_err(|error| format!("read required CI variable {name}: {error}"));
    };
    let environment = check_try!(read_environment_with(&mut read_variable));
    return check_environment_in(Path::new("."), &environment);
}

/// Verify a trusted event against a selected complete repository.
///
/// # Errors
///
/// Returns an error when event metadata or newly reachable objects are invalid.
pub(super) fn check_environment_in(repository: &Path, environment: &CiEnvironment) -> CheckResult {
    check_try!(graph::require_integrity(repository));
    let object_width = check_try!(git::object_id_length(repository));
    check_try!(require_sha(
        environment.workflow_sha.as_str(),
        "workflow",
        object_width,
        ZeroPolicy::Reject,
    ));
    check_try!(require_commit(
        repository,
        environment.workflow_sha.as_str(),
        "workflow",
    ));
    check_try!(git::require_head_object(
        repository,
        environment.event_sha.as_str(),
    ));
    return match environment.event.as_str() {
        "pull_request" => check_pull_request(repository, environment, object_width),
        "push" => ci_push::check(repository, environment, object_width),
        other => Err(format!("unsupported trusted history event {other:?}")),
    };
}

/// Validate and scan one pull request without checking out contributor code.
///
/// # Errors
///
/// Returns an error when the event identities or object delta are invalid.
fn check_pull_request(
    repository: &Path,
    environment: &CiEnvironment,
    object_width: usize,
) -> CheckResult {
    check_try!(require_pull_envelope(repository, environment, object_width));
    check_try!(require_commit(
        repository,
        environment.head.as_str(),
        "pull request head",
    ));
    check_try!(require_commit(
        repository,
        environment.merge.as_str(),
        "pull request merge",
    ));
    check_try!(require_ordered_parents(repository, environment));
    return scan_pull_delta(repository, environment);
}

/// Extract every direct parent from one commit object in its stored order.
///
/// # Errors
///
/// Returns an error when commit bytes or parent identities are malformed.
fn commit_parents(
    repository: &Path,
    commit: &str,
    object_width: usize,
) -> CheckResult<Vec<String>> {
    let contents = check_try!(git::read_object(repository, commit, ObjectKind::Commit));
    let text = check_try!(
        from_utf8(contents.as_slice())
            .map_err(|error| return format!("event commit {commit} is not UTF-8: {error}"))
    );
    let header_end = check_try!(
        text.find("\n\n")
            .ok_or_else(|| return format!("event commit {commit} has no header terminator"))
    );
    let mut parents = Vec::new();
    for parent in text
        .get(..header_end)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| return line.strip_prefix("parent "))
    {
        check_try!(require_sha(
            parent,
            "merge parent",
            object_width,
            ZeroPolicy::Reject,
        ));
        parents.push(parent.to_owned());
    }
    return Ok(parents);
}

/// Read all event fields through an injectable exact-variable reader.
///
/// # Errors
///
/// Returns an error when any required variable is missing or unreadable.
pub(super) fn read_environment_with<ReadVariable>(
    read_variable: &mut ReadVariable,
) -> CheckResult<CiEnvironment>
where
    ReadVariable: FnMut(&str) -> CheckResult<String>,
{
    return Ok(CiEnvironment {
        after: check_try!(read_variable(PUSH_AFTER_VARIABLE)),
        base: check_try!(read_variable(PULL_BASE_VARIABLE)),
        before: check_try!(read_variable(PUSH_BEFORE_VARIABLE)),
        created: check_try!(read_variable(PUSH_CREATED_VARIABLE)),
        deleted: check_try!(read_variable(PUSH_DELETED_VARIABLE)),
        event: check_try!(read_variable(EVENT_NAME_VARIABLE)),
        event_sha: check_try!(read_variable(EVENT_SHA_VARIABLE)),
        forced: check_try!(read_variable(PUSH_FORCED_VARIABLE)),
        head: check_try!(read_variable(PULL_HEAD_VARIABLE)),
        merge: check_try!(read_variable(PULL_MERGE_VARIABLE)),
        number: check_try!(read_variable(PULL_NUMBER_VARIABLE)),
        reference: check_try!(read_variable(EVENT_REFERENCE_VARIABLE)),
        reference_type: check_try!(read_variable(EVENT_REFERENCE_TYPE_VARIABLE)),
        workflow_sha: check_try!(read_variable(WORKFLOW_SHA_VARIABLE)),
    });
}

/// Require one referenced object to exist locally as a commit.
///
/// # Errors
///
/// Returns an error when the object is absent or is not a commit.
pub(super) fn require_commit(repository: &Path, object: &str, label: &str) -> CheckResult {
    if !check_try!(git::object_exists(repository, object)) {
        return Err(format!(
            "{label} object {object} is absent from trusted history"
        ));
    }
    if check_try!(git::object_kind(repository, object)) != ObjectKind::Commit {
        return Err(format!("{label} object {object} must be a commit"));
    }
    return Ok(());
}

/// Require an event field that does not belong to the selected event to be empty.
///
/// # Errors
///
/// Returns an error when the irrelevant field is nonempty.
pub(super) fn require_empty(value: &str, label: &str) -> CheckResult {
    if !value.is_empty() {
        return Err(format!("{label} must be empty"));
    }
    return Ok(());
}

/// Require the generated merge to bind the exact base and contributor head.
///
/// # Errors
///
/// Returns an error when the merge parents are malformed or out of order.
fn require_ordered_parents(repository: &Path, environment: &CiEnvironment) -> CheckResult {
    let object_width = check_try!(git::object_id_length(repository));
    let actual = check_try!(commit_parents(
        repository,
        environment.merge.as_str(),
        object_width,
    ));
    let expected = [environment.base.as_str(), environment.head.as_str()];
    if actual.iter().map(String::as_str).collect::<Vec<_>>() != expected {
        return Err("pull request merge must have ordered parents [base, head]".to_owned());
    }
    return Ok(());
}

/// Validate every field that binds a ruleset-backed pull-request event.
///
/// # Errors
///
/// Returns an error when an irrelevant field, ref, or object identity is invalid.
fn require_pull_envelope(
    repository: &Path,
    environment: &CiEnvironment,
    object_width: usize,
) -> CheckResult {
    for (value, label) in [
        (environment.after.as_str(), "pull request push-after"),
        (environment.before.as_str(), "pull request push-before"),
        (environment.created.as_str(), "pull request push-created"),
        (environment.deleted.as_str(), "pull request push-deleted"),
        (environment.forced.as_str(), "pull request push-forced"),
    ] {
        check_try!(require_empty(value, label));
    }
    let reference_kind = check_try!(ci_git::ReferenceKind::parse(
        environment.reference_type.as_str()
    ));
    let merge_reference = check_try!(ci_git::pull_merge_reference(environment.number.as_str()));
    if reference_kind != ci_git::ReferenceKind::Branch || environment.reference != merge_reference {
        return Err("pull_request must identify its canonical merge ref".to_owned());
    }
    for (value, label) in [
        (environment.base.as_str(), "pull request base"),
        (environment.event_sha.as_str(), "pull request event"),
        (environment.head.as_str(), "pull request head"),
        (environment.merge.as_str(), "pull request merge"),
    ] {
        check_try!(require_sha(value, label, object_width, ZeroPolicy::Reject));
    }
    if environment.event_sha != environment.merge {
        return Err("pull_request event commit must equal its merge commit".to_owned());
    }
    check_try!(require_commit(
        repository,
        environment.base.as_str(),
        "pull request base",
    ));
    return Ok(());
}

/// Require one canonical lowercase SHA-1 or SHA-256 identifier.
///
/// # Errors
///
/// Returns an error when the value has the wrong width, alphabet, or zero policy.
pub(super) fn require_sha(
    value: &str,
    label: &str,
    object_width: usize,
    zero_policy: ZeroPolicy,
) -> CheckResult {
    if !git::valid_object_shape(value, object_width) {
        return Err(format!(
            "{label} must be a {object_width}-digit lowercase hexadecimal Git object ID"
        ));
    }
    if zero_policy == ZeroPolicy::Reject && is_zero_object(value) {
        return Err(format!("{label} must not use Git's all-zero sentinel"));
    }
    return Ok(());
}

/// Scan every object reachable from one target but not the trusted exclusions.
///
/// # Errors
///
/// Returns an error when traversal or public object policy fails.
pub(super) fn scan_delta(
    repository: &Path,
    target: &str,
    excluded: &BTreeSet<String>,
    path_policy: &policy::PathPolicy,
) -> CheckResult {
    let objects = check_try!(git::newly_reachable_objects(repository, target, excluded));
    check_try!(policy::scan_objects(repository, &objects, path_policy));
    return write_line(
        OutputChannel::Regular,
        "Checked every Git object newly reachable through the trusted event.",
    );
}

/// Scan a pull delta and retain both trusted contributor-side identities.
///
/// # Errors
///
/// Returns an error when required objects are absent or public object policy fails.
fn scan_pull_delta(repository: &Path, environment: &CiEnvironment) -> CheckResult {
    let excluded = BTreeSet::from([environment.base.clone()]);
    let objects = check_try!(git::newly_reachable_objects(
        repository,
        environment.merge.as_str(),
        &excluded,
    ));
    if !objects.contains(environment.merge.as_str()) || !objects.contains(environment.head.as_str())
    {
        return Err(
            "pull request merge and head must both be newly reachable from base".to_owned(),
        );
    }
    check_try!(policy::scan_objects(
        repository,
        &objects,
        &policy::PathPolicy::public_surface(
            environment.base.as_str(),
            environment.workflow_sha.as_str(),
        ),
    ));
    return write_line(
        OutputChannel::Regular,
        "Checked every Git object newly reachable through the trusted pull request.",
    );
}
