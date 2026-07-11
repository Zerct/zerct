use alloc::collections::BTreeSet;

use crate::helpers::{CheckResult, LabeledSnippet};

use std::{
    ffi::OsStr,
    fs::{metadata as file_metadata, read_to_string},
    path::{Path, PathBuf},
};

/// Contract value named `MAX_AGENTS_CHAIN_BYTES`.
const MAX_AGENTS_CHAIN_BYTES: u64 = 0x8000;

/// Contract value named `MAX_AGENT_FILE_BYTES`.
const MAX_AGENT_FILE_BYTES: u64 = 0x4000;

/// Contract value named `ROOT_AGENT_PATH`.
const ROOT_AGENT_PATH: &str = "AGENTS.md";

/// Contract value named `ROOT_REQUIRED_SNIPPETS`.
const ROOT_REQUIRED_SNIPPETS: &[LabeledSnippet] = &[
    (
        "Codex loads project instructions from the repo root down",
        "root AGENTS.md must describe Codex root-to-leaf instruction loading",
    ),
    (
        "AGENTS.override.md",
        "root AGENTS.md must document the override filename precedence",
    ),
    (
        "project_doc_max_bytes",
        "root AGENTS.md must mention the Codex project_doc_max_bytes cap",
    ),
    (
        "closer `AGENTS.md`",
        "root AGENTS.md must tell agents to place directory guidance close to code",
    ),
    (
        "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin check-all --",
        "root AGENTS.md must name the full local verification gate",
    ),
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 12] = [
    size_of_val(&agents_chain_for_directory),
    size_of_val(&check_chain_sizes),
    size_of_val(&check_policy),
    size_of_val(&directory_instruction_file),
    size_of_val(&directory_label),
    size_of_val(&existing_agent_files),
    size_of_val(&instruction_directories),
    size_of_val(&normalize_markdown_words),
    size_of_val(&require_agent_files_are_useful),
    size_of_val(&require_chain_size),
    size_of_val(&require_nested_agent_scope),
    size_of_val(&require_root_agent_guidance),
];

/// Contract implementation for `agents_chain_for_directory`.
pub(super) fn agents_chain_for_directory(
    directory: &Path,
    tracked_set: &BTreeSet<String>,
) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = PathBuf::new();
    push_directory_instruction(&mut chain, current.as_path(), tracked_set);
    for component in directory.components() {
        current.push(component.as_os_str());
        push_directory_instruction(&mut chain, current.as_path(), tracked_set);
    }
    return chain;
}

/// Contract implementation for `check_chain_sizes`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_chain_sizes(tracked_files: &[String]) -> CheckResult {
    let tracked_set = tracked_files.iter().cloned().collect::<BTreeSet<_>>();
    for directory in instruction_directories(tracked_files) {
        let chain = agents_chain_for_directory(directory.as_path(), &tracked_set);
        check_try!(require_chain_size(
            directory_label(directory.as_path()).as_str(),
            chain.as_slice(),
        ));
    }
    return Ok(());
}

/// Contract implementation for `check_policy`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_policy(tracked_files: &[String]) -> CheckResult {
    let agent_files = existing_agent_files(tracked_files);
    check_try!(require_agent_files_are_useful(agent_files.as_slice()));
    check_try!(require_root_agent_guidance());
    check_try!(require_nested_agent_scope(agent_files.as_slice()));
    return check_chain_sizes(tracked_files);
}

/// Contract implementation for `directory_instruction_file`.
pub(super) fn directory_instruction_file(
    directory: &Path,
    tracked_set: &BTreeSet<String>,
) -> Option<String> {
    for filename in ["AGENTS.override.md", "AGENTS.md"] {
        let path = directory.join(filename).to_string_lossy().into_owned();
        if tracked_set.contains(path.as_str()) && Path::new(path.as_str()).is_file() {
            return Some(path);
        }
    }
    return None;
}

/// Contract implementation for `directory_label`.
pub(super) fn directory_label(directory: &Path) -> String {
    if directory.as_os_str().is_empty() {
        return "root".to_owned();
    }
    return directory.display().to_string();
}

/// Contract implementation for `existing_agent_files`.
pub(super) fn existing_agent_files(tracked_files: &[String]) -> Vec<String> {
    return tracked_files
        .iter()
        .filter(|path| return is_agent_instruction_file(path))
        .filter(|path| return Path::new(path.as_str()).is_file())
        .cloned()
        .collect();
}

