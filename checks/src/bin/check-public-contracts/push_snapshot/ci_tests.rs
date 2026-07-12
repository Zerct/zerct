//! `GitHub` event fixtures for the Rust-native trusted history scanner.

use crate::helpers::CheckResult;

use super::{PushFixture, append_safe_readme, create_fixture, git_text, remove_fixture, run_git};

use super::super::{
    CiEnvironment,
    continuous_integration::{check_environment_in, read_environment_with},
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0008] = [
    size_of_val(&build_pull_merge),
    size_of_val(&construct_environment),
    size_of_val(&ensure_ci_rejected),
    size_of_val(&merge_topic),
    size_of_val(&publish_ref),
    size_of_val(&safe_pull_environment),
    size_of_val(&safe_pull_environment_with_workflow),
    size_of_val(&safe_push_environment),
];

/// Create and publish the generated merge commit for one pull request fixture.
///
/// # Errors
///
/// Returns an error when branch setup, merge creation, publication, or checkout fails.
pub(super) fn build_pull_merge(
    fixture: &PushFixture,
    head: &str,
    message: &str,
) -> CheckResult<String> {
    check_try!(run_git(
        fixture.repository.as_path(),
        &["update-ref", "refs/heads/topic", head],
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &[
            "switch",
            "--quiet",
            "--create",
            "pull-checkout",
            fixture.baseline.as_str(),
        ],
    ));
    check_try!(merge_topic(fixture, message));
    let merge = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    check_try!(publish_ref(fixture, merge.as_str(), "refs/pull/1/merge"));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["switch", "--quiet", "--detach", merge.as_str()],
    ));
    return Ok(merge);
}

/// Build one event envelope through the same exact-variable reader as production.
///
/// # Errors
///
/// Returns an error when an expected variable cannot be mapped.
pub(super) fn construct_environment(values: &[&str; 0x000e]) -> CheckResult<CiEnvironment> {
    let [
        event,
        event_sha,
        workflow_sha,
        reference,
        reference_type,
        after,
        before,
        created,
        deleted,
        forced,
        base,
        head,
        merge,
        number,
    ] = *values;
    let mut reader = |name: &str| -> CheckResult<String> {
        let value = match name {
            "TOVUK_CI_EVENT_NAME" => event,
            "TOVUK_CI_EVENT_REF" => reference,
            "TOVUK_CI_EVENT_REF_TYPE" => reference_type,
            "TOVUK_CI_EVENT_SHA" => event_sha,
            "TOVUK_CI_PULL_BASE_SHA" => base,
            "TOVUK_CI_PULL_HEAD_SHA" => head,
            "TOVUK_CI_PULL_MERGE_SHA" => merge,
            "TOVUK_CI_PULL_NUMBER" => number,
            "TOVUK_CI_PUSH_AFTER_SHA" => after,
            "TOVUK_CI_PUSH_BEFORE_SHA" => before,
            "TOVUK_CI_PUSH_CREATED" => created,
            "TOVUK_CI_PUSH_DELETED" => deleted,
            "TOVUK_CI_PUSH_FORCED" => forced,
            "TOVUK_CI_WORKFLOW_SHA" => workflow_sha,
            other => return Err(format!("fixture does not recognize {other}")),
        };
        return Ok(value.to_owned());
    };
    return read_environment_with(&mut reader);
}

/// Require one CI scanner scenario to fail closed.
///
/// # Errors
///
/// Returns an error when the invalid scenario is unexpectedly accepted.
pub(super) fn ensure_ci_rejected(result: &CheckResult, label: &str) -> CheckResult {
    if result.is_ok() {
        return Err(format!("trusted history scanner accepted {label}"));
    }
    return Ok(());
}

/// Merge the fixture topic into the selected trusted base.
///
/// # Errors
///
/// Returns an error when the synthetic merge commit cannot be created.
fn merge_topic(fixture: &PushFixture, message: &str) -> CheckResult {
    return run_git(
        fixture.repository.as_path(),
        &[
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=.git/tovuk-empty-hooks",
            "-c",
            "user.name=Tovuk Test",
            "-c",
            "user.email=tovuk-test@example.invalid",
            "merge",
            "--quiet",
            "--no-ff",
            "--message",
            message,
            "refs/heads/topic",
        ],
    );
}

