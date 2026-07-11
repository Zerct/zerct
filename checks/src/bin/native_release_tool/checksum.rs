//! Atomic SHA-256 checksum creation and verification.

use core::fmt::Write as _;

use sha2::{Digest as _, Sha256};

use std::{
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions, remove_file, rename},
    io::{Error as InputOutputError, Read as _, Write as _},
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0008] = [
    size_of_val(&checksum_path),
    size_of_val(&cleanup_temporary_file),
    size_of_val(&create_temporary_file),
    size_of_val(&publish_checksum),
    size_of_val(&sha256_file),
    size_of_val(&validate_digest),
    size_of_val(&verify_sha256),
    size_of_val(&write_sha256),
];

/// Open temporary checksum file paired with its unique path.
type TemporaryChecksumFile = (File, PathBuf);

/// Return the sidecar path formed by appending `.sha256` to an asset path.
fn checksum_path(asset: &Path) -> PathBuf {
    let mut checksum_name = OsString::from(asset.as_os_str());
    checksum_name.push(".sha256");
    return PathBuf::from(checksum_name);
}

/// Remove a failed temporary publication and preserve both diagnostics.
fn cleanup_temporary_file(
    temporary_path: &Path,
    operation: &str,
    error: &InputOutputError,
) -> String {
    let operation_error = format!(
        "{operation} temporary checksum {}: {error}",
        temporary_path.display()
    );
    return match remove_file(temporary_path) {
        Ok(()) => operation_error,
        Err(cleanup_error) => format!(
            "{operation_error}; remove temporary checksum {}: {cleanup_error}",
            temporary_path.display()
        ),
    };
}

/// Create a collision-safe temporary checksum file beside its destination.
///
/// # Errors
///
/// Returns an error when a unique file cannot be created with `create_new`.
fn create_temporary_file(destination: &Path) -> Result<TemporaryChecksumFile, String> {
    let parent = destination
        .parent()
        .filter(|path| return !path.as_os_str().is_empty())
        .unwrap_or_else(|| return Path::new("."));
    let basename = check_try!(
        destination
            .file_name()
            .ok_or_else(|| return format!("{} must have a basename", destination.display()))
    );
    let timestamp = check_try!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| return format!("system time before Unix epoch: {error}"))
    )
    .as_nanos();
    let mut last_error = None;
    for attempt in u8::MIN..0x20 {
        let mut temporary_name = OsString::from(basename);
        temporary_name.push(format!(".tmp-{}-{timestamp}-{attempt}", process::id()));
        let temporary_path = parent.join(temporary_name);
        let mut options = OpenOptions::new();
        let configured_options = options.write(true).create_new(true);
        match configured_options.open(temporary_path.as_path()) {
            Ok(file) => return Ok((file, temporary_path)),
            Err(error) => last_error = Some((temporary_path, error)),
        }
    }
    let Some((failed_path, open_error)) = last_error else {
        return Err("temporary checksum attempt range must not be empty".to_owned());
    };
    return Err(format!(
        "create temporary checksum {}: {open_error}",
        failed_path.display()
    ));
}

/// Publish checksum contents atomically without following the destination.
///
/// # Errors
///
/// Returns an error when temporary file creation, writing, synchronization, or
/// atomic renaming fails. Failed temporary files are removed.
fn publish_checksum(destination: &Path, contents: &[u8]) -> Result<(), String> {
    let (mut temporary_file, temporary_path) = check_try!(create_temporary_file(destination));
    if let Err(error) = temporary_file.write_all(contents) {
        drop(temporary_file);
        return Err(cleanup_temporary_file(
            temporary_path.as_path(),
            "write",
            &error,
        ));
    }
    if let Err(error) = temporary_file.sync_all() {
        drop(temporary_file);
        return Err(cleanup_temporary_file(
            temporary_path.as_path(),
            "sync",
            &error,
        ));
    }
    drop(temporary_file);
    return match rename(temporary_path.as_path(), destination) {
        Ok(()) => Ok(()),
        Err(error) => Err(cleanup_temporary_file(
            temporary_path.as_path(),
            "rename",
            &error,
        )),
    };
}

/// Calculate the lowercase SHA-256 digest for a file.
///
/// # Errors
///
/// Returns an error when the file cannot be read.
fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = check_try!(
        File::open(path).map_err(|error| return format!("open {}: {error}", path.display()))
    );
    let mut hasher = Sha256::new();
    let mut buffer = [0; 0x800];
    loop {
        let bytes_read = check_try!(
            file.read(&mut buffer)
                .map_err(|error| return format!("read {}: {error}", path.display()))
        );
        if bytes_read == 0x0 {
            break;
        }
        let chunk = check_try!(
            buffer
                .get(..bytes_read)
                .ok_or_else(|| return "SHA-256 read exceeded its buffer".to_owned())
        );
        hasher.update(chunk);
    }
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(0x40);
    for byte in digest {
        check_try!(
            write!(encoded, "{byte:02x}")
                .map_err(|error| return format!("encode SHA-256: {error}"))
        );
    }
    return Ok(encoded);
}

/// Validate and normalize an expected SHA-256 digest.
///
/// # Errors
///
/// Returns an error when the digest is not exactly 64 hexadecimal characters.
fn validate_digest(expected: &str) -> Result<String, String> {
    let valid = expected.len() == 0x40 && expected.as_bytes().iter().all(u8::is_ascii_hexdigit);
    if !valid {
        return Err("expected SHA-256 must contain exactly 64 hexadecimal characters".to_owned());
    }
    return Ok(expected.to_ascii_lowercase());
}

/// Verify a file against an expected SHA-256 digest.
///
/// # Errors
///
/// Returns an error when the expected digest is invalid, the file cannot be read,
/// or the calculated digest does not match.
pub(super) fn verify_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let normalized_expected = check_try!(validate_digest(expected));
    let actual = check_try!(sha256_file(path));
    if actual != normalized_expected {
        return Err(format!(
            "{} checksum mismatch: expected {normalized_expected}, got {actual}",
            path.display()
        ));
    }
    return Ok(());
}

/// Write an asset checksum to `<asset>.sha256` in standard checksum format.
///
/// # Errors
///
/// Returns an error when the asset name is not UTF-8 or a file cannot be read or
/// written.
pub(super) fn write_sha256(asset: &Path) -> Result<PathBuf, String> {
    let basename = check_try!(
        asset
            .file_name()
            .and_then(OsStr::to_str)
            .filter(|name| return !name.is_empty())
            .ok_or_else(|| return format!("{} must have a UTF-8 basename", asset.display()))
    );
    let digest = check_try!(sha256_file(asset));
    let path = checksum_path(asset);
    let contents = format!("{digest}  {basename}\n");
    check_try!(publish_checksum(path.as_path(), contents.as_bytes()));
    return Ok(path);
}
