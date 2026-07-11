//! Vacuum binary installation and lint execution.

#[path = "vacuum_download.rs"]
mod download;

use core::fmt::Write as _;
use flate2::read::GzDecoder;
use sha2::{Digest as _, Sha256};
use std::{
    env,
    ffi::OsStr,
    fs::{copy, create_dir_all, remove_dir_all, remove_file, rename},
    io::Result as InputOutputResult,
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};
use tar::{Archive, Unpacked};
use tovuk_public_checks::check_support::CheckResult;

use download::download_asset;

/// Pinned Vacuum release used by public checks.
const DEFAULT_VACUUM_VERSION: &str = "0.26.6";

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x11] = [
    size_of_val(&VacuumHost::current),
    size_of_val(&create_temporary_directory),
    size_of_val(&extract_archive_binary),
    size_of_val(&extract_vacuum),
    size_of_val(&finish_temporary_extraction),
    size_of_val(&hex_lower),
    size_of_val(&install),
    size_of_val(&install_candidate),
    size_of_val(&install_dir),
    size_of_val(&make_executable),
    size_of_val(&move_installed_binary),
    size_of_val(&require_sha256),
    size_of_val(&require_version),
    size_of_val(&run_lint),
    size_of_val(&vacuum_asset_sha256),
    size_of_val(&vacuum_version_matches),
    size_of_val(&version),
];

/// Paths participating in one archive extraction.
#[derive(Debug)]
struct ExtractionPaths {
    /// Final installation directory.
    install_directory: PathBuf,
    /// Disposable extraction directory.
    temporary_directory: PathBuf,
    /// Final Vacuum executable path.
    vacuum_binary: PathBuf,
}

/// Supported Vacuum release host identifiers.
#[derive(Clone, Copy, Debug)]
struct VacuumHost {
    /// Vacuum release architecture name.
    architecture: &'static str,
    /// Vacuum release platform name.
    platform: &'static str,
}

impl VacuumHost {
    /// Detect the supported Vacuum release host.
    ///
    /// # Errors
    ///
    /// Returns an error when the operating system or architecture is unsupported.
    fn current() -> CheckResult<Self> {
        let platform = match env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            other => return Err(format!("unsupported vacuum host OS: {other}")),
        };
        let architecture = match env::consts::ARCH {
            "aarch64" => "arm64",
            "x86" => "i386",
            "x86_64" => "x86_64",
            other => return Err(format!("unsupported vacuum host architecture: {other}")),
        };
        return Ok(Self {
            architecture,
            platform,
        });
    }
}

/// Create an isolated Vacuum extraction directory.
///
/// # Errors
///
/// Returns an error when the installation parent is absent or directory creation
/// fails.
fn create_temporary_directory(install_directory: &Path) -> CheckResult<PathBuf> {
    let parent = check_try!(install_directory.parent().ok_or_else(|| {
        return format!(
            "{} must have a parent directory",
            install_directory.display()
        );
    }));
    check_try!(
        create_dir_all(parent)
            .map_err(|error| return format!("create {}: {error}", parent.display()))
    );
    let timestamp = check_try!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| return format!("system time before Unix epoch: {error}"))
    )
    .as_nanos();
    let temporary_directory = parent.join(format!("vacuum-download-{}-{timestamp}", process::id()));
    check_try!(
        create_dir_all(temporary_directory.as_path())
            .map_err(|error| return format!("create {}: {error}", temporary_directory.display()))
    );
    return Ok(temporary_directory);
}

/// Extract and install the Vacuum executable from a verified archive.
///
/// # Errors
///
/// Returns an error when archive reading, extraction, permission configuration,
/// or final installation fails.
fn extract_archive_binary(
    asset: &str,
    archive_bytes: &[u8],
    paths: &ExtractionPaths,
) -> CheckResult {
    let candidate = paths.temporary_directory.join("vacuum");
    let mut archive = Archive::new(GzDecoder::new(archive_bytes));
    let entries = check_try!(
        archive
            .entries()
            .map_err(|error| return format!("read {asset}: {error}"))
    );
    for archive_entry_result in entries {
        let mut archive_entry = check_try!(
            archive_entry_result.map_err(|error| return format!("read {asset} entry: {error}"))
        );
        if !archive_entry.header().entry_type().is_file() {
            continue;
        }
        let entry_path = check_try!(
            archive_entry
                .path()
                .map_err(|error| return format!("read {asset} entry path: {error}"))
        );
        if entry_path.file_name() != Some(OsStr::new("vacuum")) {
            continue;
        }
        let unpacked = check_try!(
            archive_entry
                .unpack(candidate.as_path())
                .map_err(|error| return format!("extract vacuum from {asset}: {error}"))
        );
        match unpacked {
            Unpacked::File(file) => drop(file),
            Unpacked::__Nonexhaustive => {
                return Err(format!("refused to extract vacuum from {asset}"));
            }
        }
        check_try!(install_candidate(candidate.as_path(), paths));
        return Ok(());
    }
    return Err(format!(
        "downloaded {asset} did not contain an executable vacuum binary"
    ));
}

