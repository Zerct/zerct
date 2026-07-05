//! TOML style verification for the public repository.

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use tovuk_public_checks::check_support::{CheckResult, command, repo_root, tool_path};

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
    let toml_files = collect_toml_files(repo_root.as_path())?;
    if toml_files.is_empty() {
        return Ok(());
    }
    run_taplo(
        repo_root.as_path(),
        path.as_os_str(),
        &["format", "--check"],
        toml_files.as_slice(),
    )?;
    run_taplo(
        repo_root.as_path(),
        path.as_os_str(),
        &["lint", "--no-schema"],
        toml_files.as_slice(),
    )
}

fn collect_toml_files(repo_root: &Path) -> CheckResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_toml_files_in(repo_root, Path::new("."), &mut files)?;
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_toml_files_in(
    repo_root: &Path,
    relative_dir: &Path,
    files: &mut Vec<PathBuf>,
) -> CheckResult {
    let absolute_dir = if relative_dir == Path::new(".") {
        repo_root.to_path_buf()
    } else {
        repo_root.join(relative_dir)
    };
    let mut entries = fs::read_dir(absolute_dir.as_path())
        .map_err(|error| format!("read {}: {error}", relative_dir.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read entry in {}: {error}", relative_dir.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let relative_path = if relative_dir == Path::new(".") {
            PathBuf::from(entry.file_name())
        } else {
            relative_dir.join(entry.file_name())
        };
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", relative_path.display()))?;
        if file_type.is_dir() {
            if !is_pruned_dir(relative_path.as_path()) {
                collect_toml_files_in(repo_root, relative_path.as_path(), files)?;
            }
        } else if file_type.is_file() && relative_path.extension() == Some(OsStr::new("toml")) {
            files.push(relative_path);
        }
    }
    Ok(())
}

fn is_pruned_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| matches!(name, ".git" | "target" | "node_modules"))
}

fn run_taplo(
    repo_root: &Path,
    path: &OsStr,
    fixed_args: &[&str],
    toml_files: &[PathBuf],
) -> CheckResult {
    let status = command(repo_root, path, "taplo")
        .args(fixed_args)
        .args(toml_files)
        .status()
        .map_err(|error| format!("run taplo: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("taplo failed with status {status}"))
}
