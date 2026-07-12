//! Trusted remote-ref plumbing for continuous-integration history checks.

use crate::helpers::CheckResult;

use core::str::from_utf8;

use std::path::Path;

use super::{ObjectKind, ci_push::EventFlag, git};

/// Remote created by the pinned trusted checkout action.
const TRUSTED_REMOTE: &str = "origin";

/// Private local ref that binds the exact object returned by one event fetch.
const VERIFIED_FETCH_REFERENCE: &str = "refs/tovuk-history/verified-fetch";

/// Former target, new target, force flag, and ref kind for one update.
type ForcedTransition = (String, String, EventFlag, ReferenceKind);

/// Reviewed kinds of public ref handled by the history gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReferenceKind {
    /// A branch whose raw target must be a commit.
    Branch,
    /// A lightweight or annotated tag that must peel to a commit.
    Tag,
}

impl ReferenceKind {
    /// Parse the exact `github.ref_type` value.
    ///
    /// # Errors
    ///
    /// Returns an error when the event names another ref namespace.
    pub(super) fn parse(value: &str) -> CheckResult<Self> {
        return match value {
            "branch" => Ok(Self::Branch),
            "tag" => Ok(Self::Tag),
            other => Err(format!("trusted event has unsupported ref type {other:?}")),
        };
    }

    /// Return the exact fully qualified prefix for this ref kind.
    const fn prefix(self) -> &'static str {
        return match self {
            Self::Branch => "refs/heads/",
            Self::Tag => "refs/tags/",
        };
    }
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000b] = [
    size_of_val(&advertised_exact_ref),
    size_of_val(&fetch_exact_ref),
    size_of_val(&fetch_object),
    size_of_val(&peel_to_commit),
    size_of_val(&pull_merge_reference),
    size_of_val(&require_event_target),
    size_of_val(&require_forced_state),
    size_of_val(&require_remote_ref_absent),
    size_of_val(&require_reference),
    size_of_val(&run_object_fetch),
    size_of_val(&run_ref_fetch),
];

/// Query one exact ref from the trusted checkout remote.
///
/// # Errors
///
/// Returns an error when the live ref response is malformed or unavailable.
fn advertised_exact_ref(repository: &Path, reference: &str) -> CheckResult<Option<String>> {
    let output = check_try!(git::git_output(
        repository,
        &["ls-remote", "--refs", "--", TRUSTED_REMOTE, reference],
        "git ls-remote trusted ref",
    ));
    let text = check_try!(from_utf8(output.as_slice()).map_err(|error| {
        return format!("trusted ls-remote returned non-UTF-8 output: {error}");
    }));
    if text.is_empty() {
        return Ok(None);
    }
    let mut lines = text.lines();
    let line = check_try!(
        lines
            .next()
            .ok_or_else(|| return "trusted ls-remote omitted its ref".to_owned())
    );
    if lines.next().is_some() {
        return Err("trusted ls-remote returned multiple exact refs".to_owned());
    }
    let (object, actual_reference) = check_try!(
        line.split_once('\t')
            .ok_or_else(|| return "trusted ls-remote returned a malformed ref".to_owned())
    );
    let object_width = check_try!(git::object_id_length(repository));
    if actual_reference != reference || !git::valid_object_shape(object, object_width) {
        return Err("event ls-remote returned an invalid requested ref".to_owned());
    }
    return Ok(Some(object.to_owned()));
}

/// Fetch one exact live ref into a private binding and return its raw object.
///
/// # Errors
///
/// Returns an error when the ref is absent, changes during fetch, or is not
/// privately bound to the fetched raw object.
pub(super) fn fetch_exact_ref(repository: &Path, reference: &str) -> CheckResult<String> {
    let advertised = check_try!(advertised_exact_ref(repository, reference)).ok_or_else(|| {
        return format!("event ref {reference} is absent before fetch");
    });
    let before = check_try!(advertised);
    check_try!(run_ref_fetch(repository, reference));
    let binding = check_try!(git::git_text(
        repository,
        &["show-ref", "--hash", "--verify", VERIFIED_FETCH_REFERENCE],
        "git show-ref verified event fetch",
    ));
    if binding != before {
        return Err(format!(
            "private event fetch for {reference} bound {binding} instead of {before}"
        ));
    }
    let after = check_try!(advertised_exact_ref(repository, reference));
    if after.as_deref() != Some(before.as_str()) {
        return Err(format!(
            "live event ref {reference} changed during verification"
        ));
    }
    if !check_try!(git::object_exists(repository, before.as_str())) {
        return Err(format!("fetched ref {reference} omitted object {before}"));
    }
    return Ok(before);
}

/// Fetch one exact object when a forced update removed its former ref.
///
/// # Errors
///
/// Returns an error when the event's former object cannot be recovered.
pub(super) fn fetch_object(repository: &Path, object: &str) -> CheckResult {
    if !check_try!(git::object_exists(repository, object)) {
        check_try!(run_object_fetch(repository, object));
    }
    if !check_try!(git::object_exists(repository, object)) {
        return Err(format!(
            "former event object {object} is absent after exact fetch"
        ));
    }
    return Ok(());
}