/// Publish one exact object to a fixture remote ref without running hooks.
///
/// # Errors
///
/// Returns an error when the fixture remote rejects the ref update.
pub(super) fn publish_ref(fixture: &PushFixture, object: &str, reference: &str) -> CheckResult {
    let refspec = format!("{object}:{reference}");
    return run_git(
        fixture.repository.as_path(),
        &["push", "--no-verify", "origin", refspec.as_str()],
    );
}

/// Build a canonical ruleset-backed pull-request event.
///
/// # Errors
///
/// Returns an error when the fixture environment cannot be constructed.
pub(super) fn safe_pull_environment(
    fixture: &PushFixture,
    head: &str,
    merge: &str,
) -> CheckResult<CiEnvironment> {
    return safe_pull_environment_with_workflow(fixture, head, merge, fixture.baseline.as_str());
}

/// Build a pull-request event bound to one exact workflow authority.
///
/// # Errors
///
/// Returns an error when the fixture environment cannot be constructed.
pub(super) fn safe_pull_environment_with_workflow(
    fixture: &PushFixture,
    head: &str,
    merge: &str,
    workflow_sha: &str,
) -> CheckResult<CiEnvironment> {
    return construct_environment(&[
        "pull_request",
        merge,
        workflow_sha,
        "refs/pull/1/merge",
        "branch",
        "",
        "",
        "",
        "",
        "",
        fixture.baseline.as_str(),
        head,
        merge,
        "1",
    ]);
}

/// Build a canonical fast-forward branch push event.
///
/// # Errors
///
/// Returns an error when the fixture environment cannot be constructed.
pub(super) fn safe_push_environment(
    fixture: &PushFixture,
    target: &str,
) -> CheckResult<CiEnvironment> {
    return construct_environment(&[
        "push",
        target,
        target,
        "refs/heads/main",
        "branch",
        target,
        fixture.baseline.as_str(),
        "false",
        "false",
        "false",
        "",
        "",
        "",
        "",
    ]);
}

/// Verify a canonical fast-forward branch update is accepted.
///
/// # Errors
///
/// Returns an error when fixture setup or the trusted scan fails.
#[test]
fn verify_ci_snapshot_accepts_branch_update() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-safe-push"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    check_try!(publish_ref(&fixture, target.as_str(), "refs/heads/main"));
    let event = check_try!(safe_push_environment(&fixture, target.as_str()));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify a canonical branch deletion is accepted without object scanning.
///
/// # Errors
///
/// Returns an error when fixture setup or the trusted scan fails.
#[test]
fn verify_ci_snapshot_accepts_deletion() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-safe-deletion"));
    check_try!(publish_ref(
        &fixture,
        fixture.baseline.as_str(),
        "refs/heads/delete-me"
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["push", "--no-verify", "origin", ":refs/heads/delete-me"]
    ));
    let event = check_try!(construct_environment(&[
        "push",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/heads/delete-me",
        "branch",
        fixture.zero.as_str(),
        fixture.baseline.as_str(),
        "false",
        "true",
        "false",
        "",
        "",
        "",
        "",
    ]));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify a fetched pull merge with exact ordered parents is accepted.
///
/// # Errors
///
/// Returns an error when fixture setup or the trusted scan fails.
#[test]
fn verify_ci_snapshot_accepts_pull_request() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-safe-pull"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(&fixture, head.as_str(), "safe merge"));
    let event = check_try!(safe_pull_environment(
        &fixture,
        head.as_str(),
        merge.as_str(),
    ));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify a canonical annotated tag creation is accepted.
