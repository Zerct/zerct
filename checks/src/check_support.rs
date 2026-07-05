//! Shared helpers for public repository check binaries.

use std::{
    env,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

/// Common result type for public repository checks.
pub type CheckResult<T = ()> = Result<T, String>;

/// Cargo manifest path for the local checks crate.
pub const CHECKS_MANIFEST: &str = "checks/Cargo.toml";

const TOOL_PATH_PREFIX: &str = "/opt/tovuk/native-tools/bin:/opt/tovuk/cargo-tools/bin:/opt/tovuk/cargo/bin:/opt/tovuk/rust/stable/bin:/opt/tovuk/node/bin:/usr/local/bin:/usr/bin:/bin";

/// Return the current Git repository root.
///
/// # Errors
///
/// Returns an error when Git is unavailable or the current directory is not
/// inside a Git worktree.
pub fn repo_root() -> CheckResult<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| format!("run git rev-parse --show-toplevel: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse --show-toplevel failed with status {}",
            output.status
        ));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    ))
}

/// Return the trusted tool PATH used by public repository checks.
#[must_use]
pub fn tool_path() -> OsString {
    let existing = env::var_os("PATH").unwrap_or_default();
    let mut path = OsString::from(TOOL_PATH_PREFIX);
    path.push(":");
    path.push(existing);
    path
}

/// Create a command rooted at `cwd` with the trusted check PATH.
#[must_use]
pub fn command(cwd: &Path, path: &OsStr, program: &str) -> Command {
    let mut command = Command::new(program);
    command.current_dir(cwd).env("PATH", path);
    command
}

/// Run a command and require a successful exit status.
///
/// # Errors
///
/// Returns an error when the command cannot be started or exits unsuccessfully.
pub fn run_status(cwd: &Path, path: &OsStr, program: &str, args: &[&str]) -> CheckResult {
    let status = command(cwd, path, program)
        .args(args)
        .status()
        .map_err(|error| format!("run {program}: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{program} failed with status {status}"))
}

/// Find the first executable candidate in a PATH value.
///
/// # Errors
///
/// Returns an error when none of the candidate executable names exist in the
/// supplied PATH.
pub fn find_command(path: &OsStr, candidates: &[&str]) -> CheckResult<PathBuf> {
    for directory in env::split_paths(path) {
        for candidate in candidates {
            let executable = directory.join(candidate);
            if executable.is_file() {
                return Ok(executable);
            }
        }
    }
    Err(format!("could not find any of {}", candidates.join(", ")))
}

/// Return Git-tracked files relative to the repository root.
///
/// # Errors
///
/// Returns an error when Git cannot list tracked files for the repository.
pub fn git_tracked_files(repo_root: &Path) -> CheckResult<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(repo_root)
        .output()
        .map_err(|error| format!("run git ls-files: {error}"))?;
    if !output.status.success() {
        return Err(format!("git ls-files failed with status {}", output.status));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ToOwned::to_owned)
        .collect())
}

/// Render a path with slash separators for stable diagnostics.
#[must_use]
pub fn display_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
