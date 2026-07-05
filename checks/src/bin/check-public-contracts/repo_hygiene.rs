use std::{collections::BTreeSet, path::Path};

use crate::agent_guidance;
use crate::helpers::{CheckResult, read_text};
use crate::native_release_targets;
use crate::repo_hygiene_git::existing_tracked_files;
use crate::repo_hygiene_paths::{
    is_forbidden_tracked_path, is_go_toolchain_scan_path, is_guarded_source_path,
    is_public_text_scan_path, path_has_extension,
};
use crate::repo_hygiene_required::{require_ignored_paths, require_tracked_paths};
use crate::repo_hygiene_text::{
    line_contains_forbidden_go_toolchain, line_contains_retired_npm_runner_guidance,
};
use crate::script_contracts;

const MAX_SOURCE_FILE_LINES: usize = 500;

pub(crate) fn check() -> CheckResult {
    let tracked_files = existing_tracked_files()?;
    let tracked_set = tracked_files.iter().cloned().collect::<BTreeSet<_>>();

    require_tracked_paths(&tracked_set)?;
    agent_guidance::check_chain_sizes(&tracked_files)?;
    native_release_targets::check()?;
    script_contracts::check()?;
    require_docs_deploy_observability_contract()?;
    reject_retired_npx_guidance(&tracked_files)?;
    reject_tracked_go_files(&tracked_files)?;
    reject_go_toolchain_bootstrap(&tracked_files)?;
    reject_oversized_source_files(&tracked_files)?;
    reject_forbidden_tracked_files(&tracked_files)?;
    reject_untracked_files()?;
    require_ignored_paths()?;

    println!("Checked public repository hygiene.");
    Ok(())
}

fn require_docs_deploy_observability_contract() -> CheckResult {
    let workflow = read_text(".github/workflows/docs-deploy.yml")?;
    let checker = read_text("checks/src/bin/deploy-mintlify-docs.rs")?;
    if !workflow.contains(
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin deploy-mintlify-docs --",
    ) {
        return Err(
            "Mintlify docs sync workflow must call the tracked Rust verification binary"
                .to_owned(),
        );
    }
    if !workflow.contains("Check Mintlify GitHub App sync") {
        return Err("Mintlify docs workflow must describe the GitHub App sync boundary".to_owned());
    }
    for (snippet, label) in [
        (
            "Mintlify GitHub App owns production docs sync",
            "Mintlify docs script must document that production sync is owned by the GitHub App",
        ),
        (
            "\"check-public-contracts\",\n        &[\"docs\"]",
            "Mintlify docs script must validate local docs contracts before public readiness",
        ),
        (
            "\"check-prose-style\",\n        &[\"--self-test\"]",
            "Mintlify docs script must run prose style self-test before public readiness",
        ),
        (
            "TOVUK_DOCS_GITHUB_APP_SYNC_WAIT_SECONDS",
            "Mintlify docs script must expose a bounded GitHub App sync wait",
        ),
        (
            "const DEFAULT_DOCS_PUBLIC_URL: &str = \"https://docs.tovuk.com\";",
            "Mintlify docs script must default readiness checks to the public docs domain",
        ),
        (
            "mintlify-agent-readiness",
            "Mintlify docs script must run the tracked public readiness checker",
        ),
    ] {
        if !checker.contains(snippet) {
            return Err(label.to_owned());
        }
    }
    for forbidden in [
        "api.mintlify.com/v1/project/update",
        "MINTLIFY_ADMIN_API_KEY",
        "MINTLIFY_PROJECT_ID",
    ] {
        if checker.contains(forbidden) || workflow.contains(forbidden) {
            return Err(
                "Mintlify docs workflow must not depend on the plan-gated deployment API"
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn reject_retired_npx_guidance(tracked_files: &[String]) -> CheckResult {
    reject_forbidden_line_matches(
        tracked_files,
        is_public_text_scan_path,
        line_contains_retired_npm_runner_guidance,
        "Use native `tovuk` guidance instead of retired npm-runner guidance:",
    )
}

fn reject_tracked_go_files(tracked_files: &[String]) -> CheckResult {
    let go_files = tracked_files
        .iter()
        .filter(|path| path_has_extension(path, "go") && Path::new(path.as_str()).exists())
        .cloned()
        .collect::<Vec<_>>();
    if go_files.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Tracked Go source is not allowed in the public repo; use Rust-native checks:\n{}",
            go_files.join("\n")
        ))
    }
}

fn reject_go_toolchain_bootstrap(tracked_files: &[String]) -> CheckResult {
    reject_forbidden_line_matches(
        tracked_files,
        is_go_toolchain_scan_path,
        line_contains_forbidden_go_toolchain,
        "Public repo tooling must not bootstrap Go toolchains; use Rust-native or prebuilt native release tools:",
    )
}

fn reject_forbidden_line_matches(
    tracked_files: &[String],
    scan_path: fn(&str) -> bool,
    line_matches: fn(&str) -> bool,
    message: &str,
) -> CheckResult {
    let mut matches = Vec::new();
    for path in tracked_files {
        if !scan_path(path) || !Path::new(path).is_file() {
            continue;
        }
        let source = read_text(path)?;
        for (index, line) in source.lines().enumerate() {
            if line_matches(line) {
                matches.push(format!("{}:{}", path, index + 1));
            }
        }
    }
    if matches.is_empty() {
        Ok(())
    } else {
        Err(format!("{message}\n{}", matches.join("\n")))
    }
}

fn reject_oversized_source_files(tracked_files: &[String]) -> CheckResult {
    let mut oversized = Vec::new();
    for path in tracked_files {
        if !Path::new(path).is_file() || !is_guarded_source_path(path) {
            continue;
        }
        let source = read_text(path)?;
        let line_count = source.lines().count();
        if line_count > MAX_SOURCE_FILE_LINES {
            oversized.push(format!("{path}:{line_count}"));
        }
    }
    if oversized.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Tracked public source files must stay at or below {MAX_SOURCE_FILE_LINES} lines; split these files first:\n{}",
            oversized.join("\n")
        ))
    }
}

fn reject_forbidden_tracked_files(tracked_files: &[String]) -> CheckResult {
    let forbidden = tracked_files
        .iter()
        .filter(|path| is_forbidden_tracked_path(path))
        .cloned()
        .collect::<Vec<_>>();
    if forbidden.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "These secret/generated files are tracked and must be removed from git:\n{}",
            forbidden.join("\n")
        ))
    }
}

fn reject_untracked_files() -> CheckResult {
    let untracked =
        crate::repo_hygiene_git::git_lines(&["ls-files", "--others", "--exclude-standard"])?;
    if untracked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "These files are not tracked and not ignored:\n{}\nCommit them if they are source, or add a precise .gitignore rule if generated/secret.",
            untracked.join("\n")
        ))
    }
}
