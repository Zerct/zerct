//! Shared helpers for public repository check binaries.

use core::str::from_utf8;

use std::{
    env,
    ffi::{OsStr, OsString},
    fs::metadata as filesystem_metadata,
    path::{Path, PathBuf},
    process::Command,
};

/// Cargo manifest path for the local checks crate.
pub const CHECKS_MANIFEST: &str = "checks/Cargo.toml";

/// Split secret markers reconstructed only while scanning public bytes.
const SECRET_SIGNATURE_PARTS: &[SecretSignatureParts] = &[
    SecretSignatureParts::new("-----BEGIN DSA PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN EC PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN OPENSSH PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN RSA PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("gh", "o_"),
    SecretSignatureParts::new("gh", "p_"),
    SecretSignatureParts::new("gh", "r_"),
    SecretSignatureParts::new("gh", "s_"),
    SecretSignatureParts::new("gh", "u_"),
    SecretSignatureParts::new("github_", "pat_"),
    SecretSignatureParts::new("sk_", "live_"),
    SecretSignatureParts::new("xo", "xb-"),
];

/// Common result type for public repository checks.
pub type CheckResult<T = ()> = Result<T, String>;

/// Two source-safe fragments of one recognized credential marker.
struct SecretSignatureParts {
    /// Leading marker fragment.
    prefix: &'static str,
    /// Trailing marker fragment.
    suffix: &'static str,
}

impl SecretSignatureParts {
    /// Construct one split credential marker.
    const fn new(prefix: &'static str, suffix: &'static str) -> Self {
        return Self { prefix, suffix };
    }
}

/// Create a command rooted at `cwd` with the caller's executable search path.
#[inline]
#[must_use]
pub fn command(cwd: &Path, path: &OsStr, program: &str) -> Command {
    let mut prepared_command = Command::new(program);
    let _: &mut Command = prepared_command.current_dir(cwd).env("PATH", path);
    return prepared_command;
}

/// Render a path with slash separators for stable diagnostics.
#[inline]
#[must_use]
pub fn display_path(path: &Path) -> String {
    return path
        .components()
        .map(|component| return component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
}

/// Find the first executable candidate in a PATH value.
///
/// # Errors
///
/// Returns an error when none of the candidate executable names exist in the
/// supplied PATH.
#[inline]
pub fn find_command(path: &OsStr, candidates: &[&str]) -> CheckResult<PathBuf> {
    let executable = candidates
        .iter()
        .flat_map(|candidate| {
            return env::split_paths(path).map(move |directory| return directory.join(candidate));
        })
        .find(|candidate| {
            return filesystem_metadata(candidate).is_ok_and(|metadata| return metadata.is_file());
        });
    return executable.ok_or_else(|| {
        return format!("could not find any of {}", candidates.join(", "));
    });
}

/// Return Git-tracked files relative to the repository root.
///
/// # Errors
///
/// Returns an error when Git cannot list tracked files for the repository.
#[inline]
pub fn git_tracked_files(repository: &Path) -> CheckResult<Vec<String>> {
    let output = check_try!(
        Command::new("git")
            .args(["ls-files"])
            .current_dir(repository)
            .output()
            .map_err(|error| return format!("run git ls-files: {error}"))
    );
    if !output.status.success() {
        return Err(format!("git ls-files failed with status {}", output.status));
    }
    return Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ToOwned::to_owned)
        .collect());
}

/// Reject recognized private-key and credential signatures in public bytes.
///
/// # Errors
///
/// Returns an error when UTF-8 input contains a known secret signature.
#[inline]
pub fn reject_secret_signatures(label: &str, contents: &[u8]) -> CheckResult {
    let Ok(text) = from_utf8(contents) else {
        return Ok(());
    };
    for parts in SECRET_SIGNATURE_PARTS {
        let signature = format!("{}{}", parts.prefix, parts.suffix);
        if text.contains(signature.as_str()) {
            return Err(format!(
                "{label} contains forbidden secret signature {signature}"
            ));
        }
    }
    return Ok(());
}

/// Return the current Git repository root.
///
/// # Errors
///
/// Returns an error when Git is unavailable or the current directory is not
/// inside a Git worktree.
#[inline]
pub fn repo_root() -> CheckResult<PathBuf> {
    let output = check_try!(
        Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|error| return format!("run git rev-parse --show-toplevel: {error}"))
    );
    if !output.status.success() {
        return Err(format!(
            "git rev-parse --show-toplevel failed with status {}",
            output.status
        ));
    }
    return Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    ));
}

/// Run a command and require a successful exit status.
///
/// # Errors
///
/// Returns an error when the command cannot be started or exits unsuccessfully.
#[inline]
pub fn run_status(cwd: &Path, path: &OsStr, program: &str, args: &[&str]) -> CheckResult {
    let status = check_try!(
        Command::new(program)
            .args(args)
            .current_dir(cwd)
            .env("PATH", path)
            .status()
            .map_err(|error| return format!("run {program}: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("{program} failed with status {status}"));
}

/// Return the caller-provided tool `PATH` used by public repository checks.
#[inline]
#[must_use]
pub fn tool_path() -> OsString {
    return env::var_os("PATH").unwrap_or_default();
}
#[cfg(test)]
#[path = "support/verification.rs"]
mod tests;
