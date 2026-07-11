//! Tests for shared check support.

use std::{fs::metadata as filesystem_metadata, path::Path};

use super::{
    CHECKS_MANIFEST, CheckResult, command, display_path, find_command, git_tracked_files,
    repo_root, run_status, tool_path,
};

/// Verify repository discovery and tracked-file rendering.
///
/// # Errors
///
/// Returns an error when the helpers cannot inspect the current repository.
#[test]
fn repository_helpers_find_manifest() -> CheckResult {
    let repository = check_try!(repo_root());
    let path = tool_path();
    let git = check_try!(find_command(path.as_os_str(), &["git"]));
    let prepared_command = command(&repository, path.as_os_str(), "git");
    if prepared_command.get_current_dir() != Some(repository.as_path()) {
        return Err("shared command helper did not set its working directory".to_owned());
    }
    check_try!(run_status(
        &repository,
        path.as_os_str(),
        "git",
        &["--version"]
    ));
    let tracked_files = check_try!(git_tracked_files(&repository));
    if !tracked_files
        .iter()
        .any(|file| return file == CHECKS_MANIFEST)
    {
        return Err(format!("Git does not track {CHECKS_MANIFEST}"));
    }
    if display_path(Path::new(CHECKS_MANIFEST)) != CHECKS_MANIFEST {
        return Err("display_path changed a repository-relative path".to_owned());
    }
    if filesystem_metadata(git).is_err() {
        return Err("find_command returned an unreadable path".to_owned());
    }
    return Ok(());
}

/// Verify the same helpers remain stable across a second invocation.
///
/// # Errors
///
/// Returns an error when repeated helper use changes the observed repository.
#[test]
fn repository_helpers_verify_repeatability() -> CheckResult {
    let first_repository = check_try!(repo_root());
    let path = tool_path();
    let git = check_try!(find_command(path.as_os_str(), &["git"]));
    let prepared_command = command(&first_repository, path.as_os_str(), "git");
    if prepared_command.get_program() != "git" {
        return Err("shared command helper changed the program".to_owned());
    }
    check_try!(run_status(
        &first_repository,
        path.as_os_str(),
        "git",
        &["rev-parse", "--is-inside-work-tree"]
    ));
    let tracked_files = check_try!(git_tracked_files(&first_repository));
    if tracked_files.is_empty() {
        return Err("Git returned no tracked files".to_owned());
    }
    if display_path(Path::new(CHECKS_MANIFEST)).is_empty() {
        return Err("display_path returned an empty path".to_owned());
    }
    if filesystem_metadata(git).is_err() {
        return Err("find_command returned an unreadable path".to_owned());
    }
    return Ok(());
}
