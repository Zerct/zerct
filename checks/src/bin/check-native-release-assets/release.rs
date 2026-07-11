//! `GitHub` release polling, download, checksum, and cleanup operations.

use alloc::collections::BTreeSet;

use core::time::Duration;

use serde::Deserialize;

use serde_json::from_slice;

use std::{
    ffi::OsString,
    fs::{create_dir_all, remove_dir_all},
    io::{Result as InputOutputResult, Write as _, stdout},
    path::{Path, PathBuf},
    process, thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use tovuk_public_checks::check_support::{CheckResult, command, tool_path};

use super::checksum::verify_asset_checksum;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&create_temporary_directory),
    size_of_val(&download_release_asset),
    size_of_val(&finish_temporary_check),
    size_of_val(&missing_assets),
    size_of_val(&release_asset_names),
    size_of_val(&verify_asset_checksums),
    size_of_val(&verify_downloaded_assets),
    size_of_val(&wait_for_release),
    size_of_val(&write_success),
];

/// One release asset returned by the `GitHub` API.
#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    /// Published asset name.
    name: String,
}

/// Immutable state shared across release polling and downloads.
#[derive(Debug)]
struct ReleaseCheck {
    /// Latest instant at which polling may continue.
    deadline: Instant,
    /// Trusted executable search path.
    path: OsString,
    /// Public repository root.
    repo_root: PathBuf,
    /// Native assets and sidecars required for publication.
    required_assets: Vec<String>,
    /// `GitHub` release tag.
    tag: String,
}

/// `GitHub` release response fields used by this check.
#[derive(Debug, Deserialize)]
struct ReleaseView {
    /// Assets currently attached to the release.
    assets: Vec<ReleaseAsset>,
}

/// Create an isolated native release download directory.
///
/// # Errors
///
/// Returns an error when the directory cannot be created.
fn create_temporary_directory(parent: &Path) -> CheckResult<PathBuf> {
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
    let temporary_path = parent.join(format!(
        "native-release-assets-{}-{timestamp}",
        process::id()
    ));
    check_try!(
        create_dir_all(temporary_path.as_path())
            .map_err(|error| return format!("create {}: {error}", temporary_path.display()))
    );
    return Ok(temporary_path);
}

