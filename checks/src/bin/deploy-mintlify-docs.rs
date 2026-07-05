//! Verify Mintlify GitHub App docs synchronization.

use std::{env, ffi::OsStr, path::Path, process::ExitCode, thread, time::Duration};

use tovuk_public_checks::check_support::{
    CHECKS_MANIFEST, CheckResult, command, repo_root, tool_path,
};

const DEFAULT_DOCS_PUBLIC_URL: &str = "https://docs.tovuk.com";
const DEFAULT_SYNC_WAIT_SECONDS: &str = "30";
const DEFAULT_DOCS_CHECK_RETRIES: &str = "12";
const DEFAULT_DOCS_CHECK_RETRY_DELAY_MS: &str = "10000";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> CheckResult {
    let repo_root = repo_root()?;
    let path = tool_path();
    let target =
        env::var("TOVUK_DOCS_PUBLIC_URL").unwrap_or_else(|_| DEFAULT_DOCS_PUBLIC_URL.to_owned());
    let sync_wait_seconds = sync_wait_seconds()?;

    println!("Mintlify GitHub App owns production docs sync for this repository.");
    println!("Checking local docs contracts before verifying public readiness at {target}.");
    run_check_bin(
        repo_root.as_path(),
        path.as_os_str(),
        "check-public-contracts",
        &["docs"],
    )?;
    run_check_bin(
        repo_root.as_path(),
        path.as_os_str(),
        "check-prose-style",
        &["--self-test"],
    )?;
    run_check_bin(
        repo_root.as_path(),
        path.as_os_str(),
        "check-prose-style",
        &[],
    )?;

    if sync_wait_seconds > 0 {
        println!(
            "Waiting {sync_wait_seconds}s for Mintlify GitHub App sync before public readiness check."
        );
        thread::sleep(Duration::from_secs(sync_wait_seconds));
    }

    run_readiness_check(repo_root.as_path(), path.as_os_str(), target.as_str())
}

fn sync_wait_seconds() -> CheckResult<u64> {
    let raw = env::var("TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS")
        .unwrap_or_else(|_| DEFAULT_SYNC_WAIT_SECONDS.to_owned());
    if !raw.chars().all(|character| character.is_ascii_digit()) {
        return Err(
            "TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS must be an unsigned integer.".to_owned(),
        );
    }
    raw.parse::<u64>()
        .map_err(|error| format!("parse TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS: {error}"))
}

fn run_readiness_check(repo_root: &Path, path: &OsStr, target: &str) -> CheckResult {
    let retries = env::var("TOVUK_DOCS_CHECK_RETRIES")
        .unwrap_or_else(|_| DEFAULT_DOCS_CHECK_RETRIES.to_owned());
    let retry_delay_ms = env::var("TOVUK_DOCS_CHECK_RETRY_DELAY_MS")
        .unwrap_or_else(|_| DEFAULT_DOCS_CHECK_RETRY_DELAY_MS.to_owned());
    let status = check_command(repo_root, path, "check-public-contracts")
        .args(["mintlify-agent-readiness", target])
        .env("TOVUK_DOCS_CHECK_RETRIES", retries)
        .env("TOVUK_DOCS_CHECK_RETRY_DELAY_MS", retry_delay_ms)
        .status()
        .map_err(|error| format!("run mintlify-agent-readiness: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("mintlify-agent-readiness failed with status {status}"))
}

fn run_check_bin(repo_root: &Path, path: &OsStr, bin: &str, args: &[&str]) -> CheckResult {
    let status = check_command(repo_root, path, bin)
        .args(args)
        .status()
        .map_err(|error| format!("run {bin}: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{bin} failed with status {status}"))
}

fn check_command(repo_root: &Path, path: &OsStr, bin: &str) -> std::process::Command {
    let mut command = command(repo_root, path, "cargo");
    command.args([
        "run",
        "--locked",
        "--quiet",
        "--manifest-path",
        CHECKS_MANIFEST,
        "--bin",
        bin,
        "--",
    ]);
    command
}
