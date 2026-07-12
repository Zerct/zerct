//! Public repository contract checks for Tovuk packages and docs.

/// Propagate an absent contract value without the question-mark operator.
macro_rules! check_some {
    ($option:expr) => {
        match $option {
            Some(value) => value,
            None => return None,
        }
    };
}

/// Propagate a failed public contract check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

/// Public contract checks for agent guidance.
pub mod agent_guidance;

extern crate alloc;

/// Public contract checks for cli contract.
#[path = "cli_contract_module.rs"]
pub mod cli_contract;

/// Public contract checks for docs.
pub mod docs;

/// Public contract checks for docs api contract.
#[path = "docs_api_contract_module.rs"]
pub mod docs_api_contract;

/// Public contract checks for docs navigation.
pub mod docs_navigation;

/// Public contract checks for docs sources.
pub mod docs_sources;

/// Public contract checks for helpers.
pub mod helpers;

/// Public contract checks for helpers io.
pub mod helpers_io;

/// Public contract checks for helpers public copy.
pub mod helpers_public_copy;

/// Public contract checks for html visible copy.
pub mod html_visible_copy;

/// Public contract checks for mintlify.
#[path = "mintlify_module.rs"]
pub mod mintlify;

/// Public contract checks for mintlify fetch.
pub mod mintlify_fetch;

/// Public contract checks for native release targets.
pub mod native_release_targets;

/// Public contract checks for npm.
pub mod npm;

/// Public contract checks for npm package.
pub mod npm_package;

/// Public contract checks for npm runtime.
pub mod npm_runtime;

/// Public contract checks for package versions.
pub mod package_versions;

/// Public contract checks for pushed Git objects.
#[path = "push_snapshot/module_root.rs"]
pub mod push_snapshot;

/// Public contract checks for repo hygiene.
pub mod repo_hygiene;

/// Public contract checks for repo hygiene git.
pub mod repo_hygiene_git;

/// Public contract checks for repo hygiene paths.
pub mod repo_hygiene_paths;

/// Public contract checks for repo hygiene required.
pub mod repo_hygiene_required;

/// Public contract checks for repo hygiene text.
pub mod repo_hygiene_text;

/// Public contract checks for reviewed tracked files.
pub mod repo_hygiene_tracked;

/// Public contract checks for retired contracts.
pub mod retired_contracts;

/// Public contract checks for runtime cli.
pub mod runtime_cli;

/// Public contract checks for script contracts.
pub mod script_contracts;

/// Public contract checks for support contract.
pub mod support_contract;

/// Public contract checks for types.
pub mod types;

use core::iter::Skip;

use flate2 as _;

use helpers::{CheckResult, OutputChannel, find_repo_root, write_line};

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use sha2 as _;

use std::{
    env::{self, Args},
    process::ExitCode,
};

use tar as _;

use tokio as _;

use tovuk_public_checks as _;

/// Usage displayed when no public contract group is selected.
const CONTRACT_CHECK_USAGE: &str =
    "usage: cargo run --manifest-path checks/Cargo.toml --bin check-public-contracts -- <check>";

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0007] = [
    size_of_val(&run),
    size_of_val(&run_ci_snapshot),
    size_of_val(&run_private_history),
    size_of_val(&run_push_snapshot),
    size_of_val(&run_repo_hygiene),
    size_of_val(&run_runtime_cli),
    size_of_val(&run_sync_public_tree_policy),
];

/// Command-line arguments after the checker binary name.
type ContractArguments = Skip<Args>;

fn main() -> ExitCode {
    match run() {
        Ok(()) => return ExitCode::SUCCESS,
        Err(error) => {
            drop(write_line(OutputChannel::Diagnostic, error.as_str()));
            return ExitCode::FAILURE;
        }
    }
}

/// Read one required checker argument.
///
/// # Errors
///
/// Returns an error when the argument is missing.
fn next_argument(args: &mut impl Iterator<Item = String>, label: &str) -> CheckResult<String> {
    return args.next().ok_or_else(|| format!("{label} is required"));
}

