use std::{
    env::temp_dir,
    fs::{create_dir_all, remove_dir_all, write},
    path::{Path, PathBuf},
    process::{Command, id as process_id},
};

use crate::helpers::CheckResult;

use super::{
    git_status_success_in, is_ordinary_index_entry, require_snapshot_alignment_in,
    snapshot_alignment,
};

/// Valid `GitHub` classic-token body reconstructed behind a split prefix.
const FIXTURE_TOKEN_BODY: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
/// Git arguments that stage both sparse fixture directories.
const SPARSE_ADD_ARGS: &[&str] = &["add", "included", "excluded"];
/// Git arguments that select the included sparse cone.
const SPARSE_CHECKOUT_ARGS: &[&str] = &["sparse-checkout", "set", "included"];
/// Git arguments that expose the collapsed sparse-directory entry.
const SPARSE_ENTRY_ARGS: &[&str] = &[
    "ls-files",
    "--sparse",
    "--stage",
    "-t",
    "-z",
    "--full-name",
    "--",
    "excluded/",
];
/// Git arguments that create a cone-mode sparse index.
const SPARSE_INIT_ARGS: &[&str] = &["sparse-checkout", "init", "--cone", "--sparse-index"];

/// Commit the staged fixture state with deterministic local-only identity.
///
/// # Errors
///
/// Returns an error when the fixture commit fails.
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

/// Create a clean temporary Git repository with one committed public file.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created or initialized.
fn create_repository_fixture(label: &str) -> CheckResult<PathBuf> {
    return create_repository_fixture_with_init(label, &["init", "--quiet"]);
}

/// Create a clean fixture using explicit `git init` arguments.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created or initialized.
fn create_repository_fixture_with_init(label: &str, init_args: &[&str]) -> CheckResult<PathBuf> {
    let repository = temp_dir().join(format!(
        "tovuk-repo-hygiene-snapshot-test-{}-{label}",
        process_id()
    ));
    if check_try!(
        repository
            .try_exists()
            .map_err(|error| return format!("inspect fixture: {error}"))
    ) {
        check_try!(
            remove_dir_all(repository.as_path())
                .map_err(|error| return format!("clear fixture: {error}"))
        );
    }
    check_try!(
        create_dir_all(repository.as_path())
            .map_err(|error| return format!("create fixture: {error}"))
    );
    check_try!(run_git(repository.as_path(), init_args));
    check_try!(
        create_dir_all(repository.join(".git/tovuk-empty-hooks"))
            .map_err(|error| return format!("create empty hooks directory: {error}"))
    );
    check_try!(write_fixture(repository.as_path(), "public\n"));
    check_try!(run_git(repository.as_path(), &["add", "README.md"]));
    check_try!(commit_fixture(repository.as_path(), "initial"));
    return Ok(repository);
}

/// Capture stdout from one successful Git fixture command.
///
/// # Errors
///
/// Returns an error when Git cannot start or the command fails.
fn git_stdout(repository: &Path, args: &[&str]) -> CheckResult<Vec<u8>> {
    let output = check_try!(
        Command::new("git")
            .args(args)
            .current_dir(repository)
            .output()
            .map_err(|error| return format!("run fixture git {}: {error}", args.join(" ")))
    );
    if !output.status.success() {
        return Err(format!(
            "fixture git {} failed with {}; stderr: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(output.stderr.as_slice())
        ));
    }
    return Ok(output.stdout);
}

/// Prove one special index flag cannot conceal staged or committed bytes.
///
/// # Errors
///
/// Returns an error when the fixture does not reproduce Git's hidden-diff
/// behavior or the snapshot contract accepts that state.
fn prove_special_index_flag_cannot_hide(
    repository: &Path,
    flag: &str,
    update_args: &[&str],
) -> CheckResult {
    let credential = ["gh", "p_", FIXTURE_TOKEN_BODY].concat();
    check_try!(write_fixture(repository, credential.as_str()));
    check_try!(run_git(repository, &["add", "README.md"]));
    check_try!(write_fixture(repository, "public\n"));
    check_try!(run_git(repository, update_args));

    if !check_try!(git_status_success_in(
        repository,
        &["diff", "--quiet", "--"]
    )) {
        return Err(format!("{flag} fixture did not hide the staged bytes"));
    }
    if require_snapshot_alignment_in(repository, "index").is_ok() {
        return Err(format!(
            "index alignment accepted staged bytes hidden by {flag}"
        ));
    }

    check_try!(commit_fixture(repository, "hidden credential"));
    if !check_try!(git_status_success_in(
        repository,
        &["diff", "--quiet", "HEAD", "--"]
    )) {
        return Err(format!("{flag} fixture did not hide the committed bytes"));
    }
    if require_snapshot_alignment_in(repository, "head").is_ok() {
        return Err(format!(
            "HEAD alignment accepted committed bytes hidden by {flag}"
        ));
    }
    return Ok(());
}

