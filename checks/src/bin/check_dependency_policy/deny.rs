//! Secure temporary inputs and cargo-deny execution.

use core::{
    ops::Range,
    slice,
    sync::atomic::{AtomicU64, Ordering},
};

use std::{
    env,
    ffi::OsStr,
    fs::{OpenOptions, read_to_string, remove_file},
    io::{self as input_output, Write as _},
    path::{Path, PathBuf},
    process::id as process_id,
};

use tovuk_public_checks::{
    check_support::{CheckResult, command},
    check_try,
};

use super::{BASE_DENY_CONFIG, ManifestSnapshots, TargetSnapshot, graph, policy};

/// Monotonic suffix used with atomic exclusive file creation.
static TEMPORARY_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0x0000);

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&cargo_deny_arguments),
    size_of_val(&cleanup_pair),
    size_of_val(&lock_wide_arguments),
    size_of_val(&TemporaryFile::create_new),
    size_of_val(&TemporaryFile::create_pair),
    size_of_val(&remove_temporary),
    size_of_val(&run_cargo_deny),
    size_of_val(&run_lock_wide),
    size_of_val(&run_target),
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

/// Build one target-specific cargo-deny command line.
pub(super) fn cargo_deny_arguments(
    manifest: &str,
    config_path: &str,
    metadata_path: &str,
    triple: &str,
) -> Vec<String> {
    return [
        "--manifest-path",
        manifest,
        "--metadata-path",
        metadata_path,
        "--config",
        config_path,
        "--target",
        triple,
        "--all-features",
        "--locked",
        "check",
        "--deny",
        "warnings",
        "all",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
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

/// Build one lock-wide cargo-deny security, license, and source command line.
pub(super) fn lock_wide_arguments(
    manifest: &str,
    config_path: &str,
    metadata_path: &str,
) -> Vec<String> {
    return [
        "--manifest-path",
        manifest,
        "--workspace",
        "--metadata-path",
        metadata_path,
        "--config",
        config_path,
        "--all-features",
        "--locked",
        "check",
        "--deny",
        "warnings",
        "advisories",
        "licenses",
        "sources",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
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
    manifest_snapshots: &ManifestSnapshots,
) -> CheckResult {
    check_try!(run_lock_wide(repository, path, manifest_snapshots));
    for snapshot in &manifest_snapshots.snapshots {
        check_try!(run_target(
            repository,
            path,
            manifest_snapshots.manifest.as_str(),
            snapshot
        ));
    }
    return Ok(());
}

/// Audit every locked package without applying active-graph ban checks.
///
/// # Errors
///
/// Returns an error when base policy loading, cargo-deny, or cleanup fails.
fn run_lock_wide(
    repository: &Path,
    path: &OsStr,
    manifest_snapshots: &ManifestSnapshots,
) -> CheckResult {
    let config = check_try!(
        read_to_string(repository.join(BASE_DENY_CONFIG))
            .map_err(|error| return format!("read {BASE_DENY_CONFIG}: {error}"))
    );
    let manifest = manifest_snapshots.manifest.as_str();
    let label = format!("{}-lock-wide", manifest.replace(['/', '.'], "-"));
    let [config_file, metadata_file] = check_try!(TemporaryFile::create_pair(
        label.as_str(),
        config.as_bytes(),
        manifest_snapshots.locked_metadata_json.as_slice(),
    ));
    let config_path = config_file.path.to_string_lossy().into_owned();
    let metadata_path = metadata_file.path.to_string_lossy().into_owned();
    let arguments = lock_wide_arguments(manifest, config_path.as_str(), metadata_path.as_str());
    let status_result = command(repository, path, "cargo")
        .arg("deny")
        .args(arguments)
        .status()
        .map_err(|error| return format!("run lock-wide cargo deny for {manifest}: {error}"));
    check_try!(cleanup_pair(&config_file, &metadata_file));
    let status = check_try!(status_result);
    if status.success() {
        return Ok(());
    }
    return Err(format!(
        "lock-wide cargo deny for {manifest} failed with status {status}"
    ));
}

/// Run cargo-deny against one exact shipped-target graph.
///
/// # Errors
///
/// Returns an error when configuration, execution, or cleanup fails.
fn run_target(
    repository: &Path,
    path: &OsStr,
    manifest: &str,
    snapshot: &TargetSnapshot,
) -> CheckResult {
    let features = check_try!(graph::feature_union(slice::from_ref(snapshot)));
    let config = check_try!(policy::render_deny_config(repository, &features));
    let manifest_label = manifest.replace(['/', '.'], "-");
    let label = format!("{manifest_label}-{}", snapshot.triple);
    let [config_file, metadata_file] = check_try!(TemporaryFile::create_pair(
        label.as_str(),
        config.as_bytes(),
        snapshot.metadata_json.as_slice(),
    ));
    let config_path = config_file.path.to_string_lossy().into_owned();
    let metadata_path = metadata_file.path.to_string_lossy().into_owned();
    let arguments = cargo_deny_arguments(
        manifest,
        config_path.as_str(),
        metadata_path.as_str(),
        snapshot.triple.as_str(),
    );
    let status_result = command(repository, path, "cargo")
        .arg("deny")
        .args(arguments)
        .status()
        .map_err(|error| {
            return format!("run cargo deny for {manifest} {}: {error}", snapshot.triple);
        });
    check_try!(cleanup_pair(&config_file, &metadata_file));
    let status = check_try!(status_result);
    if status.success() {
        return Ok(());
    }
    return Err(format!(
        "cargo deny for {manifest} {} failed with status {status}",
        snapshot.triple
    ));
}
