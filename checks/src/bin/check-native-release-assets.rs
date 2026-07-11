//! Verify native release assets and checksums before wrapper publishes.

/// Propagate a failed release check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

extern crate alloc;

/// Bounded native-asset and checksum verification.
#[path = "check-native-release-assets/checksum.rs"]
pub mod checksum;
/// `GitHub` release polling, download, and cleanup operations.
#[path = "check-native-release-assets/release.rs"]
pub mod release;
/// Bounded native release asset verification tests.
#[cfg(test)]
#[path = "check_native_release_assets_tests/verification.rs"]
mod tests;

#[cfg(test)]
use checksum::{MAX_CHECKSUM_BYTES, MAX_NATIVE_ASSET_BYTES, read_limited_text, sha256_file};

use core::time::Duration;

use flate2 as _;

use reqwest as _;

use serde::Deserialize;

use serde_json::from_str;

use std::{
    env,
    fs::read_to_string,
    io::{Result as InputOutputResult, Write as _, stderr},
    path::{Path, PathBuf},
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::check_support::{CheckResult, repo_root};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&parse_request),
    size_of_val(&parse_wait_seconds),
    size_of_val(&read_crate_version),
    size_of_val(&required_assets),
    size_of_val(&run),
];

/// Visibility accepted while verifying one release asset set.
#[derive(Clone, Copy, Debug)]
enum DraftPolicy {
    /// Accept a draft during the atomic prepublication check.
    Allow,
    /// Require the final public release state.
    RequirePublished,
}

/// Root native release target manifest.
#[derive(Debug, Deserialize)]
struct NativeReleaseTargets {
    /// Tracked native release targets.
    targets: Vec<NativeTarget>,
}

/// Native target fields needed to derive release asset names.
#[derive(Debug, Deserialize)]
struct NativeTarget {
    /// Platform-specific asset suffix.
    asset_ext: String,
    /// Canonical Rust target triple.
    triple: String,
}

/// Parsed command request.
#[derive(Debug)]
struct ReleaseRequest {
    /// Visibility accepted by the remote release check.
    draft_policy: DraftPolicy,
    /// Crate version whose release assets must exist.
    version: String,
    /// Maximum polling duration.
    wait_seconds: u64,
}

/// Complete remote release verification input.
#[derive(Debug)]
struct ReleaseVerification {
    /// Visibility accepted by the remote release check.
    draft_policy: DraftPolicy,
    /// Public repository root.
    repo_root: PathBuf,
    /// Exact native assets without checksum suffixes.
    required_assets: Vec<String>,
    /// Release tag derived from the synchronized package version.
    tag: String,
    /// Maximum time spent waiting for uploaded assets.
    wait_duration: Duration,
}

/// Execute the release check and report command errors on standard error.
///
/// # Errors
///
/// Returns an error when a command failure cannot be written to standard error.
fn main() -> InputOutputResult<ExitCode> {
    match run() {
        Ok(()) => return Ok(ExitCode::SUCCESS),
        Err(error) => {
            return writeln!(stderr().lock(), "{error}").map(|()| return ExitCode::FAILURE);
        }
    }
}

/// Parse the optional version and wait duration command arguments.
///
/// # Errors
///
/// Returns an error when arguments are invalid or the default crate version
/// cannot be read.
fn parse_request(repository: &Path) -> CheckResult<ReleaseRequest> {
    let arguments = env::args().skip(0x1).collect::<Vec<_>>();
    if arguments.len() > 0x3 {
        return Err(
            "usage: check-native-release-assets [version] [wait_seconds] [--allow-draft]"
                .to_owned(),
        );
    }
    let version = match arguments.first().map(String::as_str) {
        None | Some("") => check_try!(read_crate_version(repository)),
        Some(value) => value.to_owned(),
    };
    let wait_seconds = check_try!(
        arguments
            .get(0x1)
            .map_or(Ok(u64::MIN), |value| return parse_wait_seconds(value))
    );
    let draft_policy = match arguments.get(0x2).map(String::as_str) {
        None => DraftPolicy::RequirePublished,
        Some("--allow-draft") => DraftPolicy::Allow,
        Some(_) => {
            return Err(
                "usage: check-native-release-assets [version] [wait_seconds] [--allow-draft]"
                    .to_owned(),
            );
        }
    };
    return Ok(ReleaseRequest {
        draft_policy,
        version,
        wait_seconds,
    });
}

/// Parse a wait duration in seconds.
///
/// # Errors
///
/// Returns an error when the value is not an unsigned integer.
fn parse_wait_seconds(value: &str) -> CheckResult<u64> {
    return value
        .parse::<u64>()
        .map_err(|error| return format!("wait_seconds must be an unsigned integer: {error}"));
}

/// Read the native crate version from its manifest.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read or has no explicit version.
fn read_crate_version(repository: &Path) -> CheckResult<String> {
    let manifest_path = repository.join("crates").join("tovuk").join("Cargo.toml");
    let source = check_try!(
        read_to_string(manifest_path.as_path())
            .map_err(|error| return format!("read {}: {error}", manifest_path.display()))
    );
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(raw_version) = trimmed.strip_prefix("version = ") else {
            continue;
        };
        let version = raw_version.trim().trim_matches('"');
        if !version.is_empty() {
            return Ok(version.to_owned());
        }
    }
    return Err(format!(
        "{} must contain a version",
        manifest_path.display()
    ));
}

/// Derive exact native release asset names from the tracked target manifest.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read or parsed.
fn required_assets(repository: &Path, version: &str) -> CheckResult<Vec<String>> {
    let manifest_path = repository.join("native-release-targets.json");
    let source = check_try!(
        read_to_string(manifest_path.as_path())
            .map_err(|error| return format!("read {}: {error}", manifest_path.display()))
    );
    let manifest = check_try!(
        from_str::<NativeReleaseTargets>(source.as_str())
            .map_err(|error| return format!("parse {}: {error}", manifest_path.display()))
    );
    let mut assets = manifest
        .targets
        .into_iter()
        .map(|target| return format!("tovuk-{version}-{}{}", target.triple, target.asset_ext))
        .collect::<Vec<_>>();
    assets.sort();
    return Ok(assets);
}

/// Build the release context and verify its assets.
///
/// # Errors
///
/// Returns an error when repository discovery, argument parsing, deadline
/// calculation, release polling, or asset verification fails.
fn run() -> CheckResult {
    let repository = check_try!(repo_root());
    let request = check_try!(parse_request(repository.as_path()));
    let required_assets = check_try!(required_assets(
        repository.as_path(),
        request.version.as_str(),
    ));
    return release::wait_for_release(ReleaseVerification {
        draft_policy: request.draft_policy,
        repo_root: repository,
        required_assets,
        tag: format!("v{}", request.version),
        wait_duration: Duration::from_secs(request.wait_seconds),
    });
}
