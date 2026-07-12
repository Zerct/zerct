//! Build native release matrices and manage SHA-256 sidecar files.

/// Propagate a failed release operation without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

#[path = "native_release_tool/checksum.rs"]
pub mod checksum;
#[path = "native_release_tool/release_artifacts.rs"]
pub mod release_artifacts;

use core::str::from_utf8;
use flate2 as _;
use http as _;

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use tokio as _;

use serde::Deserialize;
use serde::Serialize;
use serde_json::{from_slice, to_string};
use sha2 as _;
use std::{
    env,
    fs::read as read_file,
    io::{Result as InputOutputResult, Write as _, stderr, stdout},
    path::Path,
    process::ExitCode,
};
use tar as _;
use tovuk_public_checks as _;
use url as _;

use checksum::{verify_sha256, write_sha256};
use release_artifacts::{ReleaseArtifactOperations as _, ReleaseArtifacts};

/// Native target whose build must use Zig for glibc compatibility.
const AARCH64_LINUX_GNU_TARGET: &str = "aarch64-unknown-linux-gnu";

/// Suffix selecting the oldest supported glibc ABI through `cargo-zigbuild`.
const AARCH64_LINUX_GNU_ZIG_SUFFIX: &str = ".2.17";

/// Compact command help printed for every invalid invocation.
const USAGE: &str = "usage:\n  native-release-tool asset-names <native-release-targets.json> <crate-Cargo.toml>\n  native-release-tool matrix <native-release-targets.json> <crate-Cargo.toml>\n  native-release-tool prepare-release <artifact-directory> <native-release-targets.json> <crate-Cargo.toml>\n  native-release-tool tag <crate-Cargo.toml>\n  native-release-tool verify-sha256 <file> <expected-hex>\n  native-release-tool write-sha256 <asset>";

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&is_aarch64_linux),
    size_of_val(&matrix_entry),
    size_of_val(&matrix_json),
    size_of_val(&package_version),
    size_of_val(&parse_version_value),
    size_of_val(&read_matrix),
    size_of_val(&read_package_version),
    size_of_val(&run),
    size_of_val(&run_arguments),
];

/// Serializable `GitHub` Actions matrix.
#[derive(Debug, Serialize)]
struct GitHubMatrix {
    /// Native builds included in the release job.
    include: Vec<MatrixEntry>,
}

/// One `GitHub` Actions native build.
#[derive(Debug, Serialize)]
struct MatrixEntry {
    /// Release asset suffix for the target platform.
    asset_ext: String,
    /// Exact release asset name for the crate version and target.
    asset_name: String,
    /// Binary name produced for the target platform.
    binary: String,
    /// Cargo frontend used to build the target.
    build_strategy: &'static str,
    /// Rust target passed to the selected Cargo frontend.
    build_target: String,
    /// Exact release tag for the crate version.
    release_tag: String,
    /// Exact runner selector tracked by the release manifest.
    runner: String,
    /// Canonical Rust target used to name release assets.
    target: String,
}

/// Root native release target manifest.
#[derive(Debug, Deserialize)]
struct NativeReleaseTargets {
    /// Tracked native release targets.
    targets: Vec<NativeTarget>,
}

/// Native target fields consumed by the release workflow.
#[derive(Debug, Deserialize)]
struct NativeTarget {
    /// Release asset suffix for the target platform.
    asset_ext: String,
    /// Binary name produced for the target platform.
    binary: String,
    /// Exact runner selector for this target.
    runner: String,
    /// Canonical Rust target triple.
    triple: String,
}

/// Return whether a target is the tracked GNU aarch64 Linux release target.
fn is_aarch64_linux(target: &str) -> bool {
    return target == AARCH64_LINUX_GNU_TARGET;
}

/// Execute the release utility and report command errors on standard error.
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

/// Convert one tracked target into its `GitHub` Actions matrix entry.
fn matrix_entry(target: NativeTarget, version: &str) -> MatrixEntry {
    let uses_zig = is_aarch64_linux(target.triple.as_str());
    let build_strategy = if uses_zig { "cargo-zigbuild" } else { "cargo" };
    let build_target = if uses_zig {
        format!("{}{AARCH64_LINUX_GNU_ZIG_SUFFIX}", target.triple)
    } else {
        target.triple.clone()
    };
    let asset_name = format!("tovuk-{version}-{}{}", target.triple, target.asset_ext);
    return MatrixEntry {
        asset_ext: target.asset_ext,
        asset_name,
        binary: target.binary,
        build_strategy,
        build_target,
        release_tag: format!("v{version}"),
        runner: target.runner,
        target: target.triple,
    };
}

