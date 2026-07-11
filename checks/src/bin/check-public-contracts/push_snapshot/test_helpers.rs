//! Reusable helpers for isolated pushed-object Git scenarios.

use alloc::collections::BTreeSet;

use std::{
    fs::{remove_dir_all, write},
    path::Path,
    process::Command,
};

use crate::{
    helpers::CheckResult,
    repo_hygiene_required::{PUBLIC_TREE_POLICY_PATH, render_public_tree_policy},
};

use tovuk_public_checks::check_support::git_tracked_files;

use super::{
    PUSH_LOCATION, PushFixture, append_safe_readme, check_input_in, commit_fixture,
    copy_public_tree, create_fixture, git_text,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&commit_malformed_tree),
    size_of_val(&create_malformed_tree_commit),
    size_of_val(&git_bytes),
    size_of_val(&initialize_public_history),
    size_of_val(&synchronize_fixture_public_tree),
];

/// Commit one prepared malformed root tree and expose it through a local ref.
///
/// # Errors
///
/// Returns an error when `commit-tree` or `update-ref` fails.
fn commit_malformed_tree(fixture: &PushFixture, tree: &str) -> CheckResult<String> {
    let commit = check_try!(git_text(
        fixture.repository.as_path(),
        &[
            "-c",
            "user.name=Tovuk Test",
            "-c",
            "user.email=tovuk-test@example.invalid",
            "commit-tree",
            tree,
            "-p",
            fixture.baseline.as_str(),
            "-m",
            "malformed directory mode"
        ]
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["update-ref", "refs/heads/malformed-tree", commit.as_str()]
    ));
    return Ok(commit);
}

/// Create a commit whose root tree contains a zero-padded directory mode.
///
/// # Errors
///
/// Returns an error when the malformed object fixture cannot be created.
fn create_malformed_tree_commit(fixture: &PushFixture) -> CheckResult<String> {
    let tree_expression = ["HEAD^", "{", "tree", "}"].concat();
    let root_tree = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", tree_expression.as_str()]
    ));
    let raw_tree = check_try!(git_bytes(
        fixture.repository.as_path(),
        &["cat-file", "tree", root_tree.as_str()]
    ));
    let remainder = check_try!(raw_tree.strip_prefix(b"40000 ").ok_or_else(|| {
        return "fixture root tree does not start with a canonical directory".to_owned();
    }));
    let mut malformed = b"040000 ".to_vec();
    malformed.extend_from_slice(remainder);
    check_try!(
        write(fixture.repository.join("malformed-tree"), malformed)
            .map_err(|error| return format!("write malformed tree: {error}"))
    );
    let tree = check_try!(git_text(
        fixture.repository.as_path(),
        &[
            "hash-object",
            "--literally",
            "-t",
            "tree",
            "-w",
            "malformed-tree"
        ]
    ));
    return commit_malformed_tree(fixture, tree.as_str());
}

/// Run one fixture Git command and return exact standard-output bytes.
///
/// # Errors
///
/// Returns an error when Git cannot start or exits unsuccessfully.
fn git_bytes(repository: &Path, args: &[&str]) -> CheckResult<Vec<u8>> {
    let output = check_try!(
        Command::new("git")
            .args(args)
            .current_dir(repository)
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
    return Ok(output.stdout);
}

/// Commit an evolving historical surface followed by the reviewed baseline.
///
/// # Errors
///
/// Returns an error when source copying, staging, or either commit fails.
pub(super) fn initialize_public_history(repository: &Path) -> CheckResult {
    check_try!(copy_public_tree(repository));
    check_try!(run_git(repository, &["add", "README.md"]));
    check_try!(commit_fixture(
        repository,
        "historical evolving public files",
    ));
    check_try!(run_git(repository, &["add", "--all"]));
    return commit_fixture(repository, "public baseline");
}

/// Verify raw zero-padded directory modes cannot hide behind recursive tree output.
///
/// # Errors
///
/// Returns an error when a malformed raw tree is not rejected by the Rust parser.
#[test]
fn push_snapshot_rejects_malformed_raw_tree() -> CheckResult {
    let fixture = check_try!(create_fixture("malformed-tree"));
    let commit = check_try!(create_malformed_tree_commit(&fixture));
    let input = record(
        "refs/heads/malformed-tree",
        commit.as_str(),
        "refs/heads/malformed-tree",
        fixture.zero.as_str(),
    );
    let result = check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str());
    let Err(error) = result else {
        return Err("push scanner accepted a zero-padded raw tree mode".to_owned());
    };
    if !error.contains("raw Git tree entry") || !error.contains("invalid mode") {
        return Err(format!("raw tree parser did not reject the mode: {error}"));
    }
    return remove_fixture(&fixture);
}

