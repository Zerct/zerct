//! Verify publishable Cargo, npm, and Python package archives before release.

/// Propagate a failed artifact check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

extern crate alloc;

/// Bounded archive readers and archive-member invariants.
#[path = "check-package-artifacts/archive.rs"]
pub mod archive;
/// Cargo package artifact policy.
#[path = "check-package-artifacts/cargo_package.rs"]
pub mod cargo_package;
/// npm package artifact policy.
#[path = "check-package-artifacts/npm_package.rs"]
pub mod npm_package;
/// Synthetic package artifact fixtures.
#[cfg(test)]
#[path = "check_package_artifacts_tests/fixtures.rs"]
pub mod package_artifact_fixtures;
/// Shared package metadata and leakage policy.
#[path = "check-package-artifacts/policy.rs"]
pub mod policy;
/// Python wheel and source-distribution artifact policy.
#[path = "check-package-artifacts/python_package.rs"]
pub mod python_package;
/// Synthetic package archive tests.
#[cfg(test)]
#[path = "check_package_artifacts_tests/verification.rs"]
mod tests;
/// Bounded wheel ZIP reader.
#[path = "check-package-artifacts/zip_archive.rs"]
pub mod zip_archive;
/// ZIP central-directory and local-header validation.
#[path = "check-package-artifacts/zip_directory.rs"]
pub mod zip_directory;
/// Checked ZIP central and local fixed-field readers.
#[path = "check-package-artifacts/zip_fields.rs"]
pub mod zip_fields;
/// Checked little-endian ZIP field parsing.
#[path = "check-package-artifacts/zip_format.rs"]
pub mod zip_format;

use flate2 as _;

use reqwest as _;

use serde as _;

use serde_json as _;

use sha2 as _;

use core::ops::Range;

use std::{
    env::{self, ArgsOs},
    ffi::OsString,
    io::{Result as InputOutputResult, Write as _, stderr, stdout},
    path::PathBuf,
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::check_support::CheckResult;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&parse_request),
    size_of_val(&require_argument),
    size_of_val(&require_synchronized),
    size_of_val(&run),
    size_of_val(&write_success),
];

/// Paths and expected version supplied by the release workflow.
#[derive(Debug)]
struct ArtifactRequest {
    /// Cargo `.crate` archive path.
    cargo_archive: PathBuf,
    /// npm `.tgz` archive path.
    npm_archive: PathBuf,
    /// Python source-distribution archive path.
    python_sdist: PathBuf,
    /// Python wheel archive path.
    python_wheel: PathBuf,
    /// Canonical version required in every artifact.
    version: String,
}

/// Fixed central-directory values for one ZIP member.
#[derive(Debug)]
struct CentralFields {
    /// Member identity and file attributes.
    identity: CentralIdentity,
    /// Member offsets and lengths.
    layout: CentralLayout,
}

/// Identity and file attributes from one central header.
#[derive(Debug)]
struct CentralIdentity {
    /// Declared CRC-32.
    crc32: u32,
    /// External file attributes.
    external: u32,
    /// General-purpose flags.
    flags: u16,
    /// Creator platform and version.
    made_by: u16,
    /// Compression method.
    method: u16,
}

/// Offsets and lengths from one central header.
#[derive(Debug)]
struct CentralLayout {
    /// Central comment length.
    comment_length: usize,
    /// Compressed member size.
    compressed_size: usize,
    /// Central extra-data length.
    extra_length: usize,
    /// Local-header offset.
    local_offset: usize,
    /// Member-name length.
    name_length: usize,
    /// Unpacked member size.
    unpacked_size: u64,
}

/// Fixed end-of-central-directory fields.
#[derive(Debug)]
struct EndFields {
    /// Disk containing the central directory.
    central_disk: u16,
    /// Central-directory byte size.
    central_size: usize,
    /// Central-directory start offset.
    central_start: usize,
    /// End-record comment length.
    comment_length: u16,
    /// Current disk number.
    disk: u16,
    /// Entries on the current disk.
    disk_entries: u16,
    /// Total central-directory entries.
    total_entries: u16,
}

/// Validated license bytes from a package artifact.
#[derive(Debug)]
struct LicenseEvidence {
    /// Exact packaged license contents.
    license: Vec<u8>,
}

/// Fixed values read from one local ZIP header.
#[derive(Debug)]
struct LocalFields {
    /// Compressed file size.
    compressed_size: usize,
    /// Declared CRC-32.
    crc32: u32,
    /// Local extra-data length.
    extra_length: usize,
    /// General-purpose flags.
    flags: u16,
    /// End of the fixed local header.
    header_end: usize,
    /// Compression method.
    method: u16,
    /// Local member-name length.
    name_length: usize,
    /// Unpacked file size.
    unpacked_size: u64,
}

/// Normalized archive member kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MemberKind {
    /// Directory entry.
    Directory,
    /// Regular file entry.
    File,
}

/// Validated wrapper package files shared across npm and Python.
#[derive(Debug)]
struct WrapperEvidence {
    /// Exact packaged license contents.
    license: Vec<u8>,
    /// Exact generated native target manifest contents.
    native_targets: Vec<u8>,
}