/// Extract Vacuum through a disposable directory and always clean it afterward.
///
/// # Errors
///
/// Returns an error when directory creation, extraction, installation, or cleanup
/// fails.
fn extract_vacuum(
    asset: &str,
    archive_bytes: &[u8],
    install_directory: &Path,
    vacuum_binary: &Path,
) -> CheckResult {
    let temporary_directory = check_try!(create_temporary_directory(install_directory));
    let paths = ExtractionPaths {
        install_directory: install_directory.to_path_buf(),
        temporary_directory: temporary_directory.clone(),
        vacuum_binary: vacuum_binary.to_path_buf(),
    };
    let extraction = extract_archive_binary(asset, archive_bytes, &paths);
    let cleanup = remove_dir_all(temporary_directory.as_path());
    return finish_temporary_extraction(extraction, cleanup, temporary_directory.as_path());
}

/// Combine extraction and mandatory cleanup results.
///
/// # Errors
///
/// Returns the extraction or cleanup error, preserving both when both fail.
fn finish_temporary_extraction(
    extraction: CheckResult,
    cleanup: InputOutputResult<()>,
    temporary_directory: &Path,
) -> CheckResult {
    return match (extraction, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup_error)) => Err(format!(
            "remove temporary directory {}: {cleanup_error}",
            temporary_directory.display()
        )),
        (Err(error), Err(cleanup_error)) => Err(format!(
            "{error}; remove temporary directory {}: {cleanup_error}",
            temporary_directory.display()
        )),
    };
}

/// Encode bytes as lowercase hexadecimal.
///
/// # Errors
///
/// Returns an error when formatting into the output string fails.
fn hex_lower(bytes: &[u8]) -> CheckResult<String> {
    let mut encoded = String::new();
    for byte in bytes {
        check_try!(
            write!(encoded, "{byte:02x}")
                .map_err(|error| return format!("encode SHA-256: {error}"))
        );
    }
    return Ok(encoded);
}

/// Install and verify the pinned Vacuum binary.
///
/// # Errors
///
/// Returns an error when host detection, checksum selection, download,
/// verification, extraction, or version validation fails.
#[inline]
pub fn install(repository: &Path, path: &OsStr, version: &str) -> CheckResult<PathBuf> {
    let install_directory = install_dir(repository, version);
    let vacuum_binary = install_directory.join("vacuum");
    if vacuum_version_matches(vacuum_binary.as_path(), path, version) {
        return Ok(vacuum_binary);
    }
    let host = check_try!(VacuumHost::current());
    let asset = format!(
        "vacuum_{version}_{}_{}.tar.gz",
        host.platform, host.architecture
    );
    let url = format!("https://github.com/daveshanley/vacuum/releases/download/v{version}/{asset}");
    let expected_sha256 = check_try!(vacuum_asset_sha256(version, host));
    let archive_bytes = check_try!(download_asset(url.as_str()));
    check_try!(require_sha256(
        asset.as_str(),
        archive_bytes.as_slice(),
        expected_sha256,
    ));
    check_try!(extract_vacuum(
        asset.as_str(),
        archive_bytes.as_slice(),
        install_directory.as_path(),
        vacuum_binary.as_path(),
    ));
    check_try!(require_version(vacuum_binary.as_path(), path, version));
    return Ok(vacuum_binary);
}

/// Install a successfully extracted Vacuum candidate.
///
/// # Errors
///
/// Returns an error when directory creation, executable permissions, or moving
/// the candidate into place fails.
fn install_candidate(candidate: &Path, paths: &ExtractionPaths) -> CheckResult {
    check_try!(
        create_dir_all(paths.install_directory.as_path()).map_err(|error| {
            return format!("create {}: {error}", paths.install_directory.display());
        })
    );
    check_try!(make_executable(candidate));
    return move_installed_binary(candidate, paths.vacuum_binary.as_path());
}

/// Return the configured Vacuum installation directory.
fn install_dir(repository: &Path, version: &str) -> PathBuf {
    return env::var_os("TOVUK_VACUUM_DIR").map_or_else(
        || {
            return repository
                .join("target")
                .join("tools")
                .join(format!("vacuum-{version}"));
        },
        PathBuf::from,
    );
}

/// Mark an extracted Vacuum binary executable on supported Unix hosts.
///
/// # Errors
///
/// Returns an error when the trusted system `chmod` command cannot run or fails.
fn make_executable(candidate: &Path) -> CheckResult {
    let status = check_try!(
        Command::new("/bin/chmod")
            .arg("755")
            .arg(candidate)
            .status()
            .map_err(|error| return format!("chmod {}: {error}", candidate.display()))
    );
    return status.success().then_some(()).ok_or_else(|| {
        return format!("chmod {} failed with status {status}", candidate.display());
    });
}

