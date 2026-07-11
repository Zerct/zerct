use crate::{helpers::CheckResult, types::PackageJson};

use serde::de::DeserializeOwned;

use serde_json::from_str;

use std::{
    env::var,
    fs::{read_dir, read_to_string},
    io::{Write as StandardWrite, stderr, stdout},
    path::{Path, PathBuf},
    process::Command,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&find_repo_root)];

/// Destination for a public checker status line.
#[derive(Clone, Copy, Debug)]
pub(super) enum OutputChannel {
    /// Diagnostic output written to standard error.
    Diagnostic,
    /// Regular status output written to standard output.
    Regular,
}

/// Contract implementation for `collect_paths_with_suffix`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn collect_paths_with_suffix(root: &Path, suffix: &str, paths: &mut Vec<PathBuf>) -> CheckResult {
    let entries = check_try!(
        read_dir(root).map_err(|error| format!("read directory {}: {error}", root.display()))
    );
    for entry_result in entries {
        let entry = check_try!(
            entry_result.map_err(|error| format!("read entry under {}: {error}", root.display()))
        );
        let path = entry.path();
        let file_type = check_try!(
            entry
                .file_type()
                .map_err(|error| format!("read file type for {}: {error}", path.display()))
        );
        if file_type.is_dir() {
            check_try!(collect_paths_with_suffix(path.as_path(), suffix, paths));
            continue;
        }
        if entry.file_name().to_string_lossy().ends_with(suffix) {
            paths.push(path);
        }
    }
    return Ok(());
}

/// Contract implementation for `env_int`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn env_int(name: &str, default_value: i64) -> CheckResult<i64> {
    let raw = var(name).unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(default_value);
    }
    return trimmed
        .parse::<i64>()
        .map_err(|error| format!("{name} must be an integer: {error}"));
}

/// Contract implementation for `file_exists`.
pub(super) fn file_exists(path: impl AsRef<Path>) -> bool {
    return path.as_ref().exists();
}

/// Contract implementation for `find_repo_root`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn find_repo_root() -> CheckResult<String> {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) if output.status.success() => {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
        }
        Ok(output) => {
            return Err(format!(
                "git rev-parse --show-toplevel failed with status {}",
                output.status
            ));
        }
        Err(error) => return Err(format!("run git rev-parse --show-toplevel: {error}")),
    }
}

/// Contract implementation for `must_abs`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn must_abs(path: impl AsRef<Path>) -> CheckResult<String> {
    let requested_path = path.as_ref();
    return requested_path
        .canonicalize()
        .map(|absolute| return absolute.display().to_string())
        .map_err(|error| format!("resolve {}: {error}", requested_path.display()));
}

/// Contract implementation for `read_json`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn read_json<T>(path: impl AsRef<Path>) -> CheckResult<T>
where
    T: DeserializeOwned,
{
    let requested_path = path.as_ref();
    let source = check_try!(read_text(requested_path));
    return from_str(source.as_str())
        .map_err(|error| format!("parse {}: {error}", requested_path.display()));
}

/// Contract implementation for `read_package_json`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn read_package_json(path: impl AsRef<Path>) -> CheckResult<PackageJson> {
    return read_json(path);
}

/// Contract implementation for `read_sorted_texts_recursive`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn read_sorted_texts_recursive(
    directory: &str,
    suffix: &str,
) -> CheckResult<Vec<String>> {
    let mut paths = Vec::new();
    check_try!(collect_paths_with_suffix(
        Path::new(directory),
        suffix,
        &mut paths
    ));
    paths.sort();

    return paths
        .iter()
        .map(|path| return read_text(path.as_path()))
        .collect::<CheckResult<Vec<_>>>();
}

/// Contract implementation for `read_text`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn read_text(path: impl AsRef<Path>) -> CheckResult<String> {
    let requested_path = path.as_ref();
    return read_to_string(requested_path)
        .map_err(|error| format!("read {}: {error}", requested_path.display()));
}

/// Read ordered text files and join them into one scan corpus.
///
/// # Errors
///
/// Returns an error when any requested file cannot be read.
pub(super) fn read_text_corpus(paths: &[&str]) -> CheckResult<String> {
    return paths
        .iter()
        .map(|path| return read_text(path))
        .collect::<CheckResult<Vec<_>>>()
        .map(|sources| return sources.join("\n"));
}

/// Write one complete line to a standard output channel.
///
/// # Errors
///
/// Returns an error when the selected stream cannot be written.
pub(super) fn write_line(channel: OutputChannel, message: &str) -> CheckResult {
    return match channel {
        OutputChannel::Diagnostic => {
            StandardWrite::write_fmt(&mut stderr().lock(), format_args!("{message}\n"))
                .map_err(|error| format!("write standard error: {error}"))
        }
        OutputChannel::Regular => {
            StandardWrite::write_fmt(&mut stdout().lock(), format_args!("{message}\n"))
                .map_err(|error| format!("write standard output: {error}"))
        }
    };
}
