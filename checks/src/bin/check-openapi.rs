//! `OpenAPI` score verification for the public docs contract.

/// Propagate a failed `OpenAPI` check without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

/// Pinned Vacuum installation and execution.
#[path = "check-openapi/vacuum.rs"]
pub mod vacuum;

use reqwest as _;
use serde::Deserialize;
use serde_json::{self as _, from_str};
use std::{
    fs::{metadata, read_to_string},
    io::{Result as InputOutputResult, Write as _, stderr},
    path::{Path, PathBuf},
    process::ExitCode,
};
use tovuk_public_checks::check_support::{
    CheckResult, display_path, git_tracked_files, repo_root, tool_path,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x7] = [
    size_of_val(&discover_openapi_files),
    size_of_val(&docs_openapi_path),
    size_of_val(&has_openapi_extension),
    size_of_val(&has_openapi_marker),
    size_of_val(&is_openapi_file),
    size_of_val(&require_docs_openapi),
    size_of_val(&run),
];

/// Documentation API settings consumed by the checker.
#[derive(Debug, Deserialize)]
struct DocsApi {
    /// Path to the docs `OpenAPI` source.
    openapi: String,
}

/// Relevant root documentation configuration.
#[derive(Debug, Deserialize)]
struct DocsJson {
    /// Documentation API configuration.
    api: DocsApi,
}

/// Discover tracked `OpenAPI` and Swagger JSON or YAML files.
///
/// # Errors
///
/// Returns an error when Git cannot list tracked repository files.
fn discover_openapi_files(repository: &Path) -> CheckResult<Vec<String>> {
    let mut openapi_files = check_try!(git_tracked_files(repository))
        .into_iter()
        .filter(|candidate| return is_openapi_file(candidate))
        .collect::<Vec<_>>();
    openapi_files.sort();
    openapi_files.dedup();
    return Ok(openapi_files);
}

/// Read the `OpenAPI` path configured in `docs/docs.json`.
///
/// # Errors
///
/// Returns an error when configuration cannot be read or parsed, or when the
/// configured path is empty.
fn docs_openapi_path() -> CheckResult<PathBuf> {
    let source = check_try!(
        read_to_string("docs/docs.json")
            .map_err(|error| return format!("read docs/docs.json: {error}"))
    );
    let docs = check_try!(
        from_str::<DocsJson>(source.as_str())
            .map_err(|error| return format!("docs/docs.json must be valid JSON: {error}"))
    );
    let openapi = docs.api.openapi.trim();
    if openapi.is_empty() {
        return Err("docs/docs.json must set api.openapi".to_owned());
    }
    return Ok(Path::new("docs").join(openapi));
}

/// Return whether a path has a supported JSON or YAML extension.
fn has_openapi_extension(path: &str) -> bool {
    return matches!(
        Path::new(path)
            .extension()
            .and_then(|extension| return extension.to_str()),
        Some("json" | "yaml" | "yml")
    );
}

/// Return whether a path component identifies `OpenAPI` or Swagger.
fn has_openapi_marker(path: &str) -> bool {
    return path
        .split(['/', '.', '_', '-'])
        .any(|part| matches!(part, "openapi" | "swagger"));
}

/// Return whether a tracked path is an `OpenAPI` source.
fn is_openapi_file(path: &str) -> bool {
    let lowercase = path.to_ascii_lowercase();
    return has_openapi_marker(lowercase.as_str()) && has_openapi_extension(lowercase.as_str());
}

/// Execute the `OpenAPI` checker and report failures on standard error.
///
/// # Errors
///
/// Returns an error when a command failure cannot be written to standard error.
fn main() -> InputOutputResult<ExitCode> {
    match run() {
        Ok(()) => return Ok(ExitCode::SUCCESS),
        Err(error) => {
            return writeln!(stderr().lock(), "{error}").map(|()| return ExitCode::FAILURE);
        }
    }
}

/// Require the docs-configured `OpenAPI` source to exist in discovery results.
///
/// # Errors
///
/// Returns an error when the configured path is not a regular file, no sources
/// are discovered, or the configured source is not tracked.
fn require_docs_openapi(docs_path: &Path, discovered: &[String]) -> CheckResult {
    let docs_metadata = check_try!(metadata(docs_path).map_err(|error| {
        return format!(
            "Missing OpenAPI file referenced by docs/docs.json: {} ({error})",
            docs_path.display()
        );
    }));
    if !docs_metadata.is_file() {
        return Err(format!(
            "OpenAPI path referenced by docs/docs.json is not a file: {}",
            docs_path.display()
        ));
    }
    if discovered.is_empty() {
        return Err("No OpenAPI files found.".to_owned());
    }
    let configured_path = display_path(docs_path);
    if !discovered
        .iter()
        .any(|candidate| return candidate == &configured_path)
    {
        return Err(format!(
            "docs/docs.json references {configured_path}, but it was not discovered as an OpenAPI file."
        ));
    }
    return Ok(());
}

/// Install the pinned Vacuum binary and lint every tracked `OpenAPI` source.
///
/// # Errors
///
/// Returns an error when discovery, installation, version verification, or lint
/// execution fails.
fn run() -> CheckResult {
    let repository = check_try!(repo_root());
    let path = tool_path();
    let docs_path = check_try!(docs_openapi_path());
    let openapi_files = check_try!(discover_openapi_files(repository.as_path()));
    check_try!(require_docs_openapi(docs_path.as_path(), &openapi_files,));
    let vacuum_version = vacuum::version();
    let vacuum_binary = check_try!(vacuum::install(
        repository.as_path(),
        path.as_os_str(),
        vacuum_version.as_str(),
    ));
    check_try!(vacuum::require_version(
        vacuum_binary.as_path(),
        path.as_os_str(),
        vacuum_version.as_str(),
    ));
    return vacuum::run_lint(vacuum_binary.as_path(), path.as_os_str(), &openapi_files);
}