/// Move an extracted binary into place, falling back to copy and removal.
///
/// # Errors
///
/// Returns an error when neither rename nor copy-and-remove can install it.
fn move_installed_binary(candidate: &Path, vacuum_binary: &Path) -> CheckResult {
    match rename(candidate, vacuum_binary) {
        Ok(()) => return Ok(()),
        Err(rename_error) => {
            let copied_bytes = check_try!(copy(candidate, vacuum_binary).map_err(|copy_error| {
                return format!(
                    "install {} after rename failed ({rename_error}): {copy_error}",
                    vacuum_binary.display()
                );
            }));
            if copied_bytes == u64::MIN {
                return Err(format!(
                    "installed {} as an empty file",
                    vacuum_binary.display()
                ));
            }
            return remove_file(candidate)
                .map_err(|error| return format!("remove {}: {error}", candidate.display()));
        }
    }
}

/// Verify a downloaded archive SHA-256 digest.
///
/// # Errors
///
/// Returns an error when digest encoding fails or the digest does not match.
fn require_sha256(asset: &str, archive_bytes: &[u8], expected_sha256: &str) -> CheckResult {
    let digest = Sha256::digest(archive_bytes);
    let actual_sha256 = check_try!(hex_lower(digest.as_ref()));
    if actual_sha256 == expected_sha256 {
        return Ok(());
    }
    return Err(format!(
        "downloaded {asset} checksum mismatch: expected {expected_sha256}, got {actual_sha256}"
    ));
}

/// Require the installed Vacuum binary to report the pinned version.
///
/// # Errors
///
/// Returns an error when execution fails, exits unsuccessfully, or reports a
/// different version.
#[inline]
pub fn require_version(vacuum_binary: &Path, path: &OsStr, required_version: &str) -> CheckResult {
    let output = check_try!(
        Command::new(vacuum_binary)
            .arg("version")
            .env("PATH", path)
            .output()
            .map_err(|error| return format!("read Vacuum version: {error}"))
    );
    if !output.status.success() {
        return Err(format!(
            "Vacuum version failed with status {}",
            output.status
        ));
    }
    let installed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if installed != required_version {
        return Err(format!(
            "vacuum {required_version} is required; found {installed}."
        ));
    }
    return Ok(());
}

/// Run Vacuum hard-mode linting across all discovered `OpenAPI` files.
///
/// # Errors
///
/// Returns an error when Vacuum cannot run or exits unsuccessfully.
#[inline]
pub fn run_lint(vacuum_binary: &Path, path: &OsStr, openapi_files: &[String]) -> CheckResult {
    let status = check_try!(
        Command::new(vacuum_binary)
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
            .map_err(|error| return format!("run Vacuum lint: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("Vacuum lint failed with status {status}"));
}

/// Return the pinned checksum for a supported Vacuum release asset.
///
/// # Errors
///
/// Returns an error when no checksum is pinned for the version and host.
fn vacuum_asset_sha256(version: &str, host: VacuumHost) -> CheckResult<&'static str> {
    match (version, host.platform, host.architecture) {
        ("0.26.6", "darwin", "arm64") => {
            return Ok("36e540617b960dc822eec1f65b5e8e6b5a10107c7bca27bf09d8c9afec6fdde2");
        }
        ("0.26.6", "darwin", "x86_64") => {
            return Ok("839c66424af0bfc4357ddea7b46e9c4830923bb7ac95597163df358b7f33425a");
        }
        ("0.26.6", "linux", "arm64") => {
            return Ok("2d57aa941495f970e6093a2b557ce919b02659fc913d13a6a7a8e2deea594b0b");
        }
        ("0.26.6", "linux", "i386") => {
            return Ok("76b90ed6b5bbef1fa1c4adc2d2ccfa8716cfe1df9fd8480573424653f0c42800");
        }
        ("0.26.6", "linux", "x86_64") => {
            return Ok("e81288a3d1f6eb03431b6f8e817b9a8071d2ee800eb0ada3213e4f00805e00e6");
        }
        _ => {
            return Err(format!(
                "unsupported vacuum asset checksum for version={version} platform={} architecture={}",
                host.platform, host.architecture
            ));
        }
    }
}

/// Return whether an installed Vacuum binary matches the requested version.
fn vacuum_version_matches(vacuum_binary: &Path, path: &OsStr, version: &str) -> bool {
    if !vacuum_binary.is_file() {
        return false;
    }
    return Command::new(vacuum_binary)
        .arg("version")
        .env("PATH", path)
        .output()
        .is_ok_and(|output| {
            return output.status.success()
                && String::from_utf8_lossy(&output.stdout).trim() == version;
        });
}

/// Return the configured pinned Vacuum version without a leading `v`.
#[inline]
#[must_use]
pub fn version() -> String {
    return env::var("VACUUM_VERSION")
        .unwrap_or_else(|_| return DEFAULT_VACUUM_VERSION.to_owned())
        .trim_start_matches('v')
        .to_owned();
}
