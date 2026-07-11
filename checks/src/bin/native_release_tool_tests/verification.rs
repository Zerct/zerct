//! Verification for the native release utility.

use super::{
    matrix_json, package_version, read_matrix, read_package_version, verify_sha256, write_sha256,
};
use std::{
    env::{self, consts::FAMILY},
    fs::{create_dir_all, read_to_string, remove_dir_all, symlink_metadata, write},
    path::{Path, PathBuf},
    process::{self, Command},
};

/// Compile-time references preserve the named test-helper boundary.
const _: [usize; 0x6] = [
    size_of_val(&create_symlink),
    size_of_val(&matrix_is_compact_and_preserves_tracked_runners),
    size_of_val(&package_version_rejects_invalid_contracts),
    size_of_val(&test_directory),
    size_of_val(&write_sha256_replaces_symlink_without_touching_target),
    size_of_val(&writes_and_verifies_sha256),
];

/// Create a symbolic link through the Unix `ln` utility.
///
/// # Errors
///
/// Returns an error when `ln` cannot run or create the link.
fn create_symlink(target: &Path, link: &Path) -> Result<(), String> {
    let status = check_try!(
        Command::new("ln")
            .arg("-s")
            .arg(target)
            .arg(link)
            .status()
            .map_err(|error| return format!("create checksum symlink: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("ln -s failed with status {status}"));
}

/// Verify that matrix JSON is compact and uses every runner from the manifest.
///
/// # Errors
///
/// Returns an error when the fixture cannot be prepared or the matrix is wrong.
#[test]
fn matrix_is_compact_and_preserves_tracked_runners() -> Result<(), String> {
    let directory = check_try!(test_directory("matrix"));
    let manifest_path = directory.join("native-release-targets.json");
    let crate_manifest_path = directory.join("Cargo.toml");
    check_try!(
        write(
            manifest_path.as_path(),
            r#"{"targets":[{"asset_ext":"","binary":"tovuk","runner":"ubuntu-24.04","triple":"x86_64-unknown-linux-gnu"},{"asset_ext":"","binary":"tovuk","runner":"ubuntu-24.04-arm","triple":"aarch64-unknown-linux-gnu"},{"asset_ext":"","binary":"tovuk","runner":"macos-15","triple":"aarch64-apple-darwin"}]}"#,
        )
        .map_err(|error| return format!("write fixture: {error}"))
    );
    check_try!(
        write(
            crate_manifest_path.as_path(),
            "[package]\nname = \"tovuk\"\nversion = \"1.2.3\" # release\n",
        )
        .map_err(|error| return format!("write crate fixture: {error}"))
    );
    let manifest = check_try!(read_matrix(manifest_path.as_path()));
    let version = check_try!(read_package_version(crate_manifest_path.as_path()));
    let actual = check_try!(matrix_json(manifest, version.as_str()));
    let expected = r#"{"include":[{"asset_ext":"","asset_name":"tovuk-1.2.3-x86_64-unknown-linux-gnu","binary":"tovuk","build_strategy":"cargo","build_target":"x86_64-unknown-linux-gnu","release_tag":"v1.2.3","runner":"ubuntu-24.04","target":"x86_64-unknown-linux-gnu"},{"asset_ext":"","asset_name":"tovuk-1.2.3-aarch64-unknown-linux-gnu","binary":"tovuk","build_strategy":"cargo-zigbuild","build_target":"aarch64-unknown-linux-gnu.2.17","release_tag":"v1.2.3","runner":"ubuntu-24.04-arm","target":"aarch64-unknown-linux-gnu"},{"asset_ext":"","asset_name":"tovuk-1.2.3-aarch64-apple-darwin","binary":"tovuk","build_strategy":"cargo","build_target":"aarch64-apple-darwin","release_tag":"v1.2.3","runner":"macos-15","target":"aarch64-apple-darwin"}]}"#;
    if actual != expected {
        return Err(format!("unexpected matrix: {actual}"));
    }
    check_try!(
        remove_dir_all(directory.as_path())
            .map_err(|error| return format!("remove {}: {error}", directory.display()))
    );
    return Ok(());
}

/// Verify that missing, empty, and duplicate package versions are rejected.
///
/// # Errors
///
/// Returns an error when an invalid package version is accepted or diagnosed
/// incorrectly.
#[test]
fn package_version_rejects_invalid_contracts() -> Result<(), String> {
    for (source, expected) in [
        ("[package]\nname = \"tovuk\"\n", "version is missing"),
        ("[package]\nversion = \"\"\n", "must not be empty"),
        (
            "[package]\nversion = \"1.2.3\"\nversion = \"1.2.4\"\n",
            "duplicate version",
        ),
    ] {
        check_try!(require_error_contains(package_version(source), expected));
    }
    return Ok(());
}

/// Require an operation to fail with a diagnostic fragment.
///
/// # Errors
///
/// Returns an error when the operation succeeds or its diagnostic omits the
/// expected fragment.
fn require_error_contains<Value>(
    result: Result<Value, String>,
    expected: &str,
) -> Result<(), String> {
    let Err(message) = result else {
        return Err(format!(
            "operation unexpectedly succeeded; expected {expected}"
        ));
    };
    if !message.contains(expected) {
        return Err(format!("unexpected error: {message}"));
    }
    return Ok(());
}

/// Return a deterministic temporary directory for one test process.
///
/// # Errors
///
/// Returns an error when an existing test directory cannot be removed or the
/// fresh directory cannot be created.
fn test_directory(label: &str) -> Result<PathBuf, String> {
    let path = env::temp_dir().join(format!(
        "tovuk-native-release-tool-{}-{label}",
        process::id()
    ));
    if path.exists() {
        check_try!(
            remove_dir_all(path.as_path())
                .map_err(|error| return format!("remove {}: {error}", path.display()))
        );
    }
    check_try!(
        create_dir_all(path.as_path())
            .map_err(|error| return format!("create {}: {error}", path.display()))
    );
    return Ok(path);
}

/// Verify atomic publication replaces a checksum symlink, not its target.
///
/// # Errors
///
/// Returns an error when the fixture cannot be prepared or symlink-safe
/// publication fails.
#[test]
fn write_sha256_replaces_symlink_without_touching_target() -> Result<(), String> {
    if FAMILY != "unix" {
        return Ok(());
    }
    let directory = check_try!(test_directory("checksum-symlink"));
    let asset_path = directory.join("tovuk-test-asset");
    let checksum_path = directory.join("tovuk-test-asset.sha256");
    let unrelated_path = directory.join("unrelated-file");
    check_try!(
        write(asset_path.as_path(), b"abc")
            .map_err(|error| return format!("write asset fixture: {error}"))
    );
    check_try!(
        write(unrelated_path.as_path(), b"unchanged")
            .map_err(|error| return format!("write unrelated fixture: {error}"))
    );
    check_try!(create_symlink(&unrelated_path, &checksum_path));
    if check_try!(write_sha256(asset_path.as_path())) != checksum_path {
        return Err("checksum published at an unexpected path".to_owned());
    }
    let unrelated = check_try!(
        read_to_string(unrelated_path.as_path())
            .map_err(|error| return format!("read unrelated fixture: {error}"))
    );
    if unrelated != "unchanged" {
        return Err("checksum publication followed its destination symlink".to_owned());
    }
    let metadata = check_try!(
        symlink_metadata(checksum_path.as_path())
            .map_err(|error| return format!("inspect checksum sidecar: {error}"))
    );
    if metadata.file_type().is_symlink() {
        return Err("checksum sidecar remained a symlink".to_owned());
    }
    check_try!(
        remove_dir_all(directory.as_path())
            .map_err(|error| return format!("remove {}: {error}", directory.display()))
    );
    return Ok(());
}

/// Verify exact sidecar output and case-insensitive expected digest checking.
///
/// # Errors
///
/// Returns an error when the fixture cannot be prepared or checksum behavior is
/// incorrect.
#[test]
fn writes_and_verifies_sha256() -> Result<(), String> {
    let directory = check_try!(test_directory("checksum"));
    let asset_path = directory.join("tovuk-test-asset");
    check_try!(
        write(asset_path.as_path(), b"abc")
            .map_err(|error| return format!("write fixture: {error}"))
    );
    let checksum_path = check_try!(write_sha256(asset_path.as_path()));
    let checksum = check_try!(
        read_to_string(checksum_path.as_path())
            .map_err(|error| return format!("read checksum: {error}"))
    );
    let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let expected = format!("{digest}  tovuk-test-asset\n");
    if checksum != expected {
        return Err(format!("unexpected checksum contents: {checksum:?}"));
    }
    check_try!(verify_sha256(
        asset_path.as_path(),
        digest.to_ascii_uppercase().as_str(),
    ));
    check_try!(require_error_contains(
        verify_sha256(
            asset_path.as_path(),
            "aa7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        "checksum mismatch",
    ));
    check_try!(require_error_contains(
        verify_sha256(asset_path.as_path(), "not-a-sha256-digest"),
        "exactly 64 hexadecimal characters",
    ));
    check_try!(
        remove_dir_all(directory.as_path())
            .map_err(|error| return format!("remove {}: {error}", directory.display()))
    );
    return Ok(());
}
