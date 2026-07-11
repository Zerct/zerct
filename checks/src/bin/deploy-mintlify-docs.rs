//! Verify Mintlify `GitHub` App docs synchronization.

/// Propagate a failed docs deployment check without the question-mark operator.
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

use core::time::Duration;

use std::{
    env,
    ffi::OsStr,
    io::{Write as _, stderr, stdout},
    path::Path,
    process::{Command, ExitCode},
    thread,
};

use tar as _;

use tovuk_public_checks::check_support::{
    CHECKS_MANIFEST, CheckResult, command, repo_root, tool_path,
};

/// Contract value named `DEFAULT_DOCS_CHECK_RETRIES`.
const DEFAULT_DOCS_CHECK_RETRIES: &str = "12";

/// Contract value named `DEFAULT_DOCS_CHECK_RETRY_DELAY_MS`.
const DEFAULT_DOCS_CHECK_RETRY_DELAY_MS: &str = "10000";

/// Contract value named `DEFAULT_DOCS_PUBLIC_URL`.
const DEFAULT_DOCS_PUBLIC_URL: &str = "https://docs.tovuk.com";

/// Contract value named `DEFAULT_SYNC_WAIT_SECONDS`.
const DEFAULT_SYNC_WAIT_SECONDS: &str = "30";

/// Largest accepted initial Mintlify propagation wait.
const MAX_SYNC_WAIT_SECONDS: u64 = 0x0078;

const _: [usize; 0x6] = [
    size_of_val(&configure_docs_cache_identity),
    size_of_val(&run),
    size_of_val(&run_check_bin),
    size_of_val(&run_readiness_check),
    size_of_val(&sync_wait_seconds),
    size_of_val(&write_initial_status),
];

/// Contract implementation for `check_command`.
fn check_command(repo_root: &Path, path: &OsStr, bin: &str) -> Command {
    let mut command = command(repo_root, path, "cargo");
    let _: &mut Command = command.args([
        "run",
        "--locked",
        "--quiet",
        "--manifest-path",
        CHECKS_MANIFEST,
        "--bin",
        bin,
        "--",
    ]);
    return command;
}

/// Forward the optional deployment and workflow-run identity to the readiness checker.
fn configure_docs_cache_identity(command: &mut Command) {
    for name in ["TOVUK_DOCS_CHECK_ID", "TOVUK_DOCS_REVISION"] {
        if let Some(value) = env::var_os(name) {
            let _: &mut Command = command.env(name, value);
        }
    }
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
    let target = env::var("TOVUK_DOCS_PUBLIC_URL")
        .unwrap_or_else(|_| return DEFAULT_DOCS_PUBLIC_URL.to_owned());
    let sync_wait_seconds = check_try!(sync_wait_seconds());

    check_try!(write_initial_status(target.as_str()));
    check_try!(run_check_bin(
        repo_root.as_path(),
        path.as_os_str(),
        "check-public-contracts",
        &["docs"],
    ));
    check_try!(run_check_bin(
        repo_root.as_path(),
        path.as_os_str(),
        "check-prose-style",
        &["--self-test"],
    ));
    check_try!(run_check_bin(
        repo_root.as_path(),
        path.as_os_str(),
        "check-prose-style",
        &[],
    ));

    if sync_wait_seconds > 0 {
        check_try!(writeln!(
            stdout().lock(),
            "Waiting {sync_wait_seconds}s for Mintlify GitHub App sync before public readiness check."
        )
        .map_err(|error| format!("write deployment status: {error}")));
        thread::sleep(Duration::from_secs(sync_wait_seconds));
    }

    return run_readiness_check(repo_root.as_path(), path.as_os_str(), target.as_str());
}

/// Contract implementation for `run_check_bin`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn run_check_bin(repo_root: &Path, path: &OsStr, bin: &str, args: &[&str]) -> CheckResult {
    let status = check_try!(
        check_command(repo_root, path, bin)
            .args(args)
            .status()
            .map_err(|error| format!("run {bin}: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{bin} failed with status {status}"));
}

/// Contract implementation for `run_readiness_check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn run_readiness_check(repo_root: &Path, path: &OsStr, target: &str) -> CheckResult {
    let retries = env::var("TOVUK_DOCS_CHECK_RETRIES")
        .unwrap_or_else(|_| return DEFAULT_DOCS_CHECK_RETRIES.to_owned());
    let retry_delay_ms = env::var("TOVUK_DOCS_CHECK_RETRY_DELAY_MS")
        .unwrap_or_else(|_| return DEFAULT_DOCS_CHECK_RETRY_DELAY_MS.to_owned());
    let mut readiness_command = check_command(repo_root, path, "check-public-contracts");
    let _: &mut Command = readiness_command
        .args(["mintlify-agent-readiness", target])
        .env("TOVUK_DOCS_CHECK_RETRIES", retries)
        .env("TOVUK_DOCS_CHECK_RETRY_DELAY_MS", retry_delay_ms);
    configure_docs_cache_identity(&mut readiness_command);
    let status = check_try!(
        readiness_command
            .status()
            .map_err(|error| format!("run mintlify-agent-readiness: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| format!("mintlify-agent-readiness failed with status {status}"));
}

/// Contract implementation for `sync_wait_seconds`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn sync_wait_seconds() -> CheckResult<u64> {
    let raw = env::var("TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS")
        .unwrap_or_else(|_| return DEFAULT_SYNC_WAIT_SECONDS.to_owned());
    if !raw
        .chars()
        .all(|character| return character.is_ascii_digit())
    {
        return Err(
            "TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS must be an unsigned integer.".to_owned(),
        );
    }
    let wait_seconds = check_try!(
        raw.parse::<u64>()
            .map_err(|error| format!("parse TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS: {error}"))
    );
    if wait_seconds > MAX_SYNC_WAIT_SECONDS {
        return Err(format!(
            "TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS must not exceed {MAX_SYNC_WAIT_SECONDS}."
        ));
    }
    return Ok(wait_seconds);
}

/// Write the initial Mintlify synchronization status.
///
/// # Errors
///
/// Returns an error when standard output cannot be written.
fn write_initial_status(target: &str) -> CheckResult {
    check_try!(
        writeln!(
            stdout().lock(),
            "Mintlify GitHub App owns production docs sync for this repository."
        )
        .map_err(|error| format!("write deployment status: {error}"))
    );
    return writeln!(
        stdout().lock(),
        "Checking local docs contracts before verifying public readiness at {target}."
    )
    .map_err(|error| format!("write deployment status: {error}"));
}