/// Download one exact release asset into an isolated directory.
///
/// # Errors
///
/// Returns an error when the download directory is not UTF-8, `GitHub` CLI cannot
/// run, or the download exits unsuccessfully.
fn download_release_asset(
    check: &ReleaseCheck,
    asset: &str,
    download_directory: &Path,
) -> CheckResult {
    let directory = check_try!(
        download_directory
            .to_str()
            .ok_or_else(|| return format!("{} must be UTF-8", download_directory.display()))
    );
    let status = check_try!(
        command(check.repo_root.as_path(), check.path.as_os_str(), "gh")
            .args([
                "release",
                "download",
                check.tag.as_str(),
                "--dir",
                directory,
                "--clobber",
                "--pattern",
                asset,
            ])
            .status()
            .map_err(|error| return format!("download {asset}: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("gh release download {asset} failed with status {status}"));
}

/// Combine a verification result with mandatory temporary-directory cleanup.
///
/// # Errors
///
/// Returns the verification or cleanup error, preserving both when both fail.
fn finish_temporary_check(
    verification: CheckResult,
    cleanup: InputOutputResult<()>,
    temporary_path: &Path,
) -> CheckResult {
    return match (verification, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(cleanup_error)) => Err(format!(
            "remove temporary directory {}: {cleanup_error}",
            temporary_path.display()
        )),
        (Err(error), Err(cleanup_error)) => Err(format!(
            "{error}; remove temporary directory {}: {cleanup_error}",
            temporary_path.display()
        )),
    };
}

/// Return required release assets absent from the published release.
fn missing_assets(required_assets: &[String], release_assets: &BTreeSet<String>) -> Vec<String> {
    return required_assets
        .iter()
        .flat_map(|asset| return [asset.clone(), format!("{asset}.sha256")])
        .filter(|asset| return !release_assets.contains(asset))
        .collect();
}

/// Read published asset names for one `GitHub` release.
///
/// # Errors
///
/// Returns an error when `GitHub` CLI cannot run or its JSON response is invalid.
fn release_asset_names(check: &ReleaseCheck) -> CheckResult<BTreeSet<String>> {
    let output = check_try!(
        command(check.repo_root.as_path(), check.path.as_os_str(), "gh")
            .args(["release", "view", check.tag.as_str(), "--json", "assets"])
            .output()
            .map_err(|error| return format!("run gh release view {}: {error}", check.tag))
    );
    if !output.status.success() {
        return Ok(BTreeSet::new());
    }
    let release = check_try!(
        from_slice::<ReleaseView>(&output.stdout)
            .map_err(|error| return format!("parse gh release view {}: {error}", check.tag))
    );
    return Ok(release
        .assets
        .into_iter()
        .map(|asset| return asset.name)
        .collect());
}

/// Verify all native assets in a disposable download directory.
///
/// # Errors
///
/// Returns an error when the directory cannot be created, an asset cannot be
/// downloaded or verified, or cleanup fails.
fn verify_asset_checksums(check: &ReleaseCheck) -> CheckResult {
    let temporary_path = check_try!(create_temporary_directory(
        check.repo_root.join("target").as_path(),
    ));
    let verification = verify_downloaded_assets(check, temporary_path.as_path());
    let cleanup = remove_dir_all(temporary_path.as_path());
    return finish_temporary_check(verification, cleanup, temporary_path.as_path());
}

/// Download and verify every required native asset.
///
/// # Errors
///
/// Returns an error when an asset or checksum cannot be downloaded or verified.
fn verify_downloaded_assets(check: &ReleaseCheck, temporary_path: &Path) -> CheckResult {
    for asset in &check.required_assets {
        check_try!(download_release_asset(check, asset, temporary_path));
        let checksum_asset = format!("{asset}.sha256");
        check_try!(download_release_asset(
            check,
            checksum_asset.as_str(),
            temporary_path,
        ));
        check_try!(verify_asset_checksum(
            temporary_path.join(asset).as_path(),
            temporary_path.join(checksum_asset).as_path(),
            asset,
        ));
    }
    return Ok(());
}

/// Poll until every required asset exists, then verify all checksums.
///
/// # Errors
///
/// Returns an error when `GitHub` queries or verification fail, or the deadline
/// expires while assets are missing.
pub(super) fn wait_for_release(
    repo_root: PathBuf,
    required_assets: Vec<String>,
    tag: String,
    wait_duration: Duration,
) -> CheckResult {
    let deadline = check_try!(
        Instant::now()
            .checked_add(wait_duration)
            .ok_or_else(|| return "release wait deadline overflow".to_owned())
    );
    let check = ReleaseCheck {
        deadline,
        path: tool_path(),
        repo_root,
        required_assets,
        tag,
    };
    loop {
        let release_assets = check_try!(release_asset_names(&check));
        let missing = missing_assets(&check.required_assets, &release_assets);
        if missing.is_empty() {
            check_try!(verify_asset_checksums(&check));
            check_try!(write_success(check.tag.as_str()));
            return Ok(());
        }
        if Instant::now() >= check.deadline {
            return Err(format!(
                "Missing native Tovuk release assets for {}:\n{}",
                check.tag,
                missing.join("\n")
            ));
        }
        thread::sleep(Duration::from_secs(0x14));
    }
}

/// Write the successful release verification message.
///
/// # Errors
///
/// Returns an error when standard output cannot be written.
fn write_success(tag: &str) -> CheckResult {
    return writeln!(
        stdout().lock(),
        "All native Tovuk release assets exist and match checksums for {tag}."
    )
    .map_err(|error| return format!("write release verification: {error}"));
}
