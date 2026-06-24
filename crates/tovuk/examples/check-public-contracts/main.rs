//! Public repository contract checks for Tovuk packages and docs.

mod cli_contract;
mod docs;
mod docs_sources;
mod helpers;
mod mintlify;
mod npm;
mod package_versions;
mod types;

use std::{env, process::ExitCode};

use helpers::{CheckResult, find_repo_root, read_package_json};

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
    let mut args = env::args().skip(1);
    let Some(check) = args.next() else {
        return Err("usage: scripts/check-public-contracts.sh <check>".to_owned());
    };

    let repo_root = find_repo_root();
    env::set_current_dir(repo_root.as_str()).map_err(|error| format!("cd {repo_root}: {error}"))?;

    match check.as_str() {
        "package-versions" => package_versions::check(),
        "cli-contract" => cli_contract::check(),
        "docs" => docs::check(),
        "openapi-path" => docs::print_openapi_path(),
        "npm-cli-package" => npm::check_cli_package(),
        "npm-native-runtime" => npm::check_native_runtime(),
        "mintlify-agent-readiness" => {
            let target = args
                .next()
                .unwrap_or_else(|| "https://docs.tovuk.com".to_owned());
            mintlify::check_agent_readiness(target.as_str())
        }
        "mintlify-score" => {
            let Some(path) = args.next() else {
                return Err("mintlify-score requires a score JSON path".to_owned());
            };
            mintlify::check_score(path.as_str())
        }
        "npm-version" => {
            let package = read_package_json("packages/tovuk/package.json")?;
            println!("{}", package.version);
            Ok(())
        }
        other => Err(format!("unknown check {other:?}")),
    }
}