/// One validated ZIP central-directory member.
#[derive(Debug)]
struct ZipMember {
    /// Complete local-header and file-data byte range.
    archive_range: Range<usize>,
    /// Compressed file-data byte range.
    compressed_range: Range<usize>,
    /// CRC-32 declared by the central directory.
    crc32: u32,
    /// Normalized member kind.
    kind: MemberKind,
    /// Compression method.
    method: u16,
    /// Canonical UTF-8 member path.
    path: String,
    /// Declared unpacked size.
    unpacked_size: u64,
}

/// Execute the package artifact check and report errors on standard error.
///
/// # Errors
///
/// Returns an error when a result cannot be written to its output stream.
fn main() -> InputOutputResult<ExitCode> {
    let result = parse_request().and_then(|request| return run(&request));
    return match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(error) => writeln!(stderr().lock(), "{error}").map(|()| return ExitCode::FAILURE),
    };
}

/// Parse the four artifact paths and canonical version.
///
/// # Errors
///
/// Returns an error when an argument is absent, the version is not UTF-8, or
/// an unexpected argument follows the version.
fn parse_request() -> CheckResult<ArtifactRequest> {
    let mut arguments = env::args_os();
    drop(arguments.next());
    let cargo_archive = PathBuf::from(check_try!(require_argument(&mut arguments, "crate")));
    let npm_archive = PathBuf::from(check_try!(require_argument(&mut arguments, "npm tgz")));
    let python_wheel = PathBuf::from(check_try!(require_argument(&mut arguments, "wheel")));
    let python_sdist = PathBuf::from(check_try!(require_argument(&mut arguments, "sdist")));
    let raw_version = check_try!(require_argument(&mut arguments, "version"));
    if arguments.next().is_some() {
        return Err(
            "usage: check-package-artifacts <crate> <npm.tgz> <wheel> <sdist> <version>".to_owned(),
        );
    }
    let version = check_try!(raw_version.into_string().map_err(|_invalid_version| {
        return "package artifact version must be UTF-8".to_owned();
    }));
    return Ok(ArtifactRequest {
        cargo_archive,
        npm_archive,
        python_sdist,
        python_wheel,
        version,
    });
}

/// Read one required command argument.
///
/// # Errors
///
/// Returns an error when the argument is absent.
fn require_argument(arguments: &mut ArgsOs, label: &str) -> CheckResult<OsString> {
    return arguments.next().ok_or_else(|| {
        return format!(
            "missing {label}; usage: check-package-artifacts <crate> <npm.tgz> <wheel> <sdist> <version>"
        );
    });
}

/// Require identical licenses and generated native targets across artifacts.
///
/// # Errors
///
/// Returns an error when duplicated release files differ.
fn require_synchronized(
    cargo: &LicenseEvidence,
    npm: &WrapperEvidence,
    sdist: &WrapperEvidence,
    wheel: &WrapperEvidence,
) -> CheckResult {
    check_try!(policy::require_equal_bytes(
        cargo.license.as_slice(),
        npm.license.as_slice(),
        "Cargo and npm LICENSE files",
    ));
    check_try!(policy::require_equal_bytes(
        npm.license.as_slice(),
        wheel.license.as_slice(),
        "npm and wheel LICENSE files",
    ));
    check_try!(policy::require_equal_bytes(
        wheel.license.as_slice(),
        sdist.license.as_slice(),
        "wheel and sdist LICENSE files",
    ));
    check_try!(policy::require_equal_bytes(
        npm.native_targets.as_slice(),
        wheel.native_targets.as_slice(),
        "npm and wheel native target manifests",
    ));
    return policy::require_equal_bytes(
        wheel.native_targets.as_slice(),
        sdist.native_targets.as_slice(),
        "wheel and sdist native target manifests",
    );
}

/// Validate every archive and cross-package synchronization invariant.
///
/// # Errors
///
/// Returns an error when an archive is unsafe, malformed, incomplete, leaky,
/// or inconsistent with the expected package version.
fn run(request: &ArtifactRequest) -> CheckResult {
    check_try!(policy::require_version(request.version.as_str()));
    let cargo = check_try!(cargo_package::validate(
        request.cargo_archive.as_path(),
        request.version.as_str(),
    ));
    let npm = check_try!(npm_package::validate(
        request.npm_archive.as_path(),
        request.version.as_str(),
    ));
    let wheel = check_try!(python_package::validate_wheel(
        request.python_wheel.as_path(),
        request.version.as_str(),
    ));
    let sdist = check_try!(python_package::validate_sdist(
        request.python_sdist.as_path(),
        request.version.as_str(),
    ));
    check_try!(require_synchronized(&cargo, &npm, &sdist, &wheel));
    return write_success(request.version.as_str());
}

/// Write the successful verification result.
///
/// # Errors
///
/// Returns an error when standard output cannot be written.
fn write_success(version: &str) -> CheckResult {
    return writeln!(
        stdout().lock(),
        "Verified Cargo, npm, wheel, and sdist artifacts for tovuk {version}."
    )
    .map_err(|error| return format!("write package artifact result: {error}"));
}
