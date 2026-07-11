use alloc::collections::BTreeSet;

use crate::agent_guidance;

use crate::helpers::{CheckResult, OutputChannel, read_text, require_contains, write_line};

use crate::native_release_targets;

use crate::repo_hygiene_git::{existing_tracked_files, git_lines};

use crate::repo_hygiene_paths::{
    is_forbidden_tracked_path, is_guarded_source_path, is_public_repository_scan_path,
    is_public_text_scan_path, path_has_extension,
};

use crate::repo_hygiene_required::{
    require_ignored_paths, require_tracked_paths, require_visible_paths,
};

use crate::repo_hygiene_text::{
    line_contains_private_repository_marker, line_contains_retired_npm_runner_guidance,
};

use crate::script_contracts;

use std::path::Path;

/// Tracked launchers and hooks that intentionally retain executable mode.
const ALLOWED_EXECUTABLE_PATHS: &[&str] = &[
    ".githooks/pre-commit",
    ".githooks/pre-push",
    "packages/tovuk/bin/tovuk.mjs",
    "packages/tovuk/install.mjs",
];

/// Contract value named `MAX_SOURCE_FILE_LINES`.
const MAX_SOURCE_FILE_LINES: usize = 500;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000e] = [
    size_of_val(&check),
    size_of_val(&check_repository_contracts),
    size_of_val(&check_tracked_files),
    size_of_val(&reject_forbidden_line_matches),
    size_of_val(&reject_forbidden_tracked_files),
    size_of_val(&reject_oversized_source_files),
    size_of_val(&reject_private_repository_markers),
    size_of_val(&reject_retired_npx_guidance),
    size_of_val(&reject_tracked_go_files),
    size_of_val(&reject_unexpected_git_modes),
    size_of_val(&reject_untracked_files),
    size_of_val(&require_docs_deploy_observability_contract),
    size_of_val(&require_docs_deploy_observability_checker),
    size_of_val(&require_docs_deploy_observability_gated_api),
];

/// Predicate used by tracked-text hygiene scans.
type TextPredicate = fn(&str) -> bool;

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check() -> CheckResult {
    let tracked_files = check_try!(existing_tracked_files());
    let tracked_set = tracked_files.iter().cloned().collect::<BTreeSet<_>>();

    check_try!(require_tracked_paths(&tracked_set));
    check_try!(check_repository_contracts(&tracked_files));
    check_try!(check_tracked_files(&tracked_files));
    check_try!(reject_untracked_files());
    check_try!(require_ignored_paths());
    check_try!(require_visible_paths());

    check_try!(write_line(
        OutputChannel::Regular,
        "Checked public repository hygiene.",
    ));
    return Ok(());
}

/// Check durable repository instructions and generated public contracts.
///
/// # Errors
///
/// Returns an error when any repository-level contract differs.
fn check_repository_contracts(tracked_files: &[String]) -> CheckResult {
    check_try!(agent_guidance::check_policy(tracked_files));
    check_try!(native_release_targets::check());
    check_try!(script_contracts::check());
    return require_docs_deploy_observability_contract();
}

/// Check every tracked file and Git index mode against public policy.
///
/// # Errors
///
/// Returns an error when tracked content or index metadata violates policy.
fn check_tracked_files(tracked_files: &[String]) -> CheckResult {
    check_try!(reject_private_repository_markers(tracked_files));
    check_try!(reject_retired_npx_guidance(tracked_files));
    check_try!(reject_tracked_go_files(tracked_files));
    check_try!(reject_oversized_source_files(tracked_files));
    check_try!(reject_forbidden_tracked_files(tracked_files));
    return reject_unexpected_git_modes();
}

/// Contract implementation for `reject_forbidden_line_matches`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn reject_forbidden_line_matches(
    tracked_files: &[String],
    scan_path: TextPredicate,
    line_matches: TextPredicate,
    message: &str,
) -> CheckResult {
    let mut matches = Vec::new();
    for path in tracked_files {
        if !scan_path(path) || !Path::new(path).is_file() {
            continue;
        }
        let source = check_try!(read_text(path));
        matches.extend(
            source
                .lines()
                .enumerate()
                .filter(|indexed_line| return line_matches(indexed_line.1))
                .map(|(index, _line)| {
                    return format!("{path}:{}", index.saturating_add(0x0001));
                }),
        );
    }
    if matches.is_empty() {
        return Ok(());
    }
    return Err(format!("{message}\n{}", matches.join("\n")));
}

/// Contract implementation for `reject_forbidden_tracked_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_forbidden_tracked_files(tracked_files: &[String]) -> CheckResult {
    let forbidden = tracked_files
        .iter()
        .filter(|path| return is_forbidden_tracked_path(path))
        .cloned()
        .collect::<Vec<_>>();
    if forbidden.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "These secret/generated files are tracked and must be removed from git:\n{}",
        forbidden.join("\n")
    ));
}

