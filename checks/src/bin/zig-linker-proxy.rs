//! Fail-closed Zig delegate for the strict Linux ARM64 release link.

/// Propagate a proxy failure without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

#[cfg(test)]
#[path = "zig_linker_proxy_tests/verification.rs"]
mod tests;

use core::{
    ops::Range,
    str::from_utf8,
    sync::atomic::{AtomicU64, Ordering},
};

use flate2 as _;
use http as _;
use http_body_util as _;
use hyper as _;
use hyper_rustls as _;
use hyper_util as _;
use rustls as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use tar as _;
use tokio as _;
use tovuk_public_checks as _;
use url as _;

use std::{
    env,
    ffi::{OsStr, OsString},
    fs::{File, OpenOptions, canonicalize, metadata, remove_file},
    io::{self as input_output, Read as _, Write as _, stderr},
    path::{Path, PathBuf},
    process::{Command, ExitCode, ExitStatus, id as process_id},
};

/// Exact linker option that Zig 0.16 ignores after emitting a warning.
const DEPRECATED_LINKER_OPTIMIZATION: &str = "-Wl,-O1";

/// Maximum response-file size accepted by the release-only proxy.
const MAX_RESPONSE_FILE_BYTES: usize = 0x0100_0000;

/// `u64` form used for file metadata before allocation.
const MAX_RESPONSE_FILE_BYTES_U64: u64 = 0x0100_0000;

/// Environment variable containing the pinned real Zig executable.
const REAL_ZIG_PATH_ENVIRONMENT: &str = "TOVUK_REAL_ZIG_PATH";

/// Monotonic suffix used with exclusive temporary-file creation.
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0x0000);

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000f] = [
    size_of_val(&child_exit_code),
    size_of_val(&cleanup_failed_temporary_operation),
    size_of_val(&compiler_subcommand),
    size_of_val(&contains_deprecated_option),
    size_of_val(&create_temporary_response),
    size_of_val(&open_private_file),
    size_of_val(&prepare_arguments),
    size_of_val(&prepare_compiler_argument),
    size_of_val(&read_response_file),
    size_of_val(&real_zig_path),
    size_of_val(&response_argument),
    size_of_val(&run),
    size_of_val(&run_real_zig),
    size_of_val(&sanitize_response_contents),
    size_of_val(&write_temporary_response),
];

/// Optional sanitized compiler argument.
type PreparedArgument = Option<OsString>;

/// Command arguments and temporary response files prepared for real Zig.
#[derive(Debug)]
struct PreparedArguments {
    /// Arguments delegated to the real Zig executable.
    arguments: Vec<OsString>,
    /// Sanitized response files removed after Zig exits.
    temporary_files: Vec<TemporaryResponseFile>,
}

impl PreparedArguments {
    /// Remove every temporary response file.
    ///
    /// # Errors
    /// Returns an error when any temporary response file cannot be removed.
    fn cleanup(self) -> ProxyResult<()> {
        let errors = self
            .temporary_files
            .into_iter()
            .filter_map(|temporary| return temporary.cleanup().err())
            .collect::<Vec<_>>();
        if errors.is_empty() {
            return Ok(());
        }
        return Err(errors.join("; "));
    }
}

/// Result returned by every fallible proxy operation.
type ProxyResult<Value> = Result<Value, String>;

/// Optional sanitized response bytes.
type SanitizedResponse = Option<Vec<u8>>;

/// Exclusively created sanitized response file.
#[derive(Debug)]
struct TemporaryResponseFile {
    /// Temporary file path delegated to Zig.
    path: PathBuf,
}

impl TemporaryResponseFile {
    /// Remove this response file without following another path.
    ///
    /// # Errors
    /// Returns an error when removal fails.
    fn cleanup(self) -> ProxyResult<()> {
        return remove_file(self.path.as_path())
            .map_err(|error| return format!("remove {}: {error}", self.path.display()));
    }
}

/// Optional exclusively created temporary response file.
type WrittenTemporaryResponse = Option<TemporaryResponseFile>;