/// Verify Git replacement refs cannot conceal stored secret blob bytes.
///
/// # Errors
///
/// Returns an error when the scanner follows a local replacement ref.
#[test]
fn push_snapshot_rejects_replaced_secret_blob() -> CheckResult {
    let fixture = check_try!(create_fixture("replace-secret"));
    check_try!(write_secret_readme(
        fixture.repository.as_path(),
        "replace-hidden secret"
    ));
    let secret = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD:README.md"]
    ));
    check_try!(
        write(
            fixture.repository.join("safe-replacement"),
            "public replacement\n"
        )
        .map_err(|error| return format!("write replacement blob: {error}"))
    );
    let safe = check_try!(git_text(
        fixture.repository.as_path(),
        &["hash-object", "-w", "safe-replacement"]
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["replace", secret.as_str(), safe.as_str()]
    ));
    let commit = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let input = record(
        "refs/heads/main",
        commit.as_str(),
        "refs/heads/replaced-secret",
        fixture.zero.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str()),
        "secret blob hidden by git replace"
    ));
    return remove_fixture(&fixture);
}

/// Verify the actual push location wins over a divergent configured fetch URL.
///
/// # Errors
///
/// Returns an error when the scanner queries the named remote's decoy URL.
#[test]
fn push_snapshot_uses_actual_push_location() -> CheckResult {
    let fixture = check_try!(create_fixture("push-location"));
    check_try!(append_safe_readme(fixture.repository.as_path()));
    let commit = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "HEAD"]
    ));
    let input = record(
        "refs/heads/main",
        commit.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    check_try!(require_rejected(
        &check_input_in(fixture.repository.as_path(), "fixture", input.as_str()),
        "decoy fetch URL as the push destination"
    ));
    check_try!(check_input_in(
        fixture.repository.as_path(),
        PUSH_LOCATION,
        input.as_str()
    ));
    return remove_fixture(&fixture);
}

/// Render one canonical pre-push standard-input record.
pub(super) fn record(local_ref: &str, local: &str, remote_ref: &str, remote: &str) -> String {
    return format!("{local_ref} {local} {remote_ref} {remote}\n");
}

/// Remove one fixture after a successful scenario.
///
/// # Errors
///
/// Returns an error when fixture cleanup fails.
pub(super) fn remove_fixture(fixture: &PushFixture) -> CheckResult {
    return remove_dir_all(fixture.root.as_path())
        .map_err(|error| return format!("clear fixture: {error}"));
}

/// Require one proposed push to be rejected.
///
/// # Errors
///
/// Returns an error when the scanner unexpectedly accepts the proposal.
pub(super) fn require_rejected(result: &CheckResult, label: &str) -> CheckResult {
    if result.is_ok() {
        return Err(format!("push scanner accepted {label}"));
    }
    return Ok(());
}

/// Run one fixture Git command with no shell expansion.
///
/// # Errors
///
/// Returns an error when Git cannot start or exits unsuccessfully.
pub(super) fn run_git(repository: &Path, args: &[&str]) -> CheckResult {
    let status = check_try!(
        Command::new("git")
            .args(args)
            .current_dir(repository)
            .status()
            .map_err(|error| return format!("run fixture git {}: {error}", args.join(" ")),)
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("fixture git {} failed with {status}", args.join(" ")));
}

