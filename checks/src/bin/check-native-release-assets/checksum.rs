//! Bounded checksum-sidecar and native-asset verification.

use core::fmt::Write as _;

use sha2::{Digest as _, Sha256};

use std::{
    ffi::OsStr,
    fs::{File, metadata},
    io::Read as _,
    path::Path,
};

use tovuk_public_checks::check_support::CheckResult;

/// Largest accepted native checksum sidecar.
pub(super) const MAX_CHECKSUM_BYTES: u64 = 0x1000;

/// Largest accepted native release binary.
pub(super) const MAX_NATIVE_ASSET_BYTES: u64 = 0x640_0000;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&hex_lower),
    size_of_val(&read_limited_text),
    size_of_val(&require_listed_asset),
    size_of_val(&sha256_file),
    size_of_val(&validate_asset_size),
    size_of_val(&verify_asset_checksum),
];

/// Encode bytes as lowercase hexadecimal.
///
/// # Errors
///
/// Returns an error when formatting into the output string fails.
fn hex_lower(bytes: &[u8]) -> CheckResult<String> {
    let mut encoded = String::new();
    for byte in bytes {
        check_try!(
            write!(encoded, "{byte:02x}")
                .map_err(|error| return format!("encode SHA-256: {error}"))
        );
    }
    return Ok(encoded);
}

/// Read bounded UTF-8 text without allocating beyond the public limit.
///
/// # Errors
///
/// Returns an error when the file cannot be read as UTF-8, changes while read,
/// or exceeds the configured byte limit.
pub(super) fn read_limited_text(path: &Path, maximum: u64, label: &str) -> CheckResult<String> {
    let limit = check_try!(
        maximum
            .checked_add(0x1)
            .ok_or_else(|| return format!("{label} limit overflow"))
    );
    let file = check_try!(
        File::open(path).map_err(|error| return format!("open {}: {error}", path.display()))
    );
    let mut source = String::new();
    let read_count = check_try!(
        file.take(limit)
            .read_to_string(&mut source)
            .map_err(|error| return format!("read {}: {error}", path.display()))
    );
    if read_count != source.len() {
        return Err(format!("{} changed while it was read", path.display()));
    }
    let length = check_try!(
        u64::try_from(source.len())
            .map_err(|error| return format!("measure {}: {error}", path.display()))
    );
    if length > maximum {
        return Err(format!(
            "{} exceeds the {maximum}-byte {label} limit",
            path.display()
        ));
    }
    return Ok(source);
}

/// Require a sidecar's optional filename to match the downloaded asset.
///
/// # Errors
///
/// Returns an error when the listed filename is not UTF-8 or does not match.
fn require_listed_asset(listed: &str, asset_name: &str) -> CheckResult {
    let listed_path = Path::new(listed.trim_start_matches('*'));
    let listed_asset = check_try!(
        listed_path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| return format!("{asset_name}.sha256 has an invalid asset name"))
    );
    if listed_asset != asset_name {
        return Err(format!(
            "{asset_name}.sha256 names {listed_asset}, expected {asset_name}"
        ));
    }
    return Ok(());
}

/// Stream and hash a bounded, nonempty native release asset.
///
/// # Errors
///
/// Returns an error when the asset is empty, exceeds 100 `MiB`, changes beyond the
/// bound while read, or cannot be read.
pub(super) fn sha256_file(path: &Path) -> CheckResult<String> {
    check_try!(validate_asset_size(path));
    let mut file = check_try!(
        File::open(path).map_err(|error| return format!("open {}: {error}", path.display()))
    );
    let mut digest = Sha256::new();
    let mut total = u64::MIN;
    let mut buffer = [0; 0x800];
    loop {
        let read_count = check_try!(
            file.read(&mut buffer)
                .map_err(|error| return format!("read {}: {error}", path.display()))
        );
        if read_count == usize::MIN {
            break;
        }
        let chunk = check_try!(
            buffer
                .get(..read_count)
                .ok_or_else(|| return "native asset read exceeded its buffer".to_owned())
        );
        digest.update(chunk);
        let chunk_length = check_try!(
            u64::try_from(read_count)
                .map_err(|error| return format!("measure {} read: {error}", path.display()))
        );
        total = check_try!(total.checked_add(chunk_length).ok_or_else(|| {
            return format!("{} size overflow", path.display());
        }));
        if total > MAX_NATIVE_ASSET_BYTES {
            return Err(format!(
                "{} exceeds the {MAX_NATIVE_ASSET_BYTES}-byte asset limit",
                path.display()
            ));
        }
    }
    if total == u64::MIN {
        return Err(format!("{} became empty while it was read", path.display()));
    }
    return hex_lower(digest.finalize().as_ref());
}

/// Validate the initial nonempty and maximum native asset bounds.
///
/// # Errors
///
/// Returns an error when metadata cannot be read or its length is outside the
/// public asset bounds.
fn validate_asset_size(path: &Path) -> CheckResult {
    let file_metadata = check_try!(
        metadata(path).map_err(|error| return format!("stat {}: {error}", path.display()))
    );
    if file_metadata.len() == u64::MIN || file_metadata.len() > MAX_NATIVE_ASSET_BYTES {
        return Err(format!(
            "{} must be nonempty and at most {MAX_NATIVE_ASSET_BYTES} bytes",
            path.display()
        ));
    }
    return Ok(());
}

/// Verify one native asset against its bounded checksum sidecar.
///
/// # Errors
///
/// Returns an error when the sidecar is invalid or the bounded asset digest does
/// not match it.
pub(super) fn verify_asset_checksum(
    asset_path: &Path,
    checksum_path: &Path,
    asset_name: &str,
) -> CheckResult {
    let checksum_source = check_try!(read_limited_text(
        checksum_path,
        MAX_CHECKSUM_BYTES,
        "checksum sidecar",
    ));
    let line = check_try!(
        checksum_source
            .lines()
            .map(str::trim)
            .find(|candidate| return !candidate.is_empty())
            .ok_or_else(|| return format!("{asset_name}.sha256 is empty"))
    );
    let mut parts = line.split_whitespace();
    let digest = check_try!(
        parts
            .next()
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| return format!("{asset_name}.sha256 is empty"))
    );
    if digest.len() != 0x40 || !digest.bytes().all(|byte| return byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{asset_name}.sha256 does not contain a SHA-256 digest"
        ));
    }
    if let Some(listed) = parts.next() {
        check_try!(require_listed_asset(listed, asset_name));
    }
    if parts.next().is_some() {
        return Err(format!("{asset_name}.sha256 contains unexpected fields"));
    }
    let actual = check_try!(sha256_file(asset_path));
    if actual != digest {
        return Err(format!(
            "{asset_name} checksum mismatch: expected {digest}, got {actual}"
        ));
    }
    return Ok(());
}
