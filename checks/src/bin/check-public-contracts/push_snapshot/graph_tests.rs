//! Local pre-push Git graph-integrity regressions.

use std::fs::{read, write};

use crate::helpers::CheckResult;

use super::{
    PUSH_LOCATION, append_safe_readme, check_input_in, commit_fixture, create_fixture, git_text,
    remove_fixture, run_git, write_secret_readme,
};

use super::test_helpers::{record, require_rejected};

/// Verify legacy grafts cannot hide a secret-bearing ancestor from pre-push.
///
/// # Errors
///
/// Returns an error when fixture setup fails or grafted history is accepted.
#[test]
fn pre_push_rejects_grafted_secret_ancestry() -> CheckResult {
    let fixture = check_try!(create_fixture("pre-push-graft"));
    let public_readme = check_try!(
        read(fixture.repository.join("README.md"))
            .map_err(|error| format!("read public README fixture: {error}"))
    );
    check_try!(write_secret_readme(
        fixture.repository.as_path(),
        "graft-hidden secret",
    ));
    check_try!(
        write(fixture.repository.join("README.md"), public_readme)
            .map_err(|error| format!("restore public README fixture: {error}"))
    );
    check_try!(run_git(fixture.repository.as_path(), &["add", "README.md"]));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "clean graft tip"
    ));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    check_try!(
        write(
            fixture.repository.join(".git/info/grafts"),
            format!("{target}\n"),
        )
        .map_err(|error| format!("write graft fixture: {error}"))
    );
    let input = record(
        "refs/heads/main",
        target.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str()),
        "legacy graft-hidden ancestry",
    ));
    return remove_fixture(&fixture);
}

/// Verify local pre-push scanning rejects shallow object history.
///
/// # Errors
///
/// Returns an error when fixture setup fails or shallow history is accepted.
#[test]
fn pre_push_rejects_shallow_history() -> CheckResult {
    let fixture = check_try!(create_fixture("pre-push-shallow"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    check_try!(
        write(
            fixture.repository.join(".git/shallow"),
            format!("{}\n", fixture.baseline),
        )
        .map_err(|error| format!("write shallow fixture: {error}"))
    );
    let input = record(
        "refs/heads/main",
        target.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str()),
        "shallow pre-push history",
    ));
    return remove_fixture(&fixture);
}