/// Run one Git fixture command.
///
/// # Errors
///
/// Returns an error when Git cannot start or the command fails.
fn run_git(repository: &Path, args: &[&str]) -> CheckResult {
    let status = check_try!(
        Command::new("git")
            .args(args)
            .current_dir(repository)
            .status()
            .map_err(|error| return format!("run fixture git {}: {error}", args.join(" ")))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("fixture git {} failed with {status}", args.join(" ")));
}

/// Verify matching worktree, index, and `HEAD` snapshots remain accepted.
///
/// # Errors
///
/// Returns an error when clean fixture snapshots are rejected.
#[test]
fn snapshot_alignment_accepts_matching_git_bytes() -> CheckResult {
    let repository = check_try!(create_repository_fixture("clean"));
    check_try!(require_snapshot_alignment_in(repository.as_path(), "index"));
    check_try!(require_snapshot_alignment_in(repository.as_path(), "head"));
    return remove_dir_all(repository.as_path()).map_err(|error| format!("clear fixture: {error}"));
}

/// Verify clean SHA-256 repositories use accepted ordinary object records.
///
/// # Errors
///
/// Returns an explicit Git failure when SHA-256 repositories are unsupported,
/// or an error when either clean snapshot is rejected.
#[test]
fn snapshot_alignment_accepts_matching_sha256_git_bytes() -> CheckResult {
    let repository = check_try!(create_repository_fixture_with_init(
        "sha256",
        &["init", "--quiet", "--object-format=sha256"],
    ));
    let object_format = check_try!(git_stdout(
        repository.as_path(),
        &["rev-parse", "--show-object-format"]
    ));
    if object_format != b"sha256\n" {
        return Err("Git did not create the required SHA-256 repository".to_owned());
    }
    check_try!(require_snapshot_alignment_in(repository.as_path(), "index"));
    check_try!(require_snapshot_alignment_in(repository.as_path(), "head"));
    return remove_dir_all(repository.as_path()).map_err(|error| format!("clear fixture: {error}"));
}

/// Prove `assume-unchanged` cannot hide staged or committed credential bytes.
///
/// # Errors
///
/// Returns an error when snapshot alignment accepts the special index state.
#[test]
fn snapshot_alignment_rejects_assume_unchanged() -> CheckResult {
    let repository = check_try!(create_repository_fixture("assume-unchanged"));
    check_try!(prove_special_index_flag_cannot_hide(
        repository.as_path(),
        "assume-unchanged",
        &["update-index", "--assume-unchanged", "README.md"],
    ));
    return remove_dir_all(repository.as_path()).map_err(|error| format!("clear fixture: {error}"));
}

/// Prove an fsmonitor-valid machine record is rejected on every Git platform.
///
/// # Errors
///
/// Returns an error when the parser confuses ordinary and fsmonitor-valid tags.
#[test]
fn snapshot_alignment_rejects_fsmonitor_valid() -> CheckResult {
    let object = "a".repeat(0x0028);
    let ordinary = format!("H 100644 {object} 0\tREADME.md");
    let fsmonitor_valid = format!("h 100644 {object} 0\tREADME.md");
    if !is_ordinary_index_entry(ordinary.as_bytes())
        || is_ordinary_index_entry(fsmonitor_valid.as_bytes())
    {
        return Err("index parser did not isolate the fsmonitor-valid tag".to_owned());
    }
    return Ok(());
}