/// Peel a lightweight or annotated tag target to its commit.
///
/// # Errors
///
/// Returns an error when the object cannot peel to exactly one commit.
fn peel_to_commit(repository: &Path, object: &str) -> CheckResult<String> {
    let expression = format!("{object}^{{commit}}");
    return git::git_text(
        repository,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            expression.as_str(),
        ],
        "git rev-parse event target commit",
    );
}

/// Build the only remote ref allowed for a pull request number.
///
/// # Errors
///
/// Returns an error for noncanonical, zero, or overflowing numbers.
pub(super) fn pull_merge_reference(number: &str) -> CheckResult<String> {
    let parsed = check_try!(
        number
            .parse::<u64>()
            .map_err(|error| format!("pull request number is invalid: {error}"))
    );
    if parsed == 0 || parsed.to_string() != number {
        return Err("pull request number must be canonical and positive".to_owned());
    }
    return Ok(format!("refs/pull/{number}/merge"));
}

/// Bind a live branch or tag target to `GitHub`'s event commit identity.
///
/// # Errors
///
/// Returns an error when a branch is not a commit or a tag does not peel to the event commit.
pub(super) fn require_event_target(
    repository: &Path,
    target: &str,
    event_commit: &str,
    kind: ReferenceKind,
) -> CheckResult {
    return match kind {
        ReferenceKind::Branch => {
            if check_try!(git::object_kind(repository, target)) != ObjectKind::Commit
                || target != event_commit
            {
                return Err("pushed branch raw target must equal the event commit".to_owned());
            }
            Ok(())
        }
        ReferenceKind::Tag => {
            let object_kind = check_try!(git::object_kind(repository, target));
            if !matches!(object_kind, ObjectKind::Commit | ObjectKind::Tag) {
                return Err("pushed tag must point or peel to a commit".to_owned());
            }
            let peeled = check_try!(peel_to_commit(repository, target));
            if peeled != event_commit {
                return Err("pushed tag does not peel to the event commit".to_owned());
            }
            Ok(())
        }
    };
}

/// Validate `GitHub`'s forced-update flag against the actual ref transition.
///
/// # Errors
///
/// Returns an error when the declared force state contradicts Git history.
pub(super) fn require_forced_state(repository: &Path, transition: ForcedTransition) -> CheckResult {
    let (before, after, forced, kind) = transition;
    if kind == ReferenceKind::Tag {
        if forced != EventFlag::Set {
            return Err("an existing tag update must be marked forced".to_owned());
        }
        return Ok(());
    }
    let status = check_try!(
        super::git_command(repository)
            .args([
                "merge-base",
                "--is-ancestor",
                before.as_str(),
                after.as_str(),
            ])
            .status()
            .map_err(|error| return format!("run git merge-base --is-ancestor: {error}"))
    );
    let fast_forward = if status.success() {
        true
    } else if status.code() == Some(i32::from(true)) {
        false
    } else {
        return Err(format!("git merge-base --is-ancestor failed with {status}"));
    };
    let coherent = matches!(
        (forced, fast_forward),
        (EventFlag::Clear, true) | (EventFlag::Set, false)
    );
    if !coherent {
        return Err("push forced flag contradicts the branch ancestry transition".to_owned());
    }
    return Ok(());
}

/// Require one canonical fully qualified branch or tag ref.
///
/// # Errors
///
/// Returns an error when the ref uses the wrong namespace or invalid syntax.
pub(super) fn require_reference(
    repository: &Path,
    reference: &str,
    kind: ReferenceKind,
) -> CheckResult {
    if !reference.starts_with(kind.prefix()) {
        return Err(format!(
            "event ref {reference:?} does not match its declared type"
        ));
    }
    drop(check_try!(git::git_output(
        repository,
        &["check-ref-format", reference],
        "git check-ref-format trusted event ref",
    )));
    return Ok(());
}

/// Require a deleted event ref to be absent from the live remote.
///
/// # Errors
///
/// Returns an error when the supposedly deleted ref remains advertised.
pub(super) fn require_remote_ref_absent(repository: &Path, reference: &str) -> CheckResult {
    if check_try!(advertised_exact_ref(repository, reference)).is_some() {
        return Err(format!("deleted event ref {reference} remains live"));
    }
    return Ok(());
}

/// Fetch an exact object without updating the event worktree.
///
/// # Errors
///
/// Returns an error when the event remote cannot provide the object.
fn run_object_fetch(repository: &Path, source: &str) -> CheckResult {
    drop(check_try!(git::git_output(
        repository,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "--no-recurse-submodules",
            "--",
            TRUSTED_REMOTE,
            source,
        ],
        "git fetch event object",
    )));
    return Ok(());
}

/// Fetch one remote ref into the fixed private verification binding.
///
/// # Errors
///
/// Returns an error when the event remote cannot provide and bind the ref.
fn run_ref_fetch(repository: &Path, source: &str) -> CheckResult {
    let refspec = format!("+{source}:{VERIFIED_FETCH_REFERENCE}");
    drop(check_try!(git::git_output(
        repository,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "--no-recurse-submodules",
            "--",
            TRUSTED_REMOTE,
            refspec.as_str(),
        ],
        "git fetch and bind event ref",
    )));
    return Ok(());
}
