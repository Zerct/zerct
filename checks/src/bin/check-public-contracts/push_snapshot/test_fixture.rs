//! Isolated Git fixtures for pushed-object verification.

#[path = "ci_adversarial_tests.rs"]
mod ci_adversarial_tests;

#[path = "ci_core_tests.rs"]
mod ci_core_tests;

#[path = "ci_surface_tests.rs"]
mod ci_surface_tests;

#[path = "ci_tests.rs"]
mod ci_tests;

#[path = "fixture_root.rs"]
mod fixture_root;

#[path = "graph_tests.rs"]
mod graph_tests;

#[path = "history_rewrite_tests.rs"]
mod history_rewrite_tests;

#[path = "test_helpers.rs"]
mod test_helpers;

use std::{
    fs::{copy, create_dir_all, metadata, read, write},
    path::{Path, PathBuf},
};

use crate::{helpers::CheckResult, repo_hygiene_required::reviewed_tracked_paths};

use super::check_input_in;

use fixture_root::allocate_fixture_root;
use test_helpers::{
    initialize_public_history, record, remove_fixture, require_rejected, run_git,
    synchronize_fixture_public_tree, verify_noncommit_target, verify_safe_branch,
    verify_safe_deletion, verify_safe_lightweight_tag, verify_safe_tag, write_secret_readme,
};

/// Actual location Git supplies as pre-push hook argument two.
const PUSH_LOCATION: &str = "../remote.git";

/// A working repository paired with an independently queried bare remote.
#[derive(Debug)]
struct PushFixture {
    /// Object advertised by the remote main branch.
    baseline: String,
    /// Repository in which proposed objects and refs are created.
    repository: PathBuf,
    /// Fixture root removed after each test.
    root: PathBuf,
    /// All-zero object sentinel for this repository.
    zero: String,
}

/// One remote commit deliberately absent from the primary local object database.
#[derive(Debug, Eq, PartialEq)]
struct UnavailableRemote {
    /// Commit object published by the independent clone.
    object: String,
    /// Independent clone that owns the commit locally.
    repository: PathBuf,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0013] = [
    size_of_val(&append_safe_readme),
    size_of_val(&commit_fixture),
    size_of_val(&configure_divergent_remote),
    size_of_val(&copy_public_tree),
    size_of_val(&create_fixture),
    size_of_val(&create_unavailable_remote_commit),
    size_of_val(&git_text),
    size_of_val(&initialize_repositories),
    size_of_val(&record),
    size_of_val(&remove_fixture),
    size_of_val(&require_rejected),
    size_of_val(&run_git),
    size_of_val(&synchronize_fixture_public_tree),
    size_of_val(&verify_noncommit_target),
    size_of_val(&verify_safe_branch),
    size_of_val(&verify_safe_deletion),
    size_of_val(&verify_safe_lightweight_tag),
    size_of_val(&verify_safe_tag),
    size_of_val(&write_secret_readme),
];

/// Append canonical public copy and commit a safe branch update.
///
/// # Errors
///
/// Returns an error when the fixture cannot write or commit public bytes.
fn append_safe_readme(repository: &Path) -> CheckResult {
    let mut contents = check_try!(
        read(repository.join("README.md"))
            .map_err(|error| return format!("read safe README fixture: {error}"))
    );
    contents.extend_from_slice(b"Safe push snapshot fixture.\n");
    check_try!(
        write(repository.join("README.md"), contents)
            .map_err(|error| return format!("write safe README fixture: {error}"))
    );
    check_try!(run_git(repository, &["add", "README.md"]));
    return commit_fixture(repository, "safe public update");
}

/// Commit staged fixture state with hooks and signing disabled.
///
/// # Errors
///
/// Returns an error when the isolated commit fails.
fn commit_fixture(repository: &Path, message: &str) -> CheckResult {
    return run_git(
        repository,
        &[
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=.git/tovuk-empty-hooks",
            "-c",
            "user.name=Tovuk Test",
            "-c",
            "user.email=tovuk-test@example.invalid",
            "commit",
            "--quiet",
            "--no-verify",
            "--message",
            message,
        ],
    );
}

/// Configure a decoy fetch URL and a distinct actual push location.
///
/// # Errors
///
/// Returns an error when the fixture remote URLs cannot be configured.
fn configure_divergent_remote(repository: &Path) -> CheckResult {
    check_try!(run_git(
        repository,
        &["remote", "set-url", "fixture", "../decoy.git"]
    ));
    return run_git(
        repository,
        &["remote", "set-url", "--push", "fixture", PUSH_LOCATION],
    );
}