/// Prove staged and committed bytes cannot be hidden by a different worktree copy.
///
/// # Errors
///
/// Returns an error when snapshot alignment accepts a hidden fixture mutation.
#[test]
fn snapshot_alignment_rejects_hidden_git_bytes() -> CheckResult {
    let repository = check_try!(create_repository_fixture("hidden"));
    let credential = ["gh", "p_", FIXTURE_TOKEN_BODY].concat();
    check_try!(write_fixture(repository.as_path(), credential.as_str()));
    check_try!(run_git(repository.as_path(), &["add", "README.md"]));
    check_try!(write_fixture(repository.as_path(), "public\n"));

    if require_snapshot_alignment_in(repository.as_path(), "index").is_ok() {
        return Err("index alignment accepted staged bytes hidden by the worktree".to_owned());
    }
    check_try!(require_snapshot_alignment_in(repository.as_path(), "head"));

    check_try!(commit_fixture(repository.as_path(), "hidden credential"));
    if require_snapshot_alignment_in(repository.as_path(), "head").is_ok() {
        return Err("HEAD alignment accepted committed bytes hidden by the worktree".to_owned());
    }

    check_try!(
        remove_dir_all(repository.as_path())
            .map_err(|error| return format!("clear fixture: {error}"))
    );
    return Ok(());
}

/// Prove `skip-worktree` cannot hide staged or committed credential bytes.
///
/// # Errors
///
/// Returns an error when snapshot alignment accepts the special index state.
#[test]
fn snapshot_alignment_rejects_skip_worktree() -> CheckResult {
    let repository = check_try!(create_repository_fixture("skip-worktree"));
    check_try!(prove_special_index_flag_cannot_hide(
        repository.as_path(),
        "skip-worktree",
        &["update-index", "--skip-worktree", "README.md"],
    ));
    return remove_dir_all(repository.as_path()).map_err(|error| format!("clear fixture: {error}"));
}

/// Prove a real sparse-directory index entry is rejected before diff alignment.
///
/// # Errors
///
/// Returns explicit Git evidence when sparse indexes are unsupported, or an
/// error when the fixture is not sparse or either snapshot accepts it.
#[test]
fn snapshot_alignment_rejects_sparse_index() -> CheckResult {
    let repository = check_try!(create_repository_fixture("sparse-index"));
    check_try!(write_path_fixture(
        repository.as_path(),
        "included/visible.txt",
        "public\n"
    ));
    check_try!(write_path_fixture(
        repository.as_path(),
        "excluded/credential.txt",
        "private\n"
    ));
    check_try!(run_git(repository.as_path(), SPARSE_ADD_ARGS));
    check_try!(commit_fixture(repository.as_path(), "add sparse fixture"));
    check_try!(run_git(repository.as_path(), SPARSE_INIT_ARGS));
    check_try!(run_git(repository.as_path(), SPARSE_CHECKOUT_ARGS));
    let entry = check_try!(git_stdout(repository.as_path(), SPARSE_ENTRY_ARGS));
    if !entry.starts_with(b"S 040000 ") || !entry.ends_with(b" 0\texcluded/\0") {
        return Err("Git did not create the required sparse-directory index entry".to_owned());
    }
    if require_snapshot_alignment_in(repository.as_path(), "index").is_ok()
        || require_snapshot_alignment_in(repository.as_path(), "head").is_ok()
    {
        return Err("snapshot alignment accepted a sparse-directory entry".to_owned());
    }
    return remove_dir_all(repository.as_path()).map_err(|error| format!("clear fixture: {error}"));
}

/// Verify unsupported snapshot names fail closed.
///
/// # Panics
///
/// Panics when an unknown snapshot is accepted.
#[test]
fn snapshot_alignment_rejects_unknown_names() {
    assert!(
        snapshot_alignment("worktree").is_err(),
        "unknown snapshot must fail closed"
    );
}

/// Replace the fixture's tracked file.
///
/// # Errors
///
/// Returns an error when the fixture file cannot be written.
fn write_fixture(repository: &Path, contents: &str) -> CheckResult {
    return write_path_fixture(repository, "README.md", contents);
}

/// Write one fixture path after creating its parent directory.
///
/// # Errors
///
/// Returns an error when the directory or file cannot be written.
fn write_path_fixture(repository: &Path, path: &str, contents: &str) -> CheckResult {
    let destination = repository.join(path);
    let Some(parent) = destination.parent() else {
        return Err(format!("fixture path has no parent: {path}"));
    };
    check_try!(create_dir_all(parent).map_err(|error| return format!("create {path}: {error}")));
    return write(destination, contents).map_err(|error| format!("write {path}: {error}"));
}
