//! Adversarial trusted history event fixtures.

use std::fs::{read, write};

use crate::helpers::CheckResult;

use super::{
    PushFixture, append_safe_readme, commit_fixture, create_fixture, git_text, remove_fixture,
    run_git, write_secret_readme,
};

use super::ci_tests::{
    build_pull_merge, construct_environment, ensure_ci_rejected, publish_ref, safe_push_environment,
};

use super::super::continuous_integration::check_environment_in;

/// Compile-time references preserve the named helper boundary.
const _: [usize; 0x0001] = [size_of_val(&commit_clean_tip)];

/// Commit a secret and then restore the reviewed README at a clean tip.
///
/// # Errors
///
/// Returns an error when fixture bytes or commits cannot be created.
fn commit_clean_tip(fixture: &PushFixture, label: &str) -> CheckResult<String> {
    let public_readme = check_try!(
        read(fixture.repository.join("README.md"))
            .map_err(|error| format!("read public README fixture: {error}"))
    );
    check_try!(write_secret_readme(fixture.repository.as_path(), label));
    check_try!(
        write(fixture.repository.join("README.md"), public_readme)
            .map_err(|error| format!("restore public README fixture: {error}"))
    );
    check_try!(run_git(fixture.repository.as_path(), &["add", "README.md"]));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "clean event tip",
    ));
    return git_text(fixture.repository.as_path(), &["rev-parse", "HEAD"]);
}

/// Verify a pull event cannot substitute its base for the generated merge.
///
/// # Errors
///
/// Returns an error when fixture setup fails or a mismatched event SHA is accepted.
#[test]
fn verify_ci_snapshot_rejects_pull_event_sha_mismatch() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-pull-event-sha"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(&fixture, head.as_str(), "safe merge"));
    let environment = check_try!(construct_environment(&[
        "pull_request",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/pull/1/merge",
        "branch",
        "",
        "",
        "",
        "",
        "",
        fixture.baseline.as_str(),
        head.as_str(),
        merge.as_str(),
        "1",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &environment),
        "a pull event SHA that differs from its merge",
    ));
    return remove_fixture(&fixture);
}

/// Verify a pull event cannot redirect scanning to another pull-request ref.
///
/// # Errors
///
/// Returns an error when fixture setup fails or a mismatched merge ref is accepted.
#[test]
fn verify_ci_snapshot_rejects_pull_reference_mismatch() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-pull-event-ref"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(&fixture, head.as_str(), "safe merge"));
    let environment = check_try!(construct_environment(&[
        "pull_request",
        merge.as_str(),
        fixture.baseline.as_str(),
        "refs/pull/2/merge",
        "branch",
        "",
        "",
        "",
        "",
        "",
        fixture.baseline.as_str(),
        head.as_str(),
        merge.as_str(),
        "1",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &environment),
        "a pull merge ref that differs from its number",
    ));
    return remove_fixture(&fixture);
}

/// Verify a clean tip cannot conceal a secret introduced earlier in an update.
///
/// # Errors
///
/// Returns an error when fixture setup fails or hidden secret history is accepted.
#[test]
fn verify_ci_snapshot_rejects_removed_intermediate_secret() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-secret-history"));
    let target = check_try!(commit_clean_tip(&fixture, "CI intermediate secret"));
    check_try!(publish_ref(&fixture, target.as_str(), "refs/heads/main"));
    let environment = check_try!(safe_push_environment(&fixture, target.as_str()));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &environment),
        "removed intermediate secret history",
    ));
    return remove_fixture(&fixture);
}

