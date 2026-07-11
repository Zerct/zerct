//! Tests for shared check support.

use std::{
    env::join_paths,
    ffi::OsString,
    fs::{create_dir_all, metadata as filesystem_metadata, remove_dir_all, write},
    path::{Path, PathBuf},
    process,
};

use super::{
    CHECKS_MANIFEST, CheckResult, command, display_path, find_command, git_tracked_files,
    repo_root, run_status, tool_path,
};

/// Paths created for the command-precedence regression test.
#[derive(Debug)]
struct CandidateFixture {
    /// Expected fallback candidate path.
    fallback: PathBuf,
    /// Synthetic PATH containing the candidates.
    path: OsString,
    /// Expected preferred candidate path.
    preferred: PathBuf,
    /// Fixture root removed after the test.
    root: PathBuf,
}

/// Create two command candidates whose name and directory priorities conflict.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created.
fn candidate_fixture(label: &str) -> CheckResult<CandidateFixture> {
    let root = PathBuf::from("target")
        .join("support-tests")
        .join(format!("candidate-order-{}-{label}", process::id()));
    if check_try!(
        root.try_exists()
            .map_err(|error| return format!("inspect {}: {error}", root.display()))
    ) {
        check_try!(
            remove_dir_all(root.as_path())
                .map_err(|error| return format!("clear {}: {error}", root.display()))
        );
    }
    let fallback_directory = root.join("fallback-first");
    let preferred_directory = root.join("preferred-later");
    let fallback = check_try!(write_candidate(fallback_directory.as_path(), "fallback"));
    let preferred = check_try!(write_candidate(preferred_directory.as_path(), "preferred"));
    let path = check_try!(
        join_paths([fallback_directory, preferred_directory])
            .map_err(|error| return format!("join fixture PATH: {error}"))
    );
    let fixture = CandidateFixture {
        fallback,
        path,
        preferred,
        root,
    };
    return Ok(fixture);
}

/// Verify preferred command names win even when their directory is later in PATH.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created or candidate ordering is wrong.
#[test]
fn command_discovery_prioritizes_candidate_order() -> CheckResult {
    let fixture = check_try!(candidate_fixture("preferred"));
    let discovery = find_command(fixture.path.as_os_str(), &["preferred", "fallback"]);
    let cleanup = remove_dir_all(fixture.root.as_path());
    check_try!(cleanup.map_err(|error| return format!("clear fixture: {error}")));
    let discovered = check_try!(discovery);
    if discovered != fixture.preferred {
        return Err(format!(
            "found {}, expected preferred candidate {}",
            discovered.display(),
            fixture.preferred.display()
        ));
    }
    return Ok(());
}

/// Verify command discovery falls back only after preferred names are exhausted.
///
/// # Errors
///
/// Returns an error when the fixture cannot be created or fallback discovery is wrong.
#[test]
fn command_discovery_uses_fallback_after_preferred_names() -> CheckResult {
    let fixture = check_try!(candidate_fixture("fallback"));
    let discovery = find_command(fixture.path.as_os_str(), &["missing", "fallback"]);
    let cleanup = remove_dir_all(fixture.root.as_path());
    check_try!(cleanup.map_err(|error| return format!("clear fixture: {error}")));
    let discovered = check_try!(discovery);
    if discovered != fixture.fallback {
        return Err(format!(
            "found {}, expected fallback candidate {}",
            discovered.display(),
            fixture.fallback.display()
        ));
    }
    return Ok(());
}

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

/// Create one regular command-candidate fixture file.
///
/// # Errors
///
/// Returns an error when the directory or file cannot be created.
fn write_candidate(directory: &Path, name: &str) -> CheckResult<PathBuf> {
    check_try!(
        create_dir_all(directory)
            .map_err(|error| return format!("create {}: {error}", directory.display()))
    );
    let candidate = directory.join(name);
    check_try!(
        write(candidate.as_path(), [])
            .map_err(|error| return format!("write {}: {error}", candidate.display()))
    );
    return Ok(candidate);
}