/// Serialize a native release manifest as compact `GitHub` matrix JSON.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
fn matrix_json(manifest: NativeReleaseTargets, version: &str) -> Result<String, String> {
    let matrix = GitHubMatrix {
        include: manifest
            .targets
            .into_iter()
            .map(|target| return matrix_entry(target, version))
            .collect(),
    };
    return to_string(&matrix).map_err(|error| return format!("serialize matrix: {error}"));
}

/// Extract the unique nonempty version from a manifest's `[package]` table.
///
/// # Errors
///
/// Returns an error when the package version is missing, empty, duplicated, or
/// not represented as a canonical quoted string.
fn package_version(source: &str) -> Result<String, String> {
    let mut in_package = false;
    let mut version = None;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some((key, raw_value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "version" {
            continue;
        }
        if version.is_some() {
            return Err("[package] contains duplicate version entries".to_owned());
        }
        version = Some(check_try!(parse_version_value(raw_value)));
    }
    return version.ok_or_else(|| return "[package] version is missing".to_owned());
}

/// Parse the canonical quoted value of a package version assignment.
///
/// # Errors
///
/// Returns an error when the value is empty, escaped, unterminated, or followed
/// by content other than a TOML comment.
fn parse_version_value(raw_value: &str) -> Result<String, String> {
    let trimmed = raw_value.trim();
    let remainder = check_try!(
        trimmed
            .strip_prefix('"')
            .ok_or_else(|| return "[package] version must be double-quoted".to_owned())
    );
    let (version, suffix) = check_try!(
        remainder
            .split_once('"')
            .ok_or_else(|| return "[package] version string is unterminated".to_owned())
    );
    if version.trim().is_empty() {
        return Err("[package] version must not be empty".to_owned());
    }
    if version.contains('\\') {
        return Err("[package] version must not contain TOML escapes".to_owned());
    }
    let trailing = suffix.trim();
    if !trailing.is_empty() && !trailing.starts_with('#') {
        return Err("[package] version has unexpected trailing content".to_owned());
    }
    return Ok(version.to_owned());
}

/// Read and parse the tracked native release target manifest.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read or parsed.
fn read_matrix(path: &Path) -> Result<NativeReleaseTargets, String> {
    let source = check_try!(
        read_file(path).map_err(|error| return format!("read {}: {error}", path.display()))
    );
    return from_slice(source.as_slice())
        .map_err(|error| return format!("parse {}: {error}", path.display()));
}

/// Read a crate manifest and return its unique package version.
///
/// # Errors
///
/// Returns an error when the manifest cannot be read or its package version is
/// invalid.
fn read_package_version(path: &Path) -> Result<String, String> {
    let source = check_try!(
        read_file(path).map_err(|error| return format!("read {}: {error}", path.display()))
    );
    let text = check_try!(
        from_utf8(source.as_slice())
            .map_err(|error| return format!("parse {} as UTF-8: {error}", path.display()))
    );
    return package_version(text).map_err(|error| return format!("{}: {error}", path.display()));
}

/// Parse process arguments and execute the selected release operation.
///
/// # Errors
///
/// Returns an error when arguments or the selected operation are invalid.
fn run() -> Result<(), String> {
    let arguments = env::args().skip(0x1).collect::<Vec<_>>();
    return run_arguments(arguments.as_slice());
}

/// Execute a release operation from explicit command arguments.
///
/// # Errors
///
/// Returns an error when arguments or the selected operation are invalid.
fn run_arguments(arguments: &[String]) -> Result<(), String> {
    let argument_values = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    match *argument_values.as_slice() {
        ["asset-names", manifest, crate_manifest] => {
            return ReleaseArtifacts
                .write_asset_names(Path::new(manifest), Path::new(crate_manifest));
        }
        ["matrix", manifest, crate_manifest] => {
            let targets = check_try!(read_matrix(Path::new(manifest)));
            let version = check_try!(read_package_version(Path::new(crate_manifest)));
            let matrix = check_try!(matrix_json(targets, version.as_str()));
            return writeln!(stdout().lock(), "{matrix}")
                .map_err(|error| return format!("write matrix: {error}"));
        }
        ["prepare-release", directory, manifest, crate_manifest] => {
            return ReleaseArtifacts.prepare_release(
                Path::new(directory),
                Path::new(manifest),
                Path::new(crate_manifest),
            );
        }
        ["tag", crate_manifest] => {
            return ReleaseArtifacts.write_tag(Path::new(crate_manifest));
        }
        ["verify-sha256", file, expected] => {
            return verify_sha256(Path::new(file), expected);
        }
        ["write-sha256", asset] => {
            let written_path = check_try!(write_sha256(Path::new(asset)));
            return writeln!(stdout().lock(), "{}", written_path.display())
                .map_err(|error| return format!("write checksum path: {error}"));
        }
        _ => return Err(USAGE.to_owned()),
    }
}

#[cfg(test)]
#[path = "native_release_tool_tests/verification.rs"]
mod tests;