/// Copy the exact reviewed worktree into an isolated Git repository.
///
/// # Errors
///
/// Returns an error when a reviewed path cannot be copied exactly.
fn copy_public_tree(destination: &Path) -> CheckResult {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = check_try!(
        manifest
            .parent()
            .ok_or_else(|| return "checks manifest has no repository parent".to_owned())
    );
    for relative in check_try!(reviewed_tracked_paths()) {
        let source_path = source.join(relative.as_str());
        let target = destination.join(relative.as_str());
        if let Some(parent) = target.parent() {
            check_try!(
                create_dir_all(parent)
                    .map_err(|error| return format!("create fixture parent: {error}"))
            );
        }
        let copied_bytes = check_try!(
            copy(source_path.as_path(), target)
                .map_err(|error| return format!("copy fixture path {relative}: {error}"))
        );
        let expected_bytes = check_try!(metadata(source_path).map_err(|error| {
            return format!("inspect fixture source path {relative}: {error}");
        }))
        .len();
        if copied_bytes != expected_bytes {
            return Err(format!("fixture copy changed byte count for {relative}"));
        }
    }
    return Ok(());
}

/// Create one exact public commit and publish it to a bare fixture remote.
///
/// # Errors
///
/// Returns an error when fixture setup or baseline publication fails.
fn create_fixture(label: &str) -> CheckResult<PushFixture> {
    let root = check_try!(allocate_fixture_root(label));
    let repository = root.join("work");
    check_try!(initialize_repositories(
        root.as_path(),
        repository.as_path()
    ));
    check_try!(initialize_public_history(repository.as_path()));
    check_try!(run_git(
        repository.as_path(),
        &["remote", "add", "fixture", "../remote.git"]
    ));
    check_try!(run_git(
        repository.as_path(),
        &["push", "--no-verify", "fixture", "HEAD:refs/heads/main"]
    ));
    check_try!(run_git(
        repository.as_path(),
        &["remote", "add", "origin", "../remote.git"]
    ));
    check_try!(configure_divergent_remote(repository.as_path()));
    let baseline = check_try!(git_text(repository.as_path(), &["rev-parse", "HEAD"]));
    let zero = "0".repeat(baseline.len());
    return Ok(PushFixture {
        baseline,
        repository,
        root,
        zero,
    });
}

/// Publish one safe remote commit without adding its objects to the local fixture.
///
/// # Errors
///
/// Returns an error when the independent clone cannot commit or publish.
fn create_unavailable_remote_commit(fixture: &PushFixture) -> CheckResult<UnavailableRemote> {
    check_try!(run_git(
        fixture.root.as_path(),
        &[
            "clone",
            "--quiet",
            "--branch",
            "main",
            "remote.git",
            "foreign"
        ]
    ));
    let foreign = fixture.root.join("foreign");
    check_try!(append_safe_readme(foreign.as_path()));
    let object = check_try!(git_text(foreign.as_path(), &["rev-parse", "HEAD"]));
    check_try!(run_git(
        foreign.as_path(),
        &["push", "--no-verify", "origin", "HEAD:refs/heads/hidden"]
    ));
    return Ok(UnavailableRemote {
        object,
        repository: foreign,
    });
}

/// Run Git and return one trimmed UTF-8 output value.
///
/// # Errors
///
/// Returns an error when Git fails or returns non-UTF-8 output.
fn git_text(repository: &Path, args: &[&str]) -> CheckResult<String> {
    let output = check_try!(
        super::git_command(repository)
            .args(args)
            .output()
            .map_err(|error| return format!("run fixture git {}: {error}", args.join(" ")),)
    );
    if !output.status.success() {
        return Err(format!(
            "fixture git {} failed with {}",
            args.join(" "),
            output.status
        ));
    }
    return String::from_utf8(output.stdout)
        .map(|text| return text.trim_end_matches('\n').to_owned())
        .map_err(|error| return format!("fixture Git output is not UTF-8: {error}"));
}

/// Initialize an empty working repository and independent bare remote.
///
/// # Errors
///
/// Returns an error when Git or fixture-directory initialization fails.
fn initialize_repositories(root: &Path, repository: &Path) -> CheckResult {
    check_try!(run_git(
        root,
        &[
            "init",
            "--bare",
            "--quiet",
            "--initial-branch=main",
            "remote.git"
        ]
    ));
    check_try!(run_git(
        root,
        &[
            "init",
            "--bare",
            "--quiet",
            "--initial-branch=main",
            "decoy.git"
        ]
    ));
    check_try!(create_dir_all(repository).map_err(|error| return format!("create work: {error}")));
    check_try!(run_git(
        repository,
        &["init", "--quiet", "--initial-branch=main"]
    ));
    return create_dir_all(repository.join(".git/tovuk-empty-hooks"))
        .map_err(|error| return format!("create empty hooks: {error}"));
}

