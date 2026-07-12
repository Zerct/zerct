//! Trusted branch and tag push validation.

use alloc::collections::BTreeSet;

use crate::helpers::{CheckResult, OutputChannel, write_line};

use std::path::Path;

use super::{
    CiEnvironment, ci_git,
    continuous_integration::{ZeroPolicy, require_commit, require_empty, require_sha, scan_delta},
    is_zero_object, policy, policy_paths,
};

/// Independently observed default branch used as the only creation exclusion.
const MAIN_REFERENCE: &str = "refs/heads/main";

/// Independently verified exclusion and path policy for one created ref.
#[derive(Debug, Eq, PartialEq)]
struct CreatedBoundary {
    /// Existing live-main graph excluded from creation scanning.
    exclusions: BTreeSet<String>,
    /// Current policy bound to live main when it exists independently.
    path_policy: policy::PathPolicy,
}

/// Exact state of one boolean field from a trusted event payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EventFlag {
    /// The event field was exactly `false`.
    Clear,
    /// The event field was exactly `true`.
    Set,
}

/// Parsed push event state needed after envelope validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PushState {
    /// Whether the ref was created.
    created: EventFlag,
    /// Whether the ref was deleted.
    deleted: EventFlag,
    /// Whether the update was forced.
    forced: EventFlag,
    /// Branch or tag kind declared by `github.ref_type`.
    reference_kind: ci_git::ReferenceKind,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&check),
    size_of_val(&created_ref_boundary),
    size_of_val(&event_flag),
    size_of_val(&require_push_envelope),
    size_of_val(&require_push_objects),
    size_of_val(&scan_live_push),
    size_of_val(&scan_updated_ref),
    size_of_val(&updated_path_policy),
    size_of_val(&validate_push_state),
];

/// Validate and scan one branch or tag push event.
///
/// # Errors
///
/// Returns an error when event state, live ref state, or object policy fails.
pub(super) fn check(
    repository: &Path,
    environment: &CiEnvironment,
    object_width: usize,
) -> CheckResult {
    let state = check_try!(require_push_envelope(repository, environment, object_width));
    if state.deleted == EventFlag::Set {
        check_try!(ci_git::require_remote_ref_absent(
            repository,
            environment.reference.as_str(),
        ));
        return write_line(
            OutputChannel::Regular,
            "Verified that the deleted ref makes no new Git object reachable.",
        );
    }
    return scan_live_push(repository, environment, state);
}

/// Return the one independently verified live-main exclusion for a new ref.
///
/// # Errors
///
/// Returns an error when live main changes during fetch or is not a commit.
fn created_ref_boundary(
    repository: &Path,
    environment: &CiEnvironment,
) -> CheckResult<CreatedBoundary> {
    if environment.reference == MAIN_REFERENCE {
        return Ok(CreatedBoundary {
            exclusions: BTreeSet::new(),
            path_policy: policy::PathPolicy::current(),
        });
    }
    let main = check_try!(ci_git::fetch_exact_ref(repository, MAIN_REFERENCE));
    check_try!(ci_git::require_event_target(
        repository,
        main.as_str(),
        main.as_str(),
        ci_git::ReferenceKind::Branch,
    ));
    return Ok(CreatedBoundary {
        exclusions: BTreeSet::from([main.clone()]),
        path_policy: policy::PathPolicy::public_surface(
            main.as_str(),
            environment.workflow_sha.as_str(),
        ),
    });
}

/// Parse one exact lower-case boolean from the event payload.
///
/// # Errors
///
/// Returns an error when the value is not exactly `true` or `false`.
fn event_flag(value: &str, label: &str) -> CheckResult<EventFlag> {
    return match value {
        "false" => Ok(EventFlag::Clear),
        "true" => Ok(EventFlag::Set),
        other => Err(format!(
            "{label} must be exactly true or false, not {other:?}"
        )),
    };
}

/// Validate every field that binds a push event.
///
/// # Errors
///
/// Returns an error when an irrelevant field, ref, flag, or identity is invalid.
fn require_push_envelope(
    repository: &Path,
    environment: &CiEnvironment,
    object_width: usize,
) -> CheckResult<PushState> {
    for (value, label) in [
        (environment.base.as_str(), "push pull-base"),
        (environment.head.as_str(), "push pull-head"),
        (environment.merge.as_str(), "push pull-merge"),
        (environment.number.as_str(), "push pull-number"),
    ] {
        check_try!(require_empty(value, label));
    }
    let state = PushState {
        created: check_try!(event_flag(environment.created.as_str(), "push created")),
        deleted: check_try!(event_flag(environment.deleted.as_str(), "push deleted")),
        forced: check_try!(event_flag(environment.forced.as_str(), "push forced")),
        reference_kind: check_try!(ci_git::ReferenceKind::parse(
            environment.reference_type.as_str()
        )),
    };
    check_try!(require_push_objects(repository, environment, object_width));
    check_try!(ci_git::require_reference(
        repository,
        environment.reference.as_str(),
        state.reference_kind,
    ));
    check_try!(validate_push_state(environment, state));
    return Ok(state);
}

