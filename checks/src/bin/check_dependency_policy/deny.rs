//! Secure temporary inputs and cargo-deny execution.

use core::{
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
};

use std::{
    env,
    ffi::OsStr,
    fs::{OpenOptions, remove_file},
    io::{self as input_output, Write as _},
    path::{Path, PathBuf},
    process::id as process_id,
};

use tovuk_public_checks::{
    check_support::{CheckResult, command},
    check_try,
};

use super::{TargetSnapshot, graph, policy};

/// Monotonic suffix used with atomic exclusive file creation.
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0x0000);

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&cleanup_pair),
    size_of_val(&TemporaryFile::create_new),
    size_of_val(&TemporaryFile::create_pair),
    size_of_val(&remove_temporary),
    size_of_val(&run_cargo_deny),
];

/// Temporary file removed after its bounded operation.
#[derive(Debug)]
pub(super) struct TemporaryFile {
    /// Absolute file path.
    path: PathBuf,
}

impl TemporaryFile {
    /// Write a uniquely named temporary file.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be written.
    fn create(label: &str, bytes: &[u8]) -> CheckResult<Self> {
        let attempts: Range<u8> = 0x0000..0x0040;
        let candidate_result = attempts
            .map(|attempt| {
                let sequence = TEMPORARY_FILE_SEQUENCE.fetch_add(0x0001, Ordering::Relaxed);
                let file_name = format!(
                    "tovuk-dependency-policy-{}-{sequence}-{attempt}-{label}",
                    process_id(),
                );
                return Self::create_new(env::temp_dir().join(file_name), bytes);
            })
            .find(|result| return !matches!(result, Ok(None)));
        return match candidate_result {
            Some(Ok(Some(temporary))) => Ok(temporary),
            Some(Err(error)) => Err(error),
            None | Some(Ok(None)) => {
                Err("could not reserve a unique dependency-policy temporary file".to_owned())
            }
        };
    }

    /// Atomically create one candidate without following an existing path.
    ///
    /// # Errors
    ///
    /// Returns an error for I/O failures other than an occupied candidate.
    pub(super) fn create_new(path: PathBuf, bytes: &[u8]) -> CheckResult<Option<Self>> {
        let opened = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.as_path());
        let mut file = match opened {
            Ok(file) => file,
            Err(error) if error.kind() == input_output::ErrorKind::AlreadyExists => {
                return Ok(None);
            }
            Err(error) => return Err(format!("create {}: {error}", path.display())),
        };
        if let Err(error) = file.write_all(bytes) {
            drop(remove_file(path.as_path()));
            return Err(format!("write {}: {error}", path.display()));
        }
        let temporary = Self { path };
        return Ok(Some(temporary));
    }

    /// Create the temporary cargo-deny configuration and metadata pair.
    ///
    /// # Errors
    ///
    /// Returns an error when either file cannot be created or cleanup fails.
    fn create_pair(label: &str, config: &[u8], metadata: &[u8]) -> CheckResult<[Self; 0x0002]> {
        let config_file = check_try!(Self::create(format!("{label}-deny.toml").as_str(), config));
        let metadata_result = Self::create(format!("{label}-metadata.json").as_str(), metadata);
        return match metadata_result {
            Ok(metadata_file) => Ok([config_file, metadata_file]),
            Err(error) => {
                check_try!(remove_temporary(&config_file));
                Err(error)
            }
        };
    }
}

/// Remove both temporary cargo-deny inputs while retaining every cleanup error.
///
/// # Errors
///
/// Returns an error when either temporary file cannot be removed.
fn cleanup_pair(config_file: &TemporaryFile, metadata_file: &TemporaryFile) -> CheckResult {
    let cleanup_errors = [
        remove_temporary(config_file),
        remove_temporary(metadata_file),
    ]
    .into_iter()
    .filter_map(Result::err)
    .collect::<Vec<_>>();
    if cleanup_errors.is_empty() {
        return Ok(());
    }
    return Err(cleanup_errors.join("\n"));
}
/// Remove a temporary file after its bounded operation completes.
///
/// # Errors
///
/// Returns an error when cleanup fails.
fn remove_temporary(temporary: &TemporaryFile) -> CheckResult {
    return remove_file(temporary.path.as_path())
        .map_err(|error| format!("remove {}: {error}", temporary.path.display()));
}

/// Run cargo-deny with validated metadata and a generated feature config.
///
/// # Errors
///
/// Returns an error when metadata generation, file creation, or cargo-deny fails.
pub(super) fn run_cargo_deny(
    repository: &Path,
    path: &OsStr,
    manifest: &str,
    snapshots: &[TargetSnapshot],
) -> CheckResult {
    let features = check_try!(graph::feature_union(snapshots));
    let config = check_try!(policy::render_deny_config(repository, &features));
    let metadata = check_try!(graph::cargo_metadata(repository, path, manifest, None));
    let label = manifest.replace(['/', '.'], "-");
    let [config_file, metadata_file] = check_try!(TemporaryFile::create_pair(
        label.as_str(),
        config.as_bytes(),
        metadata.stdout.as_slice(),
    ));
    let config_path = config_file.path.to_string_lossy().into_owned();
    let metadata_path = metadata_file.path.to_string_lossy().into_owned();
    let status_result = command(repository, path, "cargo")
        .args([
            "deny",
            "--manifest-path",
            manifest,
            "check",
            "--config",
            config_path.as_str(),
            "--metadata-path",
            metadata_path.as_str(),
            "all",
        ])
        .status()
        .map_err(|error| return format!("run cargo deny for {manifest}: {error}"));
    check_try!(cleanup_pair(&config_file, &metadata_file));
    let status = check_try!(status_result);
    if status.success() {
        return Ok(());
    }
    return Err(format!(
        "cargo deny for {manifest} failed with status {status}"
    ));
}