///
/// # Errors
///
/// Returns an error when fixture setup or the trusted scan fails.
#[test]
fn verify_ci_snapshot_accepts_tag_creation() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-safe-tag"));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["tag", "--annotate", "safe", "--message", "safe public tag"]
    ));
    let tag = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "refs/tags/safe"]
    ));
    check_try!(publish_ref(&fixture, tag.as_str(), "refs/tags/safe"));
    let event = check_try!(construct_environment(&[
        "push",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/tags/safe",
        "tag",
        fixture.baseline.as_str(),
        fixture.zero.as_str(),
        "true",
        "false",
        "false",
        "",
        "",
        "",
        "",
    ]));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify annotated tag metadata is scanned as a raw Git object.
///
/// # Errors
///
/// Returns an error when fixture setup fails or forbidden metadata is accepted.
#[test]
fn verify_ci_snapshot_rejects_forbidden_annotated_tag_metadata() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-tag-secret"));
    let credential = format!("{}{}", ["gh", "p_"].concat(), "aB3".repeat(0x000c));
    check_try!(run_git(
        fixture.repository.as_path(),
        &[
            "tag",
            "--annotate",
            "unsafe",
            "--message",
            credential.as_str()
        ],
    ));
    let tag = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "refs/tags/unsafe"]
    ));
    check_try!(publish_ref(&fixture, tag.as_str(), "refs/tags/unsafe"));
    let event = check_try!(construct_environment(&[
        "push",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/tags/unsafe",
        "tag",
        fixture.baseline.as_str(),
        fixture.zero.as_str(),
        "true",
        "false",
        "false",
        "",
        "",
        "",
        "",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "forbidden annotated tag metadata",
    ));
    return remove_fixture(&fixture);
}

/// Verify the generated merge metadata is included in the pull delta.
///
/// # Errors
///
/// Returns an error when fixture setup fails or forbidden metadata is accepted.
#[test]
fn verify_ci_snapshot_rejects_forbidden_pull_merge_metadata() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-pull-merge"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let credential = format!("{}{}", ["gh", "p_"].concat(), "aB3".repeat(0x000c));
    let merge = check_try!(build_pull_merge(
        &fixture,
        head.as_str(),
        credential.as_str()
    ));
    let event = check_try!(safe_pull_environment(
        &fixture,
        head.as_str(),
        merge.as_str()
    ));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "forbidden pull-request merge metadata",
    ));
    return remove_fixture(&fixture);
}

/// Verify contradictory ref creation flags fail closed.
///
/// # Errors
///
/// Returns an error when fixture setup fails or contradictory state is accepted.
#[test]
fn verify_ci_snapshot_rejects_invalid_creation_state() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-invalid-state"));
    let event = check_try!(construct_environment(&[
        "push",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/heads/main",
        "branch",
        fixture.baseline.as_str(),
        fixture.zero.as_str(),
        "false",
        "false",
        "false",
        "",
        "",
        "",
        "",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "a contradictory ref update",
    ));
    return remove_fixture(&fixture);
}

/// Verify a malformed event object identity fails closed.
///
/// # Errors
///
/// Returns an error when fixture setup fails or the malformed identity is accepted.
#[test]
fn verify_ci_snapshot_rejects_malformed_event_sha() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-malformed-sha"));
    let malformed = format!("g{}", fixture.baseline.get(0x0001..).unwrap_or_default());
    let event = check_try!(construct_environment(&[
        "push",
        malformed.as_str(),
        fixture.baseline.as_str(),
        "refs/heads/main",
        "branch",
        fixture.baseline.as_str(),
        fixture.zero.as_str(),
        "true",
        "false",
        "false",
        "",
        "",
        "",
        "",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "a malformed event SHA",
    ));
    return remove_fixture(&fixture);
}

/// Verify absence of any required event variable fails closed.
///
/// # Errors
///
/// Returns an error when a missing variable is unexpectedly accepted.
#[test]
fn verify_ci_snapshot_rejects_missing_environment() -> CheckResult {
    let mut missing_reader = |name: &str| -> CheckResult<String> {
        return Err(format!("fixture omits {name}"));
    };
    if read_environment_with(&mut missing_reader).is_ok() {
        return Err("CI environment reader accepted a missing variable".to_owned());
    }
    return Ok(());
}
