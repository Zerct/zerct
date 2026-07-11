//! Shared helpers for public repository check binaries.

use std::{
    env,
    ffi::{OsStr, OsString},
    fs::metadata as filesystem_metadata,
    path::{Path, PathBuf},
    process::Command,
};

/// Cargo manifest path for the local checks crate.
pub const CHECKS_MANIFEST: &str = "checks/Cargo.toml";

/// Common result type for public repository checks.
pub type CheckResult<T = ()> = Result<T, String>;

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
    let executable = env::split_paths(path)
        .flat_map(|directory| {
            return candidates
                .iter()
                .map(move |candidate| return directory.join(candidate));
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
