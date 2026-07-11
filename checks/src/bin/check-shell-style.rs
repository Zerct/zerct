//! Shell style verification for the public repository.

/// Propagate a failed shell check without the question-mark operator.
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
    fs::read_dir,
    io::{Write as _, stderr},
    path::{Path, PathBuf},
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::check_support::{CheckResult, command, repo_root, tool_path};

/// Shell hooks which are executable repository entrypoints.
const SHELL_HOOKS: &[&str] = &[".githooks/pre-commit", ".githooks/pre-push"];

const _: [usize; 0x3] = [
    size_of_val(&collect_shell_entrypoints),
    size_of_val(&collect_shell_sources),
    size_of_val(&run),
];

/// One shell tool invocation.
#[derive(Clone, Copy, Debug)]
struct ShellTool {
    /// Arguments placed before repository paths.
    fixed_args: &'static [&'static str],
    /// Executable name.
    program: &'static str,
}

/// Contract implementation for `collect_shell_entrypoints`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn collect_shell_entrypoints(repo_root: &Path) -> CheckResult<Vec<PathBuf>> {
    let mut entrypoints = check_try!(collect_shell_files(repo_root, Path::new("scripts")));
    entrypoints.extend(
        SHELL_HOOKS
            .iter()
            .map(PathBuf::from)
            .filter(|hook| return repo_root.join(hook).is_file()),
    );
    entrypoints.sort();
    return Ok(entrypoints);
}

/// Contract implementation for `collect_shell_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn collect_shell_files(repo_root: &Path, relative_dir: &Path) -> CheckResult<Vec<PathBuf>> {
    let absolute_dir = repo_root.join(relative_dir);
    let mut files = Vec::new();
    if !absolute_dir.is_dir() {
        return Ok(files);
    }
    let entries = check_try!(
        read_dir(absolute_dir.as_path())
            .map_err(|error| format!("read {}: {error}", relative_dir.display()))
    );
    for entry_result in entries {
        let entry = check_try!(
            entry_result
                .map_err(|error| format!("read entry in {}: {error}", relative_dir.display()))
        );
        if !check_try!(
            entry
                .file_type()
                .map_err(|error| format!("read file type for {}: {error}", entry.path().display()))
        )
        .is_file()
        {
            continue;
        }
        if entry.path().extension() == Some(OsStr::new("sh")) {
            files.push(relative_dir.join(entry.file_name()));
        }
    }
    files.sort();
    return Ok(files);
}

/// Contract implementation for `collect_shell_sources`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn collect_shell_sources(repo_root: &Path) -> CheckResult<Vec<PathBuf>> {
    let mut sources = check_try!(collect_shell_files(repo_root, Path::new("scripts")));
    sources.extend(check_try!(collect_shell_files(
        repo_root,
        Path::new("scripts/lib")
    )));
    sources.extend(
        SHELL_HOOKS
            .iter()
            .map(PathBuf::from)
            .filter(|hook| return repo_root.join(hook).is_file()),
    );
    sources.sort();
    sources.dedup();
    return Ok(sources);
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
    let shell_sources = check_try!(collect_shell_sources(repo_root.as_path()));
    let shell_entrypoints = check_try!(collect_shell_entrypoints(repo_root.as_path()));
    if shell_sources.is_empty() {
        return Ok(());
    }

    check_try!(run_shell_tool(
        repo_root.as_path(),
        path.as_os_str(),
        ShellTool {
            fixed_args: &["-n"],
            program: "bash",
        },
        shell_sources.as_slice(),
    ));
    if !shell_entrypoints.is_empty() {
        check_try!(run_shell_tool(
            repo_root.as_path(),
            path.as_os_str(),
            ShellTool {
                fixed_args: &["-x"],
                program: "shellcheck",
            },
            shell_entrypoints.as_slice(),
        ));
    }
    return run_shell_tool(
        repo_root.as_path(),
        path.as_os_str(),
        ShellTool {
            fixed_args: &["-i", "2", "-ci", "-d"],
            program: "shfmt",
        },
        shell_sources.as_slice(),
    );
}

/// Contract implementation for `run_shell_tool`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn run_shell_tool(
    repo_root: &Path,
    path: &OsStr,
    tool: ShellTool,
    files: &[PathBuf],
) -> CheckResult {
    let status = check_try!(
        command(repo_root, path, tool.program)
            .args(tool.fixed_args)
            .args(files)
            .status()
            .map_err(|error| format!("run {}: {error}", tool.program))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} failed with status {status}", tool.program));
}
