use crate::{
    helpers::CheckResult,
    repo_hygiene_paths::is_allowed_public_surface_path,
    repo_hygiene_text::{
        MAX_TRACKED_TEXT_BYTES, reject_private_implementation_terms, validate_tracked_text,
    },
};

use std::fs::{metadata as filesystem_metadata, read};

use tovuk_public_checks::check_support::reject_secret_signatures;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&reject_invalid_tracked_text_files),
    size_of_val(&reject_tracked_private_implementation_terms),
    size_of_val(&reject_tracked_secret_signatures),
    size_of_val(&reject_unapproved_public_surface_paths),
];

/// Reject noncanonical or oversized tracked public text files.
///
/// # Errors
///
/// Returns an error when tracked content is too large, unreadable, binary,
/// non-UTF-8, CRLF, unterminated, or has trailing whitespace.
pub(super) fn reject_invalid_tracked_text_files(tracked_files: &[String]) -> CheckResult {
    let mut invalid = Vec::new();
    for path in tracked_files {
        let metadata = check_try!(
            filesystem_metadata(path).map_err(|error| return format!("inspect {path}: {error}"))
        );
        if metadata.len() > MAX_TRACKED_TEXT_BYTES {
            invalid.push(format!(
                "{path} exceeds the {MAX_TRACKED_TEXT_BYTES}-byte tracked-file ceiling"
            ));
            continue;
        }
        let contents = check_try!(
            read(path).map_err(|error| return format!("read tracked file {path}: {error}"))
        );
        if let Err(error) = validate_tracked_text(path, contents.as_slice()) {
            invalid.push(error);
        }
    }
    if invalid.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Tracked public files violate canonical text policy:\n{}",
        invalid.join("\n")
    ));
}

/// Reject private implementation terminology in tracked paths or contents.
///
/// # Errors
///
/// Returns an error when a tracked path or file contains a fingerprinted term.
pub(super) fn reject_tracked_private_implementation_terms(tracked_files: &[String]) -> CheckResult {
    let mut findings = Vec::new();
    for path in tracked_files {
        if reject_private_implementation_terms("tracked path", path.as_bytes()).is_err() {
            findings.push(format!("{path}: path"));
        }
        let contents = check_try!(
            read(path).map_err(|error| return format!("read tracked file {path}: {error}"))
        );
        if reject_private_implementation_terms(path, contents.as_slice()).is_err() {
            findings.push(format!("{path}: contents"));
        }
    }
    if findings.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Tracked public files expose private implementation terminology:\n{}",
        findings.join("\n")
    ));
}

/// Reject recognized private-key or credential signatures in tracked files.
///
/// # Errors
///
/// Returns an error when any tracked UTF-8 file contains a known secret signature.
pub(super) fn reject_tracked_secret_signatures(tracked_files: &[String]) -> CheckResult {
    let mut findings = Vec::new();
    for path in tracked_files {
        let contents = check_try!(
            read(path).map_err(|error| return format!("read tracked file {path}: {error}"))
        );
        if let Err(error) = reject_secret_signatures(path, contents.as_slice()) {
            findings.push(error);
        }
    }
    if findings.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Tracked public files contain secret signatures:\n{}",
        findings.join("\n")
    ));
}

/// Reject tracked files outside the explicitly reviewed public repository surface.
///
/// # Errors
///
/// Returns an error when a new root file or top-level directory lacks policy review.
pub(super) fn reject_unapproved_public_surface_paths(tracked_files: &[String]) -> CheckResult {
    let unapproved = tracked_files
        .iter()
        .filter(|path| return !is_allowed_public_surface_path(path))
        .cloned()
        .collect::<Vec<_>>();
    if unapproved.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "These tracked paths are outside the reviewed public repository surface:\n{}",
        unapproved.join("\n")
    ));
}