/// Verify created refs scan their complete reachable graph without exclusions.
///
/// # Errors
///
/// Returns an error when fixture setup fails or historical leakage is accepted.
#[test]
fn verify_ci_snapshot_rejects_secret_history_on_created_ref() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-created-secret"));
    let target = check_try!(commit_clean_tip(&fixture, "created ref secret"));
    check_try!(publish_ref(
        &fixture,
        target.as_str(),
        "refs/heads/created-secret"
    ));
    let environment = check_try!(construct_environment(&[
        "push",
        target.as_str(),
        target.as_str(),
        "refs/heads/created-secret",
        "branch",
        target.as_str(),
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
        &check_environment_in(fixture.repository.as_path(), &environment),
        "secret history on a created ref",
    ));
    return remove_fixture(&fixture);
}

/// Verify shallow history cannot be mistaken for a complete event boundary.
///
/// # Errors
///
/// Returns an error when fixture setup fails or shallow history is accepted.
#[test]
fn verify_ci_snapshot_rejects_shallow_history() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-shallow"));
    check_try!(
        write(
            fixture.repository.join(".git/shallow"),
            format!("{}\n", fixture.baseline),
        )
        .map_err(|error| format!("write shallow boundary: {error}"))
    );
    let environment = check_try!(construct_environment(&[
        "push",
        fixture.baseline.as_str(),
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
        &check_environment_in(fixture.repository.as_path(), &environment),
        "shallow event history",
    ));
    return remove_fixture(&fixture);
}

/// Verify a force boundary cannot name an unavailable former object.
///
/// # Errors
///
/// Returns an error when fixture setup fails or the absent boundary is accepted.
#[test]
fn verify_ci_snapshot_rejects_unavailable_forced_boundary() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-force-boundary"));
    let unavailable = "f".repeat(fixture.baseline.len());
    let environment = check_try!(construct_environment(&[
        "push",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/heads/main",
        "branch",
        fixture.baseline.as_str(),
        unavailable.as_str(),
        "false",
        "false",
        "true",
        "",
        "",
        "",
        "",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &environment),
        "an unavailable forced-update boundary",
    ));
    return remove_fixture(&fixture);
}

/// Verify a pull number cannot select an arbitrary remote ref.
///
/// # Errors
///
/// Returns an error when fixture setup fails or a nonnumeric selector is accepted.
#[test]
fn verify_ci_snapshot_rejects_unnumeric_pull_ref() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-pull-number"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(&fixture, head.as_str(), "safe merge"));
    let environment = check_try!(construct_environment(&[
        "pull_request",
        merge.as_str(),
        fixture.baseline.as_str(),
        "refs/pull/1/merge",
        "branch",
        "",
        "",
        "",
        "",
        "",
        fixture.baseline.as_str(),
        head.as_str(),
        merge.as_str(),
        "1/../../heads/main",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &environment),
        "a nonnumeric pull ref selector",
    ));
    return remove_fixture(&fixture);
}

/// Verify the generated pull merge must retain exact parent order.
///
/// # Errors
///
/// Returns an error when fixture setup fails or reversed identities are accepted.
#[test]
fn verify_ci_snapshot_rejects_unordered_pull_parents() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-pull-parents"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(&fixture, head.as_str(), "safe merge"));
    let environment = check_try!(construct_environment(&[
        "pull_request",
        merge.as_str(),
        fixture.baseline.as_str(),
        "refs/pull/1/merge",
        "branch",
        "",
        "",
        "",
        "",
        "",
        head.as_str(),
        fixture.baseline.as_str(),
        merge.as_str(),
        "1",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &environment),
        "reversed pull merge parents",
    ));
    return remove_fixture(&fixture);
}

/// Verify manual dispatch cannot masquerade as a ref-history event.
///
/// # Errors
///
/// Returns an error when fixture setup fails or manual dispatch is accepted.
#[test]
fn verify_ci_snapshot_rejects_workflow_dispatch() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-dispatch"));
    let event = check_try!(construct_environment(&[
        "workflow_dispatch",
        fixture.baseline.as_str(),
        fixture.baseline.as_str(),
        "refs/heads/main",
        "branch",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
    ]));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "manual dispatch as a history audit",
    ));
    return remove_fixture(&fixture);
}
