//! `OpenAPI` score verification for the public docs contract.

use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use serde::Deserialize;

use tovuk_public_checks::check_support::{
    CheckResult, display_path, git_tracked_files, repo_root, tool_path,
};

const DEFAULT_VACUUM_VERSION: &str = "0.26.6";

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
    let docs_openapi_path = docs_openapi_path()?;
    if !docs_openapi_path.is_file() {
        return Err(format!(
            "Missing OpenAPI file referenced by docs/docs.json: {}",
            docs_openapi_path.display()
        ));
    }

    let openapi_files = discover_openapi_files(repo_root.as_path())?;
    if openapi_files.is_empty() {
        return Err("No OpenAPI files found.".to_owned());
    }
    let docs_openapi = display_path(docs_openapi_path.as_path());
    if !openapi_files.iter().any(|path| path == &docs_openapi) {
        return Err(format!(
            "docs/docs.json references {docs_openapi}, but it was not discovered as an OpenAPI file."
        ));
    }

    let vacuum_version = vacuum_version();
    let vacuum_bin = install_vacuum(
        repo_root.as_path(),
        path.as_os_str(),
        vacuum_version.as_str(),
    )?;
    require_vacuum_version(
        vacuum_bin.as_path(),
        path.as_os_str(),
        vacuum_version.as_str(),
    )?;
    run_vacuum_lint(vacuum_bin.as_path(), path.as_os_str(), &openapi_files)
}

#[derive(Deserialize)]
struct DocsJson {
    api: DocsApi,
}

#[derive(Deserialize)]
struct DocsApi {
    openapi: String,
}

fn docs_openapi_path() -> CheckResult<PathBuf> {
    let source = std::fs::read_to_string("docs/docs.json")
        .map_err(|error| format!("read docs/docs.json: {error}"))?;
    let docs = serde_json::from_str::<DocsJson>(source.as_str())
        .map_err(|error| format!("docs/docs.json must be valid JSON: {error}"))?;
    let openapi = docs.api.openapi.trim();
    if openapi.is_empty() {
        return Err("docs/docs.json must set api.openapi".to_owned());
    }
    Ok(Path::new("docs").join(openapi))
}

fn discover_openapi_files(repo_root: &Path) -> CheckResult<Vec<String>> {
    let mut openapi_files = git_tracked_files(repo_root)?
        .into_iter()
        .filter(|path| is_openapi_file(path))
        .collect::<Vec<_>>();
    openapi_files.sort();
    openapi_files.dedup();
    Ok(openapi_files)
}

fn is_openapi_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    has_openapi_marker(lower.as_str()) && has_openapi_extension(lower.as_str())
}

fn has_openapi_marker(path: &str) -> bool {
    path.split(['/', '.', '_', '-'])
        .any(|part| matches!(part, "openapi" | "swagger"))
}

fn has_openapi_extension(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("json" | "yaml" | "yml")
    )
}

fn vacuum_version() -> String {
    env::var("VACUUM_VERSION")
        .unwrap_or_else(|_| DEFAULT_VACUUM_VERSION.to_owned())
        .trim_start_matches('v')
        .to_owned()
}

fn install_vacuum(repo_root: &Path, path: &std::ffi::OsStr, version: &str) -> CheckResult<PathBuf> {
    let output = Command::new("scripts/install-vacuum.sh")
        .current_dir(repo_root)
        .env("PATH", path)
        .env("VACUUM_VERSION", version)
        .output()
        .map_err(|error| format!("install Vacuum: {error}"))?;
    if !output.status.success() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(format!(
            "scripts/install-vacuum.sh failed with status {}",
            output.status
        ));
    }
    Ok(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim().to_owned(),
    ))
}

fn require_vacuum_version(
    vacuum_bin: &Path,
    path: &std::ffi::OsStr,
    required_version: &str,
) -> CheckResult {
    let output = Command::new(vacuum_bin)
        .arg("version")
        .env("PATH", path)
        .output()
        .map_err(|error| format!("read Vacuum version: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Vacuum version failed with status {}",
            output.status
        ));
    }
    let installed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if installed == required_version {
        Ok(())
    } else {
        Err(format!(
            "vacuum {required_version} is required; found {installed}."
        ))
    }
}

fn run_vacuum_lint(
    vacuum_bin: &Path,
    path: &std::ffi::OsStr,
    openapi_files: &[String],
) -> CheckResult {
    let status = Command::new(vacuum_bin)
        .args([
            "lint",
            "--ruleset",
            ".vacuum.yaml",
            "--hard-mode",
            "--fail-severity",
            "hint",
            "--min-score",
            "100",
            "--details",
            "--all-results",
            "--no-style",
            "--no-banner",
        ])
        .args(openapi_files)
        .env("PATH", path)
        .status()
        .map_err(|error| format!("run Vacuum lint: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("Vacuum lint failed with status {status}"))
}
