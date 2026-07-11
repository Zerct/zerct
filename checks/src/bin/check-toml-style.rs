//! TOML style verification for the public repository.

/// Propagate a failed TOML check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

use flate2 as _;

use http as _;

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use tokio as _;

use url as _;

use serde as _;

use serde_json as _;

use sha2 as _;

use std::{
    ffi::OsStr,
    fs::{DirEntry, read_dir},
    io::{Write as _, stderr},
    path::{Path, PathBuf},
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::check_support::{CheckResult, command, repo_root, tool_path};

const _: [usize; 0x4] = [
    size_of_val(&collect_toml_files),
    size_of_val(&collect_toml_entry),
    size_of_val(&is_pruned_dir),
    size_of_val(&run),
];

/// Collect one directory entry when it is relevant to TOML style checks.
///
/// # Errors
///
/// Returns an error when file metadata cannot be inspected or a child directory cannot be read.
fn collect_toml_entry(
    repo_root: &Path,
    relative_dir: &Path,
    files: &mut Vec<PathBuf>,
    entry: &DirEntry,
) -> CheckResult {
    let relative_path = if relative_dir == Path::new(".") {
        PathBuf::from(entry.file_name())
    } else {
        relative_dir.join(entry.file_name())
    };
    let file_type = check_try!(
        entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", relative_path.display()))
    );
    if file_type.is_dir() {
        if is_pruned_dir(relative_path.as_path()) {
            return Ok(());
        }
        return collect_toml_files_in(repo_root, relative_path.as_path(), files);
    }
    if relative_path.extension() == Some(OsStr::new("toml")) {
        files.push(relative_path);
    }
    return Ok(());
}

/// Contract implementation for `collect_toml_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn collect_toml_files(repo_root: &Path) -> CheckResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    check_try!(collect_toml_files_in(repo_root, Path::new("."), &mut files));
    files.sort();
    files.dedup();
    return Ok(files);
}

/// Contract implementation for `collect_toml_files_in`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn collect_toml_files_in(
    repo_root: &Path,
    relative_dir: &Path,
    files: &mut Vec<PathBuf>,
) -> CheckResult {
    let absolute_dir = if relative_dir == Path::new(".") {
        repo_root.to_path_buf()
    } else {
        repo_root.join(relative_dir)
    };
    let mut entries = check_try!(
        check_try!(
            read_dir(absolute_dir.as_path())
                .map_err(|error| format!("read {}: {error}", relative_dir.display()))
        )
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read entry in {}: {error}", relative_dir.display()))
    );
    entries.sort_by_key(DirEntry::file_name);

    for entry in entries {
        check_try!(collect_toml_entry(repo_root, relative_dir, files, &entry));
    }
    return Ok(());
}

/// Contract implementation for `is_pruned_dir`.
fn is_pruned_dir(path: &Path) -> bool {
    return path
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| matches!(name, ".git" | "node_modules" | "target" | "vendor"));
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => return ExitCode::SUCCESS,
        Err(error) => {
            let _write_result = writeln!(stderr().lock(), "{error}");
            return ExitCode::FAILURE;
        }
    }
}

/// Contract implementation for `run`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn run() -> CheckResult {
    let repo_root = check_try!(repo_root());
    let path = tool_path();
    let toml_files = check_try!(collect_toml_files(repo_root.as_path()));
    if toml_files.is_empty() {
        return Ok(());
    }
    check_try!(run_taplo(
        repo_root.as_path(),
        path.as_os_str(),
        &["format", "--check"],
        toml_files.as_slice(),
    ));
    return run_taplo(
        repo_root.as_path(),
        path.as_os_str(),
        &["lint", "--no-schema"],
        toml_files.as_slice(),
    );
}

/// Contract implementation for `run_taplo`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn run_taplo(
    repo_root: &Path,
    path: &OsStr,
    fixed_args: &[&str],
    toml_files: &[PathBuf],
) -> CheckResult {
    let status = check_try!(
        command(repo_root, path, "taplo")
            .args(fixed_args)
            .args(toml_files)
            .status()
            .map_err(|error| format!("run taplo: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| format!("taplo failed with status {status}"));
}