/// Verify unavailable unrelated tips are filtered while exact bases fail closed.
///
/// # Errors
///
/// Returns an error when filtering fails or an unavailable exact base is accepted.
#[test]
fn push_snapshot_filters_unavailable_new_ref_exclusions() -> CheckResult {
    let fixture = check_try!(create_fixture("unavailable"));
    let unavailable = check_try!(create_unavailable_remote_commit(&fixture));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["update-ref", "refs/heads/topic", fixture.baseline.as_str()]
    ));
    let creation = record(
        "refs/heads/topic",
        fixture.baseline.as_str(),
        "refs/heads/topic",
        fixture.zero.as_str(),
    );
    check_try!(check_input_in(
        fixture.repository.as_path(),
        PUSH_LOCATION,
        creation.as_str()
    ));
    check_try!(run_git(
        unavailable.repository.as_path(),
        &["push", "--no-verify", "origin", "HEAD:refs/heads/main"]
    ));
    let update = record(
        "refs/heads/main",
        fixture.baseline.as_str(),
        "refs/heads/main",
        unavailable.object.as_str(),
    );
    let result = check_input_in(fixture.repository.as_path(), PUSH_LOCATION, update.as_str());
    let Err(error) = result else {
        return Err("push scanner accepted an unavailable exact remote base".to_owned());
    };
    if !error.contains("fetch the destination ref before pushing") {
        return Err(format!(
            "missing actionable unavailable-base diagnostic: {error}"
        ));
    }
    return remove_fixture(&fixture);
}

/// Verify malformed, mismatched, unknown, and noncommit updates fail closed.
///
/// # Errors
///
/// Returns an error when any invalid update is accepted.
#[test]
fn push_snapshot_rejects_malformed_and_unknown_targets() -> CheckResult {
    let fixture = check_try!(create_fixture("invalid"));
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, "malformed\n"),
        "malformed input"
    ));
    let mismatch = record(
        "refs/heads/main",
        fixture.baseline.as_str(),
        "refs/heads/main",
        fixture.zero.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(
            fixture.repository.as_path(),
            PUSH_LOCATION,
            mismatch.as_str()
        ),
        "stale remote state"
    ));
    let unknown = record(
        "refs/heads/main",
        fixture.baseline.as_str(),
        "refs/notes/test",
        fixture.zero.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(
            fixture.repository.as_path(),
            PUSH_LOCATION,
            unknown.as_str()
        ),
        "unknown ref namespace"
    ));
    check_try!(verify_noncommit_target(&fixture));
    return remove_fixture(&fixture);
}

/// Verify a secret on a non-HEAD ref cannot bypass the scanner.
///
/// # Errors
///
/// Returns an error when a secret-bearing topic ref is accepted.
#[test]
fn push_snapshot_rejects_non_head_secret() -> CheckResult {
    let fixture = check_try!(create_fixture("non-head"));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["switch", "--quiet", "--create", "topic"]
    ));
    check_try!(write_secret_readme(
        fixture.repository.as_path(),
        "topic secret"
    ));
    let proposed = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let input = record(
        "refs/heads/topic",
        proposed.as_str(),
        "refs/heads/topic",
        fixture.zero.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str()),
        "non-HEAD secret"
    ));
    return remove_fixture(&fixture);
}

/// Verify a removed intermediate secret remains visible in push history.
///
/// # Errors
///
/// Returns an error when a clean tip conceals a secret-bearing parent commit.
#[test]
fn push_snapshot_rejects_removed_intermediate_secret() -> CheckResult {
    let fixture = check_try!(create_fixture("intermediate"));
    let public_readme = check_try!(
        read(fixture.repository.join("README.md")).map_err(|error| format!("read README: {error}"))
    );
    check_try!(write_secret_readme(
        fixture.repository.as_path(),
        "intermediate secret"
    ));
    check_try!(
        write(fixture.repository.join("README.md"), public_readme)
            .map_err(|error| format!("restore README: {error}"))
    );
    check_try!(run_git(fixture.repository.as_path(), &["add", "README.md"]));
    check_try!(commit_fixture(
        fixture.repository.as_path(),
        "clean public tip"
    ));
    let proposed = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let input = record(
        "refs/heads/main",
        proposed.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str()),
        "removed intermediate secret"
    ));
    return remove_fixture(&fixture);
}

/// Verify safe branch, annotated-tag, and deletion records are accepted.
///
/// # Errors
///
/// Returns an error when a valid public update fails verification.
#[test]
fn push_snapshot_verifies_safe_updates_and_deletions() -> CheckResult {
    let fixture = check_try!(create_fixture("safe"));
    check_try!(verify_safe_branch(&fixture));
    check_try!(verify_safe_tag(&fixture));
    check_try!(verify_safe_lightweight_tag(&fixture));
    check_try!(verify_safe_deletion(&fixture));
    return remove_fixture(&fixture);
}