/// Contract implementation for `instruction_directories`.
pub(super) fn instruction_directories(tracked_files: &[String]) -> BTreeSet<PathBuf> {
    return tracked_files
        .iter()
        .filter(|path| return is_agent_instruction_file(path))
        .filter(|path| return Path::new(path.as_str()).is_file())
        .map(|path| {
            return Path::new(path.as_str())
                .parent()
                .map_or_else(PathBuf::new, Path::to_path_buf);
        })
        .collect();
}

/// Contract implementation for `is_agent_instruction_file`.
fn is_agent_instruction_file(path: &str) -> bool {
    return Path::new(path)
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|file_name| matches!(file_name, "AGENTS.md" | "AGENTS.override.md"));
}

/// Contract implementation for `normalize_markdown_words`.
pub(super) fn normalize_markdown_words(source: &str) -> String {
    return source.split_whitespace().collect::<Vec<_>>().join(" ");
}

/// Contract implementation for `push_directory_instruction`.
fn push_directory_instruction(
    chain: &mut Vec<String>,
    directory: &Path,
    tracked_set: &BTreeSet<String>,
) {
    if let Some(path) = directory_instruction_file(directory, tracked_set) {
        chain.push(path);
    }
}

/// Contract implementation for `require_agent_files_are_useful`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_agent_files_are_useful(agent_files: &[String]) -> CheckResult {
    for path in agent_files {
        let source =
            check_try!(read_to_string(path).map_err(|error| format!("read {path}: {error}")));
        if source.trim().is_empty() {
            return Err(format!("{path} must not be empty"));
        }

        let byte_len =
            check_try!(file_metadata(path).map_err(|error| format!("stat {path}: {error}"))).len();
        if byte_len > MAX_AGENT_FILE_BYTES {
            return Err(format!(
                "{path} is {byte_len} bytes, above the {MAX_AGENT_FILE_BYTES}-byte local AGENTS.md limit; split or delete stale guidance"
            ));
        }
    }
    return Ok(());
}

/// Contract implementation for `require_chain_size`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_chain_size(label: &str, paths: &[String]) -> CheckResult {
    let separator_count = check_try!(
        u64::try_from(paths.len().saturating_sub(0x0001))
            .map_err(|error| format!("AGENTS.md chain length does not fit in u64: {error}"))
    );
    let mut total_bytes = separator_count.saturating_mul(0x0002);
    for path in paths {
        let file_bytes =
            check_try!(file_metadata(path).map_err(|error| format!("stat {path}: {error}"))).len();
        total_bytes = total_bytes.saturating_add(file_bytes);
    }
    if total_bytes <= MAX_AGENTS_CHAIN_BYTES {
        return Ok(());
    }
    return Err(format!(
        "{label} AGENTS.md chain is {total_bytes} bytes, above Codex default project_doc_max_bytes {MAX_AGENTS_CHAIN_BYTES}"
    ));
}

/// Contract implementation for `require_nested_agent_scope`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_nested_agent_scope(agent_files: &[String]) -> CheckResult {
    let mut missing_scope = Vec::new();
    let mut repeated_codex_policy = Vec::new();
    for path in agent_files {
        if path == ROOT_AGENT_PATH {
            continue;
        }
        let source =
            check_try!(read_to_string(path).map_err(|error| format!("read {path}: {error}")));
        if !source
            .lines()
            .take(6)
            .any(|line| return line.contains("This file applies to"))
        {
            missing_scope.push(path.clone());
        }
        if source.contains("Codex loads project instructions")
            || source.contains("project_doc_max_bytes")
        {
            repeated_codex_policy.push(path.clone());
        }
    }

    if !missing_scope.is_empty() {
        return Err(format!(
            "Nested AGENTS.md files must start with a concrete scope line:\n{}",
            missing_scope.join("\n")
        ));
    }
    if !repeated_codex_policy.is_empty() {
        return Err(format!(
            "Nested AGENTS.md files must not repeat root Codex loading or project_doc_max_bytes policy:\n{}",
            repeated_codex_policy.join("\n")
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_root_agent_guidance`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_root_agent_guidance() -> CheckResult {
    let source = check_try!(
        read_to_string(ROOT_AGENT_PATH).map_err(|error| format!("read AGENTS.md: {error}"))
    );
    let searchable_source = normalize_markdown_words(source.as_str());
    for (snippet, label) in ROOT_REQUIRED_SNIPPETS.iter().copied() {
        if !searchable_source.contains(snippet) {
            return Err((*label).to_owned());
        }
    }
    return Ok(());
}