/// Contract implementation for `reject_oversized_source_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_oversized_source_files(tracked_files: &[String]) -> CheckResult {
    let mut oversized = Vec::new();
    for path in tracked_files {
        if !Path::new(path).is_file() || !is_guarded_source_path(path) {
            continue;
        }
        let source = check_try!(read_text(path));
        let line_count = source.lines().count();
        if line_count > MAX_SOURCE_FILE_LINES {
            oversized.push(format!("{path}:{line_count}"));
        }
    }
    if oversized.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Tracked public source files must stay at or below {MAX_SOURCE_FILE_LINES} lines; split these files first:\n{}",
        oversized.join("\n")
    ));
}

/// Reject developer-local paths and private-engine locations everywhere.
///
/// # Errors
///
/// Returns an error when a tracked UTF-8 file exposes a private repository marker.
pub(super) fn reject_private_repository_markers(tracked_files: &[String]) -> CheckResult {
    return reject_forbidden_line_matches(
        tracked_files,
        is_public_repository_scan_path,
        line_contains_private_repository_marker,
        "Tracked public files must not expose developer-local or private-engine paths:",
    );
}

/// Contract implementation for `reject_retired_npx_guidance`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_retired_npx_guidance(tracked_files: &[String]) -> CheckResult {
    return reject_forbidden_line_matches(
        tracked_files,
        is_public_text_scan_path,
        line_contains_retired_npm_runner_guidance,
        "Use native `tovuk` guidance instead of retired npm-runner guidance:",
    );
}

/// Contract implementation for `reject_tracked_go_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_tracked_go_files(tracked_files: &[String]) -> CheckResult {
    let go_files = tracked_files
        .iter()
        .filter(|path| return path_has_extension(path, "go") && Path::new(path.as_str()).exists())
        .cloned()
        .collect::<Vec<_>>();
    if go_files.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Tracked Go source is not allowed in the public repo; use Rust-native checks:\n{}",
        go_files.join("\n")
    ));
}

/// Reject symlinks, submodules, unmerged entries, and unexpected executables.
///
/// # Errors
///
/// Returns an error when a tracked index entry has an unapproved Git mode.
pub(super) fn reject_unexpected_git_modes() -> CheckResult {
    let mut invalid = Vec::new();
    for entry in check_try!(git_lines(&["ls-files", "--stage"])) {
        let (metadata, path) = check_try!(
            entry
                .split_once('\t')
                .ok_or_else(|| return format!("malformed Git index entry: {entry}"))
        );
        let mut fields = metadata.split_whitespace();
        let mode = check_try!(
            fields
                .next()
                .ok_or_else(|| return format!("Git index entry lacks a mode: {entry}"))
        );
        let _object = check_try!(
            fields
                .next()
                .ok_or_else(|| return format!("Git index entry lacks an object: {entry}"))
        );
        let stage = check_try!(
            fields
                .next()
                .ok_or_else(|| return format!("Git index entry lacks a stage: {entry}"))
        );
        if fields.next().is_some() || stage != "0" {
            invalid.push(format!("{mode} stage {stage} {path}"));
            continue;
        }
        let approved =
            mode == "100644" || (mode == "100755" && ALLOWED_EXECUTABLE_PATHS.contains(&path));
        if !approved {
            invalid.push(format!("{mode} {path}"));
        }
    }
    if invalid.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Tracked public files contain symlinks, submodules, or unexpected executable modes:\n{}",
        invalid.join("\n")
    ));
}

/// Contract implementation for `reject_untracked_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_untracked_files() -> CheckResult {
    let untracked = check_try!(git_lines(&["ls-files", "--others", "--exclude-standard"]));
    if untracked.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "These files are not tracked and not ignored:\n{}\nCommit them if they are source, or add a precise .gitignore rule if generated/secret.",
        untracked.join("\n")
    ));
}

/// Require the Rust docs deploy checker to expose all readiness controls.
///
/// # Errors
///
/// Returns an error when a readiness control is missing.
fn require_docs_deploy_observability_checker(checker: &str) -> CheckResult {
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
    return Ok(());
}

/// Contract implementation for `require_docs_deploy_observability_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_docs_deploy_observability_contract() -> CheckResult {
    let workflow = check_try!(read_text(".github/workflows/docs-deploy.yml"));
    let checker = check_try!(read_text("checks/src/bin/deploy-mintlify-docs.rs"));
    check_try!(require_contains(
        workflow.as_str(),
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin deploy-mintlify-docs --",
        "Mintlify docs sync workflow must call the tracked Rust verification binary",
    ));
    check_try!(require_contains(
        workflow.as_str(),
        "Check Mintlify GitHub App sync",
        "Mintlify docs workflow must describe the GitHub App sync boundary",
    ));
    check_try!(require_docs_deploy_observability_checker(checker.as_str()));
    return require_docs_deploy_observability_gated_api(checker.as_str(), workflow.as_str());
}

/// Reject plan-gated Mintlify deployment API dependencies.
///
/// # Errors
///
/// Returns an error when a plan-gated deployment API is referenced.
fn require_docs_deploy_observability_gated_api(checker: &str, workflow: &str) -> CheckResult {
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
    return Ok(());
}
