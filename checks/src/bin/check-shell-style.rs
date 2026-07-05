//! Shell style verification for the public repository.

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
    let shell_sources = collect_shell_sources(repo_root.as_path())?;
    let shell_entrypoints = collect_shell_entrypoints(repo_root.as_path())?;

    run_shell_tool(
        repo_root.as_path(),
        path.as_os_str(),
        "bash",
        &["-n"],
        shell_sources.as_slice(),
    )?;
    run_shell_tool(
        repo_root.as_path(),
        path.as_os_str(),
        "shellcheck",
        &["-x"],
        shell_entrypoints.as_slice(),
    )?;
    run_shell_tool(
        repo_root.as_path(),
        path.as_os_str(),
        "shfmt",
        &["-i", "2", "-ci", "-d"],
        shell_sources.as_slice(),
    )
}

fn collect_shell_sources(repo_root: &Path) -> CheckResult<Vec<PathBuf>> {
    let mut sources = collect_shell_files(repo_root, Path::new("scripts"))?;
    sources.extend(collect_shell_files(repo_root, Path::new("scripts/lib"))?);
    sources.sort();
    sources.dedup();
    Ok(sources)
}

fn collect_shell_entrypoints(repo_root: &Path) -> CheckResult<Vec<PathBuf>> {
    collect_shell_files(repo_root, Path::new("scripts"))
}

fn collect_shell_files(repo_root: &Path, relative_dir: &Path) -> CheckResult<Vec<PathBuf>> {
    let absolute_dir = repo_root.join(relative_dir);
    let mut files = Vec::new();
    let entries = fs::read_dir(absolute_dir.as_path())
        .map_err(|error| format!("read {}: {error}", relative_dir.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("read entry in {}: {error}", relative_dir.display()))?;
        if !entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", entry.path().display()))?
            .is_file()
        {
            continue;
        }
        if entry.path().extension() == Some(OsStr::new("sh")) {
            files.push(relative_dir.join(entry.file_name()));
        }
    }
    files.sort();
    Ok(files)
}

fn run_shell_tool(
    repo_root: &Path,
    path: &OsStr,
    program: &str,
    fixed_args: &[&str],
    files: &[PathBuf],
) -> CheckResult {
    let status = command(repo_root, path, program)
        .args(fixed_args)
        .args(files)
        .status()
        .map_err(|error| format!("run {program}: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{program} failed with status {status}"))
}
