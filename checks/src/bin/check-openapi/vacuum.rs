//! Vacuum binary installation and lint execution.

use std::{
    env,
    ffi::OsStr,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use tovuk_public_checks::check_support::CheckResult;

const DEFAULT_VACUUM_VERSION: &str = "0.26.6";

pub(crate) fn version() -> String {
    env::var("VACUUM_VERSION")
        .unwrap_or_else(|_| DEFAULT_VACUUM_VERSION.to_owned())
        .trim_start_matches('v')
        .to_owned()
}

pub(crate) fn install(repo_root: &Path, path: &OsStr, version: &str) -> CheckResult<PathBuf> {
    let install_dir = install_dir(repo_root, version);
    let vacuum_bin = install_dir.join("vacuum");
    if vacuum_version_matches(vacuum_bin.as_path(), path, version) {
        return Ok(vacuum_bin);
    }

    let host = VacuumHost::current()?;
    let asset = format!("vacuum_{version}_{}_{}.tar.gz", host.os, host.arch);
    let url = format!("https://github.com/daveshanley/vacuum/releases/download/v{version}/{asset}");
    let expected_sha256 = vacuum_asset_sha256(version, &host)?;
    let archive_bytes = download_asset(url.as_str())?;
    require_sha256(asset.as_str(), archive_bytes.as_slice(), expected_sha256)?;
    extract_vacuum(
        asset.as_str(),
        archive_bytes.as_slice(),
        install_dir.as_path(),
        vacuum_bin.as_path(),
    )?;
    require_version(vacuum_bin.as_path(), path, version)?;
    Ok(vacuum_bin)
}

pub(crate) fn require_version(
    vacuum_bin: &Path,
    path: &OsStr,
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

pub(crate) fn run_lint(vacuum_bin: &Path, path: &OsStr, openapi_files: &[String]) -> CheckResult {
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

struct VacuumHost {
    os: &'static str,
    arch: &'static str,
}

impl VacuumHost {
    fn current() -> CheckResult<Self> {
        let os = match env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            other => return Err(format!("unsupported vacuum host OS: {other}")),
        };
        let arch = match env::consts::ARCH {
            "aarch64" => "arm64",
            "x86" => "i386",
            "x86_64" => "x86_64",
            other => return Err(format!("unsupported vacuum host architecture: {other}")),
        };
        Ok(Self { os, arch })
    }
}

fn install_dir(repo_root: &Path, version: &str) -> PathBuf {
    env::var_os("TOVUK_VACUUM_DIR").map_or_else(
        || {
            repo_root
                .join("target")
                .join("tools")
                .join(format!("vacuum-{version}"))
        },
        PathBuf::from,
    )
}

fn vacuum_version_matches(vacuum_bin: &Path, path: &OsStr, version: &str) -> bool {
    if !vacuum_bin.is_file() {
        return false;
    }
    Command::new(vacuum_bin)
        .arg("version")
        .env("PATH", path)
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == version
        })
}

fn vacuum_asset_sha256(version: &str, host: &VacuumHost) -> CheckResult<&'static str> {
    match (version, host.os, host.arch) {
        ("0.26.6", "darwin", "arm64") => {
            Ok("36e540617b960dc822eec1f65b5e8e6b5a10107c7bca27bf09d8c9afec6fdde2")
        }
        ("0.26.6", "darwin", "x86_64") => {
            Ok("839c66424af0bfc4357ddea7b46e9c4830923bb7ac95597163df358b7f33425a")
        }
        ("0.26.6", "linux", "arm64") => {
            Ok("2d57aa941495f970e6093a2b557ce919b02659fc913d13a6a7a8e2deea594b0b")
        }
        ("0.26.6", "linux", "x86_64") => {
            Ok("e81288a3d1f6eb03431b6f8e817b9a8071d2ee800eb0ada3213e4f00805e00e6")
        }
        ("0.26.6", "linux", "i386") => {
            Ok("76b90ed6b5bbef1fa1c4adc2d2ccfa8716cfe1df9fd8480573424653f0c42800")
        }
        _ => Err(format!(
            "unsupported vacuum asset checksum for version={version} os={} arch={}",
            host.os, host.arch
        )),
    }
}

fn download_asset(url: &str) -> CheckResult<Vec<u8>> {
    let response =
        reqwest::blocking::get(url).map_err(|error| format!("download {url}: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("download {url} failed with status {status}"));
    }
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("read {url} response body: {error}"))
}

fn require_sha256(asset: &str, archive_bytes: &[u8], expected_sha256: &str) -> CheckResult {
    let digest = Sha256::digest(archive_bytes);
    let actual_sha256 = hex_lower(digest.as_ref());
    if actual_sha256 == expected_sha256 {
        Ok(())
    } else {
        Err(format!(
            "downloaded {asset} checksum mismatch: expected {expected_sha256}, got {actual_sha256}"
        ))
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn extract_vacuum(
    asset: &str,
    archive_bytes: &[u8],
    install_dir: &Path,
    vacuum_bin: &Path,
) -> CheckResult {
    let temp_dir = TempDir::new(install_dir)?;
    let candidate = temp_dir.path().join("vacuum");
    let mut archive = Archive::new(GzDecoder::new(archive_bytes));
    for entry in archive
        .entries()
        .map_err(|error| format!("read {asset}: {error}"))?
    {
        let mut entry = entry.map_err(|error| format!("read {asset} entry: {error}"))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|error| format!("read {asset} entry path: {error}"))?;
        if path.file_name() != Some(OsStr::new("vacuum")) {
            continue;
        }
        entry
            .unpack(candidate.as_path())
            .map_err(|error| format!("extract vacuum from {asset}: {error}"))?;
        fs::create_dir_all(install_dir)
            .map_err(|error| format!("create {}: {error}", install_dir.display()))?;
        fs::set_permissions(candidate.as_path(), fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("chmod {}: {error}", candidate.display()))?;
        move_installed_binary(candidate.as_path(), vacuum_bin)?;
        return Ok(());
    }
    Err(format!(
        "downloaded {asset} did not contain an executable vacuum binary"
    ))
}

fn move_installed_binary(candidate: &Path, vacuum_bin: &Path) -> CheckResult {
    match fs::rename(candidate, vacuum_bin) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            fs::copy(candidate, vacuum_bin).map_err(|copy_error| {
                format!(
                    "install {} after rename failed ({rename_error}): {copy_error}",
                    vacuum_bin.display()
                )
            })?;
            fs::remove_file(candidate)
                .map_err(|error| format!("remove {}: {error}", candidate.display()))
        }
    }
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(install_dir: &Path) -> CheckResult<Self> {
        let parent = install_dir
            .parent()
            .ok_or_else(|| format!("{} must have a parent directory", install_dir.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system time before Unix epoch: {error}"))?
            .as_nanos();
        let path = parent.join(format!("vacuum-download-{}-{timestamp}", process::id()));
        fs::create_dir(path.as_path())
            .map_err(|error| format!("create {}: {error}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.path.as_path());
    }
}
