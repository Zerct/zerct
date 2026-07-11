//! Bounded native release asset verification.

use super::{MAX_CHECKSUM_BYTES, MAX_NATIVE_ASSET_BYTES, read_limited_text, sha256_file};
use std::{
    env::temp_dir,
    fs::{File, create_dir_all, remove_dir_all, write as write_file},
    path::{Path, PathBuf},
    process::id as process_id,
};

/// Compile-time references preserve the named test boundaries.
const _: [usize; 0x4] = [
    size_of_val(&fixture_directory),
    size_of_val(&rejects_oversized_and_empty_release_inputs),
    size_of_val(&require_asset_limits),
    size_of_val(&require_checksum_limit),
];

/// Return the isolated bounded-input fixture directory.
fn fixture_directory() -> PathBuf {
    return temp_dir().join(format!("tovuk-native-assets-test-{}", process_id()));
}

/// Verify checksum and native asset size limits fail closed.
///
/// # Errors
///
/// Returns an error when fixture setup, validation, or cleanup fails.
#[test]
fn rejects_oversized_and_empty_release_inputs() -> Result<(), String> {
    let directory = fixture_directory();
    if directory.exists() {
        check_try!(
            remove_dir_all(directory.as_path())
                .map_err(|error| return format!("remove {}: {error}", directory.display()))
        );
    }
    check_try!(
        create_dir_all(directory.as_path())
            .map_err(|error| return format!("create {}: {error}", directory.display()))
    );
    check_try!(require_asset_limits(&directory));
    check_try!(require_checksum_limit(&directory));
    check_try!(
        remove_dir_all(directory.as_path())
            .map_err(|error| return format!("remove {}: {error}", directory.display()))
    );
    return Ok(());
}

/// Verify nonempty and 100 `MiB` native asset bounds.
///
/// # Errors
///
/// Returns an error when fixtures cannot be prepared or a bound is violated.
fn require_asset_limits(directory: &Path) -> Result<(), String> {
    let valid_asset = directory.join("valid");
    check_try!(
        write_file(valid_asset.as_path(), b"abc")
            .map_err(|error| return format!("write {}: {error}", valid_asset.display()))
    );
    let digest = check_try!(sha256_file(valid_asset.as_path()));
    if digest != "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" {
        return Err(format!("unexpected streaming digest: {digest}"));
    }
    let empty_asset = directory.join("empty");
    check_try!(
        write_file(empty_asset.as_path(), [])
            .map_err(|error| return format!("write {}: {error}", empty_asset.display()))
    );
    check_try!(require_error(
        sha256_file(empty_asset.as_path()),
        "nonempty",
    ));
    let oversized_asset = directory.join("oversized");
    let file = check_try!(
        File::create(oversized_asset.as_path())
            .map_err(|error| return format!("create {}: {error}", oversized_asset.display()))
    );
    check_try!(
        file.set_len(MAX_NATIVE_ASSET_BYTES.saturating_add(0x1))
            .map_err(|error| return format!("resize {}: {error}", oversized_asset.display()))
    );
    return require_error(sha256_file(oversized_asset.as_path()), "at most");
}

/// Verify the exact 4 `KiB` checksum sidecar bound.
///
/// # Errors
///
/// Returns an error when fixtures cannot be prepared or a bound is violated.
fn require_checksum_limit(directory: &Path) -> Result<(), String> {
    let checksum = directory.join("asset.sha256");
    let maximum_length = check_try!(
        usize::try_from(MAX_CHECKSUM_BYTES)
            .map_err(|error| return format!("convert checksum fixture length: {error}"))
    );
    check_try!(
        write_file(checksum.as_path(), vec![b'a'; maximum_length])
            .map_err(|error| return format!("write {}: {error}", checksum.display()))
    );
    let maximum = check_try!(read_limited_text(
        checksum.as_path(),
        MAX_CHECKSUM_BYTES,
        "checksum",
    ));
    if maximum.len() != maximum_length {
        return Err("the exact checksum limit must be accepted".to_owned());
    }
    let oversized_length = check_try!(
        usize::try_from(MAX_CHECKSUM_BYTES.saturating_add(0x1))
            .map_err(|error| return format!("convert checksum fixture length: {error}"))
    );
    check_try!(
        write_file(checksum.as_path(), vec![b'a'; oversized_length])
            .map_err(|error| return format!("write {}: {error}", checksum.display()))
    );
    return require_error(
        read_limited_text(checksum.as_path(), MAX_CHECKSUM_BYTES, "checksum"),
        "exceeds",
    );
}

/// Require a validation operation to fail with a diagnostic fragment.
///
/// # Errors
///
/// Returns an error when validation succeeds or reports an unexpected error.
fn require_error<Value>(result: Result<Value, String>, expected: &str) -> Result<(), String> {
    let Err(message) = result else {
        return Err(format!(
            "validation unexpectedly succeeded; expected {expected}"
        ));
    };
    if !message.contains(expected) {
        return Err(format!("unexpected validation error: {message}"));
    }
    return Ok(());
}