/// Convert one child status into a portable process exit code.
fn child_exit_code(status: ExitStatus) -> ExitCode {
    if status.success() {
        return ExitCode::SUCCESS;
    }
    let Some(raw_code) = status.code() else {
        return ExitCode::FAILURE;
    };
    return match u8::try_from(raw_code) {
        Ok(code) => ExitCode::from(code),
        Err(_conversion_error) => ExitCode::FAILURE,
    };
}

/// Remove a partially written response file and preserve both diagnostics.
fn cleanup_failed_temporary_operation(
    path: &Path,
    operation: &str,
    operation_error: &input_output::Error,
) -> String {
    let failure = format!("{operation} {}: {operation_error}", path.display());
    return match remove_file(path) {
        Ok(()) => failure,
        Err(cleanup_error) => {
            format!("{failure}; remove {}: {cleanup_error}", path.display())
        }
    };
}

/// Return whether arguments select Zig's C or C++ driver.
fn compiler_subcommand(arguments: &[OsString]) -> bool {
    return matches!(
        arguments
            .first()
            .and_then(|argument| return argument.to_str()),
        Some("cc" | "c++")
    );
}

/// Return whether encoded argument bytes contain the deprecated exact token.
fn contains_deprecated_option(argument: &OsStr) -> bool {
    let needle = DEPRECATED_LINKER_OPTIMIZATION.as_bytes();
    return argument
        .as_encoded_bytes()
        .windows(needle.len())
        .any(|window| return window == needle);
}

/// Reserve and write one unique private response file.
///
/// # Errors
/// Returns an error when no unique response path can be created or written.
fn create_temporary_response(contents: &[u8]) -> ProxyResult<TemporaryResponseFile> {
    let attempts: Range<u8> = 0x0000..0x0040;
    let candidate_result = attempts
        .map(|attempt| {
            let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(0x0001, Ordering::Relaxed);
            let file_name = format!(
                "tovuk-zig-linker-proxy-{}-{sequence}-{attempt}.rsp",
                process_id(),
            );
            return write_temporary_response(env::temp_dir().join(file_name), contents);
        })
        .find(|result| return !matches!(result, Ok(None)));
    return match candidate_result {
        Some(Ok(Some(temporary))) => Ok(temporary),
        Some(Err(error)) => Err(error),
        None | Some(Ok(None)) => {
            Err("could not reserve a unique Zig response-file path".to_owned())
        }
    };
}

/// Report one proxy failure without hiding the real Zig failure stream.
///
/// # Errors
/// Returns an error when standard error cannot be written.
fn main() -> input_output::Result<ExitCode> {
    match run() {
        Ok(exit_code) => return Ok(exit_code),
        Err(error) => {
            return writeln!(stderr().lock(), "zig-linker-proxy: {error}")
                .map(|()| return ExitCode::FAILURE);
        }
    }
}

/// Open one private file with exclusive creation.
///
/// # Errors
/// Returns an error when the path cannot be created exclusively.
fn open_private_file(path: &Path) -> input_output::Result<File> {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        return options.write(true).create_new(true).mode(0o600).open(path);
    }
    #[cfg(not(unix))]
    {
        return options.write(true).create_new(true).open(path);
    }
}

/// Prepare all Zig arguments, filtering only compiler-driver invocations.
///
/// # Errors
/// Returns an error when an argument cannot be sanitized unambiguously.
fn prepare_arguments(arguments: Vec<OsString>) -> ProxyResult<PreparedArguments> {
    if !compiler_subcommand(arguments.as_slice()) {
        return Ok(PreparedArguments {
            arguments,
            temporary_files: Vec::new(),
        });
    }
    let mut prepared = PreparedArguments {
        arguments: Vec::with_capacity(arguments.len()),
        temporary_files: Vec::new(),
    };
    for argument in arguments {
        let filtered_result = prepare_compiler_argument(argument, &mut prepared.temporary_files);
        let filtered = match filtered_result {
            Ok(value) => value,
            Err(error) => {
                return match prepared.cleanup() {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(format!("{error}; cleanup failed: {cleanup_error}")),
                };
            }
        };
        if let Some(filtered_argument) = filtered {
            prepared.arguments.push(filtered_argument);
        }
    }
    return Ok(prepared);
}

