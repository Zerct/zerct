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

use flate2 as _;

use helpers::{CheckResult, OutputChannel, find_repo_root, write_line};

use sha2 as _;

use std::{env, process::ExitCode};

use tar as _;

use tovuk_public_checks as _;

/// Usage displayed when no public contract group is selected.
const CONTRACT_CHECK_USAGE: &str =
    "usage: cargo run --manifest-path checks/Cargo.toml --bin check-public-contracts -- <check>";

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&run)];

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
        "repo-hygiene" => return repo_hygiene::check(),
        "native-release-targets" => return native_release_targets::check(),
        "package-versions" => return package_versions::check(),
        "public-version" => return package_versions::print_canonical_version(),
        "cli-contract" => return cli_contract::check(),
        "docs" => return docs::check(),
        "openapi-path" => return docs::print_openapi_path(),
        "npm-cli-package" => return npm::check_cli_package(),
        "npm-native-runtime" => return npm::check_native_runtime(),
        "runtime-cli" => {
            let native_cli = check_try!(next_argument(&mut args, "runtime-cli native CLI path"));
            let python_bin = check_try!(next_argument(
                &mut args,
                "runtime-cli Python interpreter path",
            ));
            return runtime_cli::check(native_cli.as_str(), python_bin.as_str());
        }
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
