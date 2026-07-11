//! Synthetic package archive verification tests.

use std::{fs::remove_dir_all, path::Path};

use tovuk_public_checks::check_support::CheckResult;

use super::{
    archive::read_tar_gz,
    npm_package,
    package_artifact_fixtures::{
        VERSION, artifact_request, symlink_entry, test_directory, text_entry, write_npm, write_tar,
        write_zip,
    },
    run,
    zip_archive::read_zip,
};

/// Compile-time references preserve the named test-helper boundary.
const _: [usize; 0x0004] = [
    size_of_val(&accepts_complete_synthetic_artifacts),
    size_of_val(&rejects_duplicate_and_traversal_zip_paths),
    size_of_val(&rejects_symlink_and_sensitive_members),
    size_of_val(&rejects_unsynchronized_package_metadata),
];

/// Verify a complete synchronized synthetic release artifact set.
///
/// # Errors
///
/// Returns an error when fixture creation, verification, or cleanup fails.
#[test]
fn accepts_complete_synthetic_artifacts() -> CheckResult {
    let directory = check_try!(test_directory("valid"));
    let request = check_try!(artifact_request(directory.as_path()));
    let result = run(&request);
    check_try!(remove_fixture(directory.as_path()));
    return result;
}

/// Verify duplicate and traversal ZIP paths are rejected before extraction.
///
/// # Errors
///
/// Returns an error when fixture I/O fails or an unsafe archive is accepted.
#[test]
fn rejects_duplicate_and_traversal_zip_paths() -> CheckResult {
    let directory = check_try!(test_directory("paths"));
    let duplicate = directory.join("duplicate.whl");
    let entries = vec![text_entry("same", "one"), text_entry("same", "two")];
    check_try!(write_zip(duplicate.as_path(), entries.as_slice()));
    check_try!(require_error(
        read_zip(duplicate.as_path(), "test"),
        "duplicate",
    ));
    let traversal = directory.join("traversal.whl");
    check_try!(write_zip(
        traversal.as_path(),
        &[text_entry("../escape", "secret")],
    ));
    check_try!(require_error(
        read_zip(traversal.as_path(), "test"),
        "unsafe",
    ));
    return remove_fixture(directory.as_path());
}

/// Verify ZIP symlinks and secret local configuration paths are rejected.
///
/// # Errors
///
/// Returns an error when fixture I/O fails or an unsafe member is accepted.
#[test]
fn rejects_symlink_and_sensitive_members() -> CheckResult {
    let directory = check_try!(test_directory("sensitive"));
    let symlink = directory.join("symlink.whl");
    check_try!(write_zip(
        symlink.as_path(),
        &[symlink_entry("link", "target")],
    ));
    check_try!(require_error(
        read_zip(symlink.as_path(), "test"),
        "symlink",
    ));
    let local = directory.join("local.tgz");
    check_try!(write_tar(
        local.as_path(),
        &[text_entry("package/.env", "TOKEN=secret")],
    ));
    check_try!(require_error(
        read_tar_gz(local.as_path(), "test"),
        "forbidden",
    ));
    return remove_fixture(directory.as_path());
}

/// Verify packaged npm metadata cannot drift from the expected version.
///
/// # Errors
///
/// Returns an error when fixture I/O fails or drifted metadata is accepted.
#[test]
fn rejects_unsynchronized_package_metadata() -> CheckResult {
    let directory = check_try!(test_directory("metadata"));
    let archive = directory.join(format!("tovuk-{VERSION}.tgz"));
    check_try!(write_npm(archive.as_path(), VERSION, "9.9.9"));
    check_try!(require_error(
        npm_package::validate(archive.as_path(), VERSION),
        "version must be",
    ));
    return remove_fixture(directory.as_path());
}

/// Remove one synthetic fixture directory.
///
/// # Errors
///
/// Returns an error when recursive removal fails.
fn remove_fixture(directory: &Path) -> CheckResult {
    return remove_dir_all(directory)
        .map_err(|error| return format!("remove {}: {error}", directory.display()));
}

/// Require one operation to fail with a diagnostic fragment.
///
/// # Errors
///
/// Returns an error when the operation succeeds or reports another finding.
fn require_error<Value>(result: CheckResult<Value>, expected: &str) -> CheckResult {
    let Err(error) = result else {
        return Err(format!(
            "operation unexpectedly succeeded; expected {expected}"
        ));
    };
    return error
        .contains(expected)
        .then_some(())
        .ok_or_else(|| return format!("unexpected error {error:?}; expected {expected}"));
}
