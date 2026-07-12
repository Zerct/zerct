//! Pull-request public-surface and created-ref history fixtures.

use std::{
    fs::{create_dir_all, write},
    path::Path,
};

use crate::helpers::CheckResult;

use super::{
    PushFixture, append_safe_readme, commit_fixture, create_fixture, git_text, remove_fixture,
    run_git, synchronize_fixture_public_tree,
};

use super::ci_tests::{
    build_pull_merge, construct_environment, ensure_ci_rejected, publish_ref, safe_pull_environment,
};

use super::super::continuous_integration::check_environment_in;

/// Whether a fixture path requires an explicit ignored-path override.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AddMode {
    /// Add a normal reviewed path.
    Normal,
    /// Force-add an intentionally sensitive ignored path.
    Sensitive,
}

/// Compile-time references preserve the named helper boundary.
const _: [usize; 0x0001] = [size_of_val(&commit_path)];

/// Write, stage, and commit one fixture path.
///
/// # Errors
///
/// Returns an error when its parent, bytes, index, or commit cannot be created.
fn commit_path(
    fixture: &PushFixture,
    relative: &str,
    contents: &str,
    mode: AddMode,
) -> CheckResult<String> {
    let path = fixture.repository.join(relative);
    if let Some(parent) = path
        .parent()
        .filter(|parent| return *parent != Path::new(""))
    {
        check_try!(create_dir_all(parent).map_err(|error| format!("create fixture path: {error}")));
    }
    check_try!(write(path, contents).map_err(|error| format!("write fixture path: {error}")));
    let arguments = match mode {
        AddMode::Normal => vec!["add", "--", relative],
        AddMode::Sensitive => vec!["add", "--force", "--", relative],
    };
    check_try!(run_git(fixture.repository.as_path(), arguments.as_slice()));
    check_try!(synchronize_fixture_public_tree(
        fixture.repository.as_path()
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "public surface fixture",
    ));
    return git_text(fixture.repository.as_path(), &["rev-parse", "HEAD"]);
}

/// Verify a new branch scans only objects unique from independently observed main.
///
/// # Errors
///
/// Returns an error when evolving main history is rescanned or the unique update fails.
#[test]
fn verify_ci_snapshot_accepts_created_branch_from_main() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-created-branch"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    check_try!(publish_ref(
        &fixture,
        target.as_str(),
        "refs/heads/safe-created"
    ));
    let event = check_try!(construct_environment(&[
        "push",
        target.as_str(),
        target.as_str(),
        "refs/heads/safe-created",
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
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify later movement of the synthetic merge ref cannot change an event.
///
/// # Errors
///
/// Returns an error when fixture setup or immutable event scanning fails.
#[test]
fn verify_ci_snapshot_accepts_pull_reference_movement() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-moved-pull-ref"));
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
    let replacement = format!("+{}", fixture.baseline);
    check_try!(publish_ref(
        &fixture,
        replacement.as_str(),
        "refs/pull/1/merge",
    ));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify trusted pull history permits a new file inside a public namespace.
///
/// # Errors
///
/// Returns an error when setup or public-surface scanning fails.
#[test]
fn verify_ci_snapshot_accepts_safe_new_pull_path() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-safe-pull-path"));
    let head = check_try!(commit_path(
        &fixture,
        "docs/new-public-page.mdx",
        "# New public page\n",
        AddMode::Normal,
    ));
    let merge = check_try!(build_pull_merge(&fixture, head.as_str(), "safe path merge"));
    let event = check_try!(safe_pull_environment(
        &fixture,
        head.as_str(),
        merge.as_str()
    ));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify trusted pull history permits removing a non-core public file.
///
/// # Errors
///
/// Returns an error when setup or public-surface scanning fails.
#[test]
fn verify_ci_snapshot_accepts_safe_pull_removal() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-safe-pull-removal"));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["rm", "--quiet", "--", "docs/changelog.mdx"]
    ));
    check_try!(synchronize_fixture_public_tree(
        fixture.repository.as_path()
    ));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "remove public documentation",
    ));
    let head = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let merge = check_try!(build_pull_merge(
        &fixture,
        head.as_str(),
        "safe removal merge"
    ));
    let event = check_try!(safe_pull_environment(
        &fixture,
        head.as_str(),
        merge.as_str()
    ));
    check_try!(check_environment_in(fixture.repository.as_path(), &event));
    return remove_fixture(&fixture);
}

/// Verify trusted pull history rejects a sensitive ignored path.
///
/// # Errors
///
/// Returns an error when setup fails or the sensitive path is accepted.
#[test]
fn verify_ci_snapshot_rejects_sensitive_pull_path() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-sensitive-pull-path"));
    let head = check_try!(commit_path(
        &fixture,
        ".env",
        "PUBLIC_PLACEHOLDER=true\n",
        AddMode::Sensitive,
    ));
    let merge = check_try!(build_pull_merge(
        &fixture,
        head.as_str(),
        "sensitive path merge"
    ));
    let event = check_try!(safe_pull_environment(
        &fixture,
        head.as_str(),
        merge.as_str()
    ));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "a sensitive pull request path",
    ));
    return remove_fixture(&fixture);
}

/// Verify trusted pull history rejects an unapproved top-level file.
///
/// # Errors
///
/// Returns an error when setup fails or the unapproved path is accepted.
#[test]
fn verify_ci_snapshot_rejects_unapproved_pull_path() -> CheckResult {
    let fixture = check_try!(create_fixture("ci-unapproved-pull-path"));
    let head = check_try!(commit_path(
        &fixture,
        "unexpected.txt",
        "unexpected public surface\n",
        AddMode::Normal,
    ));
    let merge = check_try!(build_pull_merge(
        &fixture,
        head.as_str(),
        "unapproved path merge"
    ));
    let event = check_try!(safe_pull_environment(
        &fixture,
        head.as_str(),
        merge.as_str()
    ));
    check_try!(ensure_ci_rejected(
        &check_environment_in(fixture.repository.as_path(), &event),
        "an unapproved pull request path",
    ));
    return remove_fixture(&fixture);
}
