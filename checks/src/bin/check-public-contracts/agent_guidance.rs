use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::helpers::CheckResult;

const MAX_AGENTS_CHAIN_BYTES: u64 = 32_768;
const MAX_AGENT_FILE_BYTES: u64 = 16_384;
const ROOT_AGENT_PATH: &str = "AGENTS.md";
const ROOT_REQUIRED_SNIPPETS: &[(&str, &str)] = &[
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

pub(crate) fn check_policy(tracked_files: &[String]) -> CheckResult {
    let agent_files = existing_agent_files(tracked_files);
    require_agent_files_are_useful(agent_files.as_slice())?;
    require_root_agent_guidance()?;
    require_nested_agent_scope(agent_files.as_slice())?;
    check_chain_sizes(tracked_files)
}

fn existing_agent_files(tracked_files: &[String]) -> Vec<String> {
    tracked_files
        .iter()
        .filter(|path| is_agent_instruction_file(path))
        .filter(|path| Path::new(path.as_str()).is_file())
        .cloned()
        .collect()
}

fn require_agent_files_are_useful(agent_files: &[String]) -> CheckResult {
    for path in agent_files {
        let source = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
        if source.trim().is_empty() {
            return Err(format!("{path} must not be empty"));
        }

        let byte_len = fs::metadata(path)
            .map_err(|error| format!("stat {path}: {error}"))?
            .len();
        if byte_len > MAX_AGENT_FILE_BYTES {
            return Err(format!(
                "{path} is {byte_len} bytes, above the {MAX_AGENT_FILE_BYTES}-byte local AGENTS.md limit; split or delete stale guidance"
            ));
        }
    }
    Ok(())
}

fn require_root_agent_guidance() -> CheckResult {
    let source =
        fs::read_to_string(ROOT_AGENT_PATH).map_err(|error| format!("read AGENTS.md: {error}"))?;
    let searchable_source = normalize_markdown_words(source.as_str());
    for (snippet, label) in ROOT_REQUIRED_SNIPPETS {
        if !searchable_source.contains(snippet) {
            return Err((*label).to_owned());
        }
    }
    Ok(())
}

fn normalize_markdown_words(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn require_nested_agent_scope(agent_files: &[String]) -> CheckResult {
    let mut missing_scope = Vec::new();
    let mut repeated_codex_policy = Vec::new();
    for path in agent_files {
        if path == ROOT_AGENT_PATH {
            continue;
        }
        let source = fs::read_to_string(path).map_err(|error| format!("read {path}: {error}"))?;
        if !source
            .lines()
            .take(6)
            .any(|line| line.contains("This file applies to"))
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
    Ok(())
}

fn check_chain_sizes(tracked_files: &[String]) -> CheckResult {
    let tracked_set = tracked_files.iter().cloned().collect::<BTreeSet<_>>();
    for directory in instruction_directories(tracked_files) {
        let chain = agents_chain_for_directory(directory.as_path(), &tracked_set);
        require_chain_size(
            directory_label(directory.as_path()).as_str(),
            chain.as_slice(),
        )?;
    }
    Ok(())
}

fn instruction_directories(tracked_files: &[String]) -> BTreeSet<PathBuf> {
    tracked_files
        .iter()
        .filter(|path| is_agent_instruction_file(path))
        .filter(|path| Path::new(path.as_str()).is_file())
        .map(|path| {
            Path::new(path.as_str())
                .parent()
                .map_or_else(PathBuf::new, Path::to_path_buf)
        })
        .collect()
}

fn agents_chain_for_directory(directory: &Path, tracked_set: &BTreeSet<String>) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = PathBuf::new();
    push_directory_instruction(&mut chain, current.as_path(), tracked_set);
    for component in directory.components() {
        current.push(component.as_os_str());
        push_directory_instruction(&mut chain, current.as_path(), tracked_set);
    }
    chain
}

fn push_directory_instruction(
    chain: &mut Vec<String>,
    directory: &Path,
    tracked_set: &BTreeSet<String>,
) {
    if let Some(path) = directory_instruction_file(directory, tracked_set) {
        chain.push(path);
    }
}

fn directory_instruction_file(directory: &Path, tracked_set: &BTreeSet<String>) -> Option<String> {
    for filename in ["AGENTS.override.md", "AGENTS.md"] {
        let path = directory.join(filename).to_string_lossy().into_owned();
        if tracked_set.contains(path.as_str()) && Path::new(path.as_str()).is_file() {
            return Some(path);
        }
    }
    None
}

fn is_agent_instruction_file(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|file_name| matches!(file_name, "AGENTS.md" | "AGENTS.override.md"))
}

fn directory_label(directory: &Path) -> String {
    if directory.as_os_str().is_empty() {
        "root".to_owned()
    } else {
        directory.display().to_string()
    }
}

fn require_chain_size(label: &str, paths: &[String]) -> CheckResult {
    let mut total_bytes = paths.len().saturating_sub(1) as u64 * 2;
    for path in paths {
        total_bytes += fs::metadata(path)
            .map_err(|error| format!("stat {path}: {error}"))?
            .len();
    }
    if total_bytes <= MAX_AGENTS_CHAIN_BYTES {
        Ok(())
    } else {
        Err(format!(
            "{label} AGENTS.md chain is {total_bytes} bytes, above Codex default project_doc_max_bytes {MAX_AGENTS_CHAIN_BYTES}"
        ))
    }
}