/// Validate the raw object identities carried by a push event.
///
/// # Errors
///
/// Returns an error when an identity has the wrong shape or event kind.
fn require_push_objects(
    repository: &Path,
    environment: &CiEnvironment,
    object_width: usize,
) -> CheckResult {
    check_try!(require_sha(
        environment.after.as_str(),
        "push after",
        object_width,
        ZeroPolicy::Allow,
    ));
    check_try!(require_sha(
        environment.before.as_str(),
        "push before",
        object_width,
        ZeroPolicy::Allow,
    ));
    check_try!(require_sha(
        environment.event_sha.as_str(),
        "push event",
        object_width,
        ZeroPolicy::Reject,
    ));
    check_try!(require_commit(
        repository,
        environment.event_sha.as_str(),
        "push event",
    ));
    return Ok(());
}

/// Fetch and scan the exact live non-deleted event ref.
///
/// # Errors
///
/// Returns an error when the live target or its event boundary is invalid.
fn scan_live_push(repository: &Path, environment: &CiEnvironment, state: PushState) -> CheckResult {
    if environment.after != environment.event_sha {
        return Err("non-deleted push after must equal the event commit".to_owned());
    }
    let raw_target = check_try!(ci_git::fetch_exact_ref(
        repository,
        environment.reference.as_str()
    ));
    check_try!(ci_git::require_event_target(
        repository,
        raw_target.as_str(),
        environment.after.as_str(),
        state.reference_kind,
    ));
    if state.created == EventFlag::Set {
        let boundary = check_try!(created_ref_boundary(repository, environment));
        return scan_delta(
            repository,
            raw_target.as_str(),
            &boundary.exclusions,
            &boundary.path_policy,
        );
    }
    return scan_updated_ref(repository, environment, raw_target.as_str(), state);
}

/// Fetch the former ref object and scan one update relative to it.
///
/// # Errors
///
/// Returns an error when the former object or declared force state is invalid.
fn scan_updated_ref(
    repository: &Path,
    environment: &CiEnvironment,
    raw_target: &str,
    state: PushState,
) -> CheckResult {
    check_try!(ci_git::fetch_object(
        repository,
        environment.before.as_str()
    ));
    check_try!(ci_git::require_forced_state(
        repository,
        (
            environment.before.clone(),
            environment.after.clone(),
            state.forced,
            state.reference_kind,
        ),
    ));
    let path_policy = check_try!(updated_path_policy(repository, environment, state));
    return scan_delta(
        repository,
        raw_target,
        &BTreeSet::from([environment.before.clone()]),
        &path_policy,
    );
}

/// Select current or rewrite-compatible policy for one verified update.
///
/// # Errors
///
/// Returns an error when a current branch tip lacks an exact tree manifest.
fn updated_path_policy(
    repository: &Path,
    environment: &CiEnvironment,
    state: PushState,
) -> CheckResult<policy::PathPolicy> {
    return Ok(match (state.reference_kind, state.forced) {
        (ci_git::ReferenceKind::Branch, EventFlag::Clear) => {
            if check_try!(policy_paths::commit_has_manifest(
                repository,
                environment.before.as_str(),
            )) {
                policy::PathPolicy::public_surface(
                    environment.before.as_str(),
                    environment.workflow_sha.as_str(),
                )
            } else {
                check_try!(policy_paths::require_current_tip(
                    repository,
                    environment.after.as_str(),
                ));
                policy::PathPolicy::historical()
            }
        }
        (ci_git::ReferenceKind::Branch, EventFlag::Set) => {
            check_try!(policy_paths::require_current_tip(
                repository,
                environment.after.as_str(),
            ));
            policy::PathPolicy::historical()
        }
        (ci_git::ReferenceKind::Tag, EventFlag::Clear | EventFlag::Set) => {
            policy::PathPolicy::historical()
        }
    });
}

/// Require exact coherence among creation, deletion, force, and object sentinels.
///
/// # Errors
///
/// Returns an error when flags contradict raw before and after identities.
fn validate_push_state(environment: &CiEnvironment, state: PushState) -> CheckResult {
    let before_zero = is_zero_object(environment.before.as_str());
    let after_zero = is_zero_object(environment.after.as_str());
    if state.created == EventFlag::Set {
        if state.deleted == EventFlag::Set
            || state.forced == EventFlag::Set
            || !before_zero
            || after_zero
        {
            return Err("created push must be non-forced zero-to-object ref creation".to_owned());
        }
    } else if state.deleted == EventFlag::Set {
        if state.forced == EventFlag::Set || before_zero || !after_zero {
            return Err("deleted push must be non-forced object-to-zero ref deletion".to_owned());
        }
    } else if before_zero || after_zero {
        return Err("updated push must contain nonzero before and after objects".to_owned());
    } else {
        return Ok(());
    }
    return Ok(());
}
