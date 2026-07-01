use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::de::DeserializeOwned;

use crate::{helpers::CheckResult, types::PackageJson};

pub(crate) fn read_sorted_texts_recursive(
    directory: &str,
    suffix: &str,
) -> CheckResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_paths_with_suffix(Path::new(directory), suffix, &mut paths)?;
    paths.sort();

    paths
        .iter()
        .map(|path| read_text(path.as_path()))
        .collect::<CheckResult<Vec<_>>>()
}

fn collect_paths_with_suffix(root: &Path, suffix: &str, paths: &mut Vec<PathBuf>) -> CheckResult {
    let entries = fs::read_dir(root)
        .map_err(|error| format!("read directory {}: {error}", root.display()))?;
    for entry_result in entries {
        let entry = entry_result
            .map_err(|error| format!("read entry under {}: {error}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", path.display()))?;
        if file_type.is_dir() {
            collect_paths_with_suffix(path.as_path(), suffix, paths)?;
            continue;
        }
        if entry.file_name().to_string_lossy().ends_with(suffix) {
            paths.push(path);
        }
    }
    Ok(())
}

pub(crate) fn read_text(path: impl AsRef<Path>) -> CheckResult<String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

pub(crate) fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> CheckResult<T> {
    let path = path.as_ref();
    let source = read_text(path)?;
    serde_json::from_str(source.as_str())
        .map_err(|error| format!("parse {}: {error}", path.display()))
}

pub(crate) fn read_package_json(path: impl AsRef<Path>) -> CheckResult<PackageJson> {
    read_json(path)
}

pub(crate) fn env_int(name: &str, default_value: i64) -> CheckResult<i64> {
    let raw = std::env::var(name).unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(default_value);
    }
    trimmed
        .parse::<i64>()
        .map_err(|_| format!("{name} must be an integer"))
}

pub(crate) fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

pub(crate) fn must_abs(path: impl AsRef<Path>) -> CheckResult<String> {
    let path = path.as_ref();
    path.canonicalize()
        .map(|absolute| absolute.display().to_string())
        .map_err(|error| format!("resolve {}: {error}", path.display()))
}

pub(crate) fn find_repo_root() -> CheckResult<String> {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        }
        Ok(output) => Err(format!(
            "git rev-parse --show-toplevel failed with status {}",
            output.status
        )),
        Err(error) => Err(format!("run git rev-parse --show-toplevel: {error}")),
    }
}
