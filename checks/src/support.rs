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
    SecretSignatureParts::new("-----BEGIN ENCRYPTED PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN OPENSSH PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN PGP PRIVATE ", "KEY BLOCK-----"),
    SecretSignatureParts::new("-----BEGIN PRIVATE ", "KEY-----"),
    SecretSignatureParts::new("-----BEGIN RSA PRIVATE ", "KEY-----"),
];

/// Provider-specific credential patterns reconstructed only while scanning.
const SECRET_TOKEN_PATTERNS: &[SecretTokenPattern] = &[
    SecretTokenPattern {
        body_length: 0x0024,
        extras: b"",
        name: "GitHub OAuth access token",
        prefix: SecretSignatureParts::new("gh", "o_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0024,
        extras: b"",
        name: "GitHub personal access token",
        prefix: SecretSignatureParts::new("gh", "p_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x004c,
        extras: b"",
        name: "GitHub refresh token",
        prefix: SecretSignatureParts::new("gh", "r_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0024,
        extras: b"_.-",
        name: "GitHub installation access token",
        prefix: SecretSignatureParts::new("gh", "s_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0024,
        extras: b"",
        name: "GitHub user access token",
        prefix: SecretSignatureParts::new("gh", "u_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0052,
        extras: b"",
        name: "GitHub fine-grained personal access token",
        prefix: SecretSignatureParts::new("github_", "pat_"),
        required_hyphens: 0x0000,
        separator: b"_",
        separator_offset: 0x0016,
    },
    SecretTokenPattern {
        body_length: 0x0024,
        extras: b"",
        name: "npm access token",
        prefix: SecretSignatureParts::new("npm", "_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0055,
        extras: b"-_",
        name: "PyPI API token",
        prefix: SecretSignatureParts::new("pypi", "-"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0018,
        extras: b"-",
        name: "Slack bot token",
        prefix: SecretSignatureParts::new("xo", "xb-"),
        required_hyphens: 0x0001,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0018,
        extras: b"-",
        name: "Slack user token",
        prefix: SecretSignatureParts::new("xo", "xp-"),
        required_hyphens: 0x0001,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0018,
        extras: b"",
        name: "Stripe live secret key",
        prefix: SecretSignatureParts::new("sk_", "live_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
    SecretTokenPattern {
        body_length: 0x0018,
        extras: b"",
        name: "Stripe live restricted key",
        prefix: SecretSignatureParts::new("rk_", "live_"),
        required_hyphens: 0x0000,
        separator: b"",
        separator_offset: 0x0000,
    },
];

/// Common result type for public repository checks.
pub type CheckResult<T = ()> = Result<T, String>;

/// Two source-safe fragments of one recognized credential marker.
#[derive(Clone, Copy, Debug)]
struct SecretSignatureParts {
    /// Leading marker fragment.
    prefix: &'static str,
    /// Trailing marker fragment.
    suffix: &'static str,
}

impl SecretSignatureParts {
    /// Return whether raw bytes contain this exact ASCII signature.
    fn is_match(self, contents: &[u8]) -> bool {
        let signature = self.signature();
        return contents
            .windows(signature.len())
            .any(|candidate| return candidate == signature.as_bytes());
    }

    /// Construct one split credential marker.
    const fn new(prefix: &'static str, suffix: &'static str) -> Self {
        return Self { prefix, suffix };
    }

    /// Reconstruct this signature only at scan time.
    fn signature(self) -> String {
        return format!("{}{}", self.prefix, self.suffix);
    }
}

/// One provider token's prefix, alphabet, length, and shape constraints.
#[derive(Clone, Copy, Debug)]
struct SecretTokenPattern {
    /// Number of bytes sufficient to identify the credential body.
    body_length: usize,
    /// Additional non-alphanumeric bytes accepted in the token body.
    extras: &'static [u8],
    /// Provider token name used in diagnostics.
    name: &'static str,
    /// Split prefix kept source-safe inside the scanner itself.
    prefix: SecretSignatureParts,
    /// Minimum internal hyphens required by the provider's shape.
    required_hyphens: usize,
    /// Fixed separator required between the body segments.
    separator: &'static [u8],
    /// Byte offset where the fixed separator begins.
    separator_offset: usize,
}

impl SecretTokenPattern {
    /// Return whether one byte belongs to this provider's token body.
    fn accepts(self, byte: u8) -> bool {
        return byte.is_ascii_alphanumeric() || self.extras.contains(&byte);
    }

    /// Return whether raw bytes contain this complete provider-token pattern.
    fn is_match(self, contents: &[u8]) -> bool {
        let signature = self.prefix.signature();
        return contents
            .windows(signature.len())
            .enumerate()
            .any(|(start, candidate)| {
                return candidate == signature.as_bytes()
                    && self.matches_at(contents, start, signature.len());
            });
    }

    /// Return whether the credential body is valid after one matching prefix.
    fn matches_at(self, contents: &[u8], start: usize, prefix_length: usize) -> bool {
        let body_start = start.saturating_add(prefix_length);
        return contents
            .get(body_start..)
            .is_some_and(|body| return self.matches_body(body));
    }

    /// Return whether bytes begin with this provider's identifying body shape.
    fn matches_body(self, contents: &[u8]) -> bool {
        let Some(body) = contents.get(..self.body_length) else {
            return false;
        };
        let Some((leading, separator_and_trailing)) = body.split_at_checked(self.separator_offset)
        else {
            return false;
        };
        let Some(trailing) = separator_and_trailing.strip_prefix(self.separator) else {
            return false;
        };
        let hyphens = body.iter().filter(|byte| return **byte == b'-').count();
        return leading
            .iter()
            .chain(trailing)
            .all(|byte| return self.accepts(*byte))
            && hyphens >= self.required_hyphens;
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
            .args(["ls-files", "--cached", "-z", "--full-name", "--"])
            .current_dir(repository)
            .output()
            .map_err(|error| return format!("run git ls-files: {error}"))
    );
    if !output.status.success() {
        return Err(format!("git ls-files failed with status {}", output.status));
    }
    if output.stdout.is_empty() {
        return Ok(Vec::new());
    }
    let records = check_try!(
        output
            .stdout
            .strip_suffix(b"\0")
            .ok_or_else(|| return "git ls-files returned unterminated output".to_owned())
    );
    return records
        .split(|byte| return *byte == 0x00)
        .map(|path| {
            return from_utf8(path)
                .map(str::to_owned)
                .map_err(|error| return format!("git ls-files returned non-UTF-8 path: {error}"));
        })
        .collect();
}

/// Reject recognized private-key and credential signatures in public bytes.
///
/// # Errors
///
/// Returns an error when input contains a known ASCII secret signature.
#[inline]
pub fn reject_secret_signatures(label: &str, contents: &[u8]) -> CheckResult {
    for parts in SECRET_SIGNATURE_PARTS {
        if parts.is_match(contents) {
            let signature = parts.signature();
            return Err(format!(
                "{label} contains forbidden secret signature {signature}"
            ));
        }
    }
    for pattern in SECRET_TOKEN_PATTERNS {
        if pattern.is_match(contents) {
            return Err(format!(
                "{label} contains a forbidden {} signature",
                pattern.name
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