/// Prepare one compiler-driver argument.
///
/// # Errors
/// Returns an error for ambiguous deprecated-option forms or invalid response
/// files.
fn prepare_compiler_argument(
    argument: OsString,
    temporary_files: &mut Vec<TemporaryResponseFile>,
) -> ProxyResult<PreparedArgument> {
    if argument == OsStr::new(DEPRECATED_LINKER_OPTIMIZATION) {
        return Ok(None);
    }
    if argument.as_encoded_bytes().first() != Some(&b'@') {
        if contains_deprecated_option(argument.as_os_str()) {
            let rendered = argument.to_string_lossy();
            return Err(format!(
                "refusing ambiguous linker option {rendered}; only exact {DEPRECATED_LINKER_OPTIMIZATION:?} may be removed",
            ));
        }
        return Ok(Some(argument));
    }
    let response_text = check_try!(argument.to_str().ok_or_else(|| {
        return "response-file argument must be valid UTF-8".to_owned();
    }));
    let response_path = check_try!(
        response_text
            .strip_prefix('@')
            .filter(|path| return !path.is_empty())
            .map(Path::new)
            .ok_or_else(|| return "response-file argument must name a path".to_owned())
    );
    let contents = check_try!(read_response_file(response_path));
    let Some(sanitized) = check_try!(sanitize_response_contents(contents.as_slice())) else {
        return Ok(Some(argument));
    };
    let temporary = check_try!(create_temporary_response(sanitized.as_slice()));
    let filtered_argument = response_argument(temporary.path.as_path());
    temporary_files.push(temporary);
    return Ok(Some(filtered_argument));
}

/// Read one bounded UTF-8 response file.
///
/// # Errors
/// Returns an error when the response file is unreadable, oversized, or not
/// valid UTF-8.
fn read_response_file(path: &Path) -> ProxyResult<Vec<u8>> {
    let file_result = File::open(path)
        .map_err(|error| return format!("open response file {}: {error}", path.display()));
    let file = check_try!(file_result);
    let response_size = check_try!(file.metadata().map_err(|error| {
        return format!("inspect response file {}: {error}", path.display());
    }))
    .len();
    if response_size > MAX_RESPONSE_FILE_BYTES_U64 {
        return Err(format!(
            "response file {} exceeds {MAX_RESPONSE_FILE_BYTES} bytes",
            path.display()
        ));
    }
    let mut bytes = Vec::new();
    let mut bounded_file = file.take(0x0100_0001);
    let bytes_read = check_try!(bounded_file.read_to_end(&mut bytes).map_err(|error| {
        return format!("read response file {}: {error}", path.display());
    }));
    if bytes_read > MAX_RESPONSE_FILE_BYTES {
        return Err(format!(
            "response file {} exceeds {MAX_RESPONSE_FILE_BYTES} bytes",
            path.display()
        ));
    }
    if from_utf8(bytes.as_slice()).is_err() {
        return Err(format!(
            "response file {} must contain valid UTF-8",
            path.display()
        ));
    }
    return Ok(bytes);
}

/// Resolve and validate the pinned real Zig executable.
///
/// # Errors
/// Returns an error when the real Zig path is absent, invalid, or recursive.
fn real_zig_path() -> ProxyResult<PathBuf> {
    let configured = check_try!(
        env::var_os(REAL_ZIG_PATH_ENVIRONMENT)
            .filter(|value| return !value.is_empty())
            .ok_or_else(|| return format!("{REAL_ZIG_PATH_ENVIRONMENT} must name pinned Zig"))
    );
    let configured_path = PathBuf::from(configured);
    let file_metadata = check_try!(metadata(configured_path.as_path()).map_err(|error| {
        return format!(
            "inspect {REAL_ZIG_PATH_ENVIRONMENT} {}: {error}",
            configured_path.display()
        );
    }));
    if !file_metadata.is_file() {
        return Err(format!(
            "{REAL_ZIG_PATH_ENVIRONMENT} {} must be a file",
            configured_path.display()
        ));
    }
    let real_path = check_try!(canonicalize(configured_path.as_path()).map_err(|error| {
        return format!("resolve real Zig {}: {error}", configured_path.display());
    }));
    let current_path = check_try!(
        env::current_exe().map_err(|error| return format!("resolve proxy executable: {error}"))
    );
    let canonical_proxy = check_try!(
        canonicalize(current_path.as_path()).map_err(|error| return format!(
            "resolve proxy path {}: {error}",
            current_path.display()
        ))
    );
    if real_path == canonical_proxy {
        return Err(format!(
            "{REAL_ZIG_PATH_ENVIRONMENT} must not point back to zig-linker-proxy"
        ));
    }
    return Ok(real_path);
}