/// Contract implementation for `run`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
#[inline]
pub fn run() -> CheckResult {
    let mut args = env::args().skip(0x0001);
    let check = check_try!(
        args.next()
            .ok_or_else(|| return CONTRACT_CHECK_USAGE.to_owned())
    );

    let repo_root = check_try!(find_repo_root());
    check_try!(
        env::set_current_dir(repo_root.as_str())
            .map_err(|error| format!("cd {repo_root}: {error}"))
    );

    match check.as_str() {
        "ci-snapshot" => return run_ci_snapshot(&mut args),
        "repo-hygiene" => return run_repo_hygiene(&mut args),
        "sync-public-tree-policy" => return run_sync_public_tree_policy(&mut args),
        "native-release-targets" => return native_release_targets::check(),
        "package-versions" => return package_versions::check(),
        "private-history" => return run_private_history(&mut args),
        "public-version" => return package_versions::print_canonical_version(),
        "push-snapshot" => return run_push_snapshot(&mut args),
        "cli-contract" => return cli_contract::check(),
        "docs" => return docs::check(),
        "openapi-path" => return docs::print_openapi_path(),
        "npm-cli-package" => return npm::check_cli_package(),
        "npm-native-runtime" => return npm::check_native_runtime(),
        "runtime-cli" => return run_runtime_cli(&mut args),
        "mintlify-agent-readiness" => {
            let target = args
                .next()
                .unwrap_or_else(|| return "https://docs.tovuk.com".to_owned());
            return mintlify::check_agent_readiness(target.as_str());
        }
        "mintlify-score" => {
            let path = check_try!(next_argument(&mut args, "mintlify-score JSON path"));
            return mintlify::check_score(path.as_str());
        }
        other => return Err(format!("unknown check {other:?}")),
    }
}

/// Run trusted event history policy without positional parameters.
///
/// # Errors
///
/// Returns an error when an unexpected parameter or event object is invalid.
fn run_ci_snapshot(args: &mut ContractArguments) -> CheckResult {
    if args.next().is_some() {
        return Err("ci-snapshot accepts no arguments".to_owned());
    }
    return push_snapshot::check_ci();
}

/// Run reachable-history private-term policy without positional parameters.
///
/// # Errors
///
/// Returns an error when an unexpected parameter or reachable object is invalid.
fn run_private_history(args: &mut ContractArguments) -> CheckResult {
    if args.next().is_some() {
        return Err("private-history accepts no arguments".to_owned());
    }
    return push_snapshot::check_private_history();
}

/// Run pushed-object policy against the pre-push hook's standard input.
///
/// # Errors
///
/// Returns an error when the remote argument or pushed objects are invalid.
fn run_push_snapshot(args: &mut ContractArguments) -> CheckResult {
    let push_location = check_try!(next_argument(args, "push-snapshot push location"));
    if args.next().is_some() {
        return Err("push-snapshot accepts exactly one push location".to_owned());
    }
    return push_snapshot::check(push_location.as_str());
}

/// Run repository hygiene against the selected immutable Git snapshot.
///
/// # Errors
///
/// Returns an error when the snapshot argument or repository policy is invalid.
fn run_repo_hygiene(args: &mut ContractArguments) -> CheckResult {
    let snapshot = args.next().unwrap_or_else(|| return "index".to_owned());
    if args.next().is_some() {
        return Err("repo-hygiene accepts at most one snapshot argument".to_owned());
    }
    return repo_hygiene::check(snapshot.as_str());
}

/// Run public runtime checks against the selected native CLI and Python.
///
/// # Errors
///
/// Returns an error when either path is absent or the runtime contract fails.
fn run_runtime_cli(args: &mut ContractArguments) -> CheckResult {
    let native_cli = check_try!(next_argument(args, "runtime-cli native CLI path"));
    let python_bin = check_try!(next_argument(args, "runtime-cli Python interpreter path"));
    return runtime_cli::check(native_cli.as_str(), python_bin.as_str());
}

/// Check or regenerate the canonical data-only public-tree policy.
///
/// # Errors
///
/// Returns an error for unsupported arguments or policy drift.
fn run_sync_public_tree_policy(args: &mut ContractArguments) -> CheckResult {
    let mode = args.next().unwrap_or_else(|| return "--check".to_owned());
    if args.next().is_some() {
        return Err("sync-public-tree-policy accepts exactly one mode".to_owned());
    }
    return match mode.as_str() {
        "--check" => repo_hygiene_required::check_current_public_tree_policy(),
        "--write" => repo_hygiene_required::synchronize_public_tree_policy(),
        other => Err(format!(
            "sync-public-tree-policy mode must be --check or --write, not {other:?}"
        )),
    };
}
