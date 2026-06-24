use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::helpers::CheckResult;

const MAX_AGENTS_CHAIN_BYTES: u64 = 32_768;

pub(crate) fn check_chain_sizes(tracked_files: &[String]) -> CheckResult {
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