/// Form an `@path` argument without requiring a UTF-8 temporary directory.
fn response_argument(path: &Path) -> OsString {
    let mut argument = OsString::from("@");
    argument.push(path.as_os_str());
    return argument;
}

/// Validate, prepare, and delegate this invocation to pinned real Zig.
///
/// # Errors
/// Returns an error when validation, delegation, or cleanup fails.
fn run() -> ProxyResult<ExitCode> {
    let real_zig = check_try!(real_zig_path());
    let prepared = check_try!(prepare_arguments(env::args_os().skip(0x1).collect()));
    return run_real_zig(real_zig.as_path(), prepared);
}

/// Delegate prepared arguments and remove all temporary response files.
///
/// # Errors
/// Returns an error when Zig cannot run or temporary cleanup fails.
fn run_real_zig(real_zig: &Path, prepared: PreparedArguments) -> ProxyResult<ExitCode> {
    let status_result = Command::new(real_zig)
        .args(&prepared.arguments)
        .status()
        .map_err(|error| return format!("run real Zig {}: {error}", real_zig.display()));
    let cleanup_result = prepared.cleanup();
    return match (status_result, cleanup_result) {
        (Ok(status), Ok(())) => Ok(child_exit_code(status)),
        (Err(run_error), Ok(())) => Err(run_error),
        (Ok(_status), Err(cleanup_error)) => Err(cleanup_error),
        (Err(run_error), Err(cleanup_error)) => {
            Err(format!("{run_error}; cleanup failed: {cleanup_error}"))
        }
    };
}

/// Remove exact deprecated linker-option lines from one response file.
///
/// # Errors
/// Returns an error when the response is not UTF-8 or contains an ambiguous
/// deprecated-option form.
fn sanitize_response_contents(contents: &[u8]) -> ProxyResult<SanitizedResponse> {
    let text = check_try!(
        from_utf8(contents)
            .map_err(|error| return format!("decode response file as UTF-8: {error}"))
    );
    let mut removed = false;
    let mut sanitized = Vec::with_capacity(contents.len());
    for segment in text.split_inclusive('\n') {
        let without_newline = segment.strip_suffix('\n').unwrap_or(segment);
        let logical_line = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        if logical_line == DEPRECATED_LINKER_OPTIMIZATION {
            removed = true;
            continue;
        }
        if logical_line.contains(DEPRECATED_LINKER_OPTIMIZATION) {
            return Err(format!(
                "refusing ambiguous response-file linker option {logical_line:?}; only an exact line may be removed"
            ));
        }
        sanitized.extend_from_slice(segment.as_bytes());
    }
    if removed {
        return Ok(Some(sanitized));
    }
    return Ok(None);
}

/// Exclusively create one temporary response candidate and write its bytes.
///
/// # Errors
/// Returns an error for I/O failures other than an occupied candidate.
fn write_temporary_response(
    path: PathBuf,
    contents: &[u8],
) -> ProxyResult<WrittenTemporaryResponse> {
    let mut file = match open_private_file(path.as_path()) {
        Ok(file) => file,
        Err(error) if error.kind() == input_output::ErrorKind::AlreadyExists => return Ok(None),
        Err(error) => return Err(format!("create {}: {error}", path.display())),
    };
    if let Err(error) = file.write_all(contents) {
        drop(file);
        return Err(cleanup_failed_temporary_operation(
            path.as_path(),
            "write",
            &error,
        ));
    }
    if let Err(error) = file.sync_all() {
        drop(file);
        return Err(cleanup_failed_temporary_operation(
            path.as_path(),
            "sync",
            &error,
        ));
    }
    drop(file);
    return Ok(Some(TemporaryResponseFile { path }));
}