/// Regenerate and stage a fixture's data-only tracked-path binding.
///
/// # Errors
///
/// Returns an error when Git paths, rendering, writing, or staging fails.
pub(super) fn synchronize_fixture_public_tree(repository: &Path) -> CheckResult {
    let paths = check_try!(git_tracked_files(repository))
        .into_iter()
        .collect::<BTreeSet<_>>();
    let rendered = check_try!(render_public_tree_policy(&paths));
    check_try!(
        write(repository.join(PUBLIC_TREE_POLICY_PATH), rendered)
            .map_err(|error| return format!("write fixture public-tree policy: {error}"))
    );
    return run_git(repository, &["add", "--", PUBLIC_TREE_POLICY_PATH]);
}

/// Require a blob-backed tag target to fail closed.
///
/// # Errors
///
/// Returns an error when fixture setup fails or the noncommit target is accepted.
pub(super) fn verify_noncommit_target(fixture: &PushFixture) -> CheckResult {
    check_try!(
        write(fixture.repository.join("blob-fixture"), "public blob\n")
            .map_err(|error| return format!("write blob: {error}"))
    );
    let blob = check_try!(git_text(
        fixture.repository.as_path(),
        &["hash-object", "-w", "blob-fixture"]
    ));
    check_try!(run_git(
        fixture.repository.as_path(),
        &["update-ref", "refs/tags/blob", blob.as_str()]
    ));
    let input = record(
        "refs/tags/blob",
        blob.as_str(),
        "refs/tags/blob",
        fixture.zero.as_str(),
    );
    return require_rejected(
        &check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str()),
        "noncommit tag target",
    );
}

/// Verify one ordinary safe branch update.
///
/// # Errors
///
/// Returns an error when a canonical public commit is rejected.
pub(super) fn verify_safe_branch(fixture: &PushFixture) -> CheckResult {
    check_try!(append_safe_readme(fixture.repository.as_path()));
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
    return check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str());
}

/// Verify one all-zero local deletion record.
///
/// # Errors
///
/// Returns an error when a coherent deletion record is rejected.
pub(super) fn verify_safe_deletion(fixture: &PushFixture) -> CheckResult {
    let input = record(
        "(delete)",
        fixture.zero.as_str(),
        "refs/heads/main",
        fixture.baseline.as_str(),
    );
    return check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str());
}

/// Verify a new lightweight tag to an already-remote commit has no new objects.
///
/// # Errors
///
/// Returns an error when the safe lightweight tag is rejected.
pub(super) fn verify_safe_lightweight_tag(fixture: &PushFixture) -> CheckResult {
    check_try!(run_git(
        fixture.repository.as_path(),
        &["tag", "safe-lightweight", fixture.baseline.as_str()]
    ));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "refs/tags/safe-lightweight"]
    ));
    let input = record(
        "refs/tags/safe-lightweight",
        target.as_str(),
        "refs/tags/safe-lightweight",
        fixture.zero.as_str(),
    );
    return check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str());
}

/// Verify a new annotated tag's own tag object is scanned and accepted.
///
/// # Errors
///
/// Returns an error when tag creation or scanning fails.
pub(super) fn verify_safe_tag(fixture: &PushFixture) -> CheckResult {
    check_try!(run_git(
        fixture.repository.as_path(),
        &[
            "-c",
            "tag.gpgSign=false",
            "-c",
            "core.hooksPath=.git/tovuk-empty-hooks",
            "tag",
            "--annotate",
            "--message",
            "safe tag",
            "safe-tag"
        ]
    ));
    let target = check_try!(git_text(
        fixture.repository.as_path(),
        &["rev-parse", "refs/tags/safe-tag"]
    ));
    let input = record(
        "refs/tags/safe-tag",
        target.as_str(),
        "refs/tags/safe-tag",
        fixture.zero.as_str(),
    );
    return check_input_in(fixture.repository.as_path(), PUSH_LOCATION, input.as_str());
}

/// Replace README bytes with a reconstructed credential fixture and commit it.
///
/// # Errors
///
/// Returns an error when the credential fixture cannot be committed.
pub(super) fn write_secret_readme(repository: &Path, message: &str) -> CheckResult {
    let credential = format!("{}{}", ["gh", "p_"].concat(), "aB3".repeat(0x000c));
    check_try!(
        write(repository.join("README.md"), format!("{credential}\n"))
            .map_err(|error| return format!("write secret fixture: {error}"))
    );
    check_try!(run_git(repository, &["add", "README.md"]));
    return commit_fixture(repository, message);
}
