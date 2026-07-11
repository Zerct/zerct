//! `GitHub` release polling, download, checksum, and cleanup operations.

use alloc::collections::BTreeSet;

use core::time::Duration;

use serde_json::{Value, from_slice};

use std::{
    ffi::OsString,
    fs::{create_dir_all, remove_dir_all},
    io::{Result as InputOutputResult, Write as _, stdout},
    path::{Path, PathBuf},
    process, thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use tovuk_public_checks::check_support::{CheckResult, command, tool_path};

use super::{DraftPolicy, ReleaseVerification, checksum::verify_asset_checksum};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0010] = [
    size_of_val(&create_temporary_directory),
    size_of_val(&download_release_asset),
    size_of_val(&expected_asset_names),
    size_of_val(&finish_temporary_check),
    size_of_val(&missing_assets),
    size_of_val(&parse_release_assets),
    size_of_val(&parse_release_view),
    size_of_val(&release_asset_names),
    size_of_val(&release_flag),
    size_of_val(&release_view),
    size_of_val(&unexpected_assets),
    size_of_val(&validate_release_state),
    size_of_val(&verify_asset_checksums),
    size_of_val(&verify_downloaded_assets),
    size_of_val(&wait_for_release),
    size_of_val(&write_success),
];

/// One release asset returned by the `GitHub` API.
#[derive(Debug)]
struct ReleaseAsset {
    /// Published asset name.
    name: String,
}

/// Immutable state shared across release polling and downloads.
#[derive(Debug)]
struct ReleaseCheck {
    /// Latest instant at which polling may continue.
    deadline: Instant,
    /// Visibility accepted by the remote release check.
    draft_policy: DraftPolicy,
    /// Trusted executable search path.
    path: OsString,
    /// Public repository root.
    repo_root: PathBuf,
    /// Native assets and sidecars required for publication.
    required_assets: Vec<String>,
    /// `GitHub` release tag.
    tag: String,
}

/// Boolean release state represented without Boolean-bearing policy structs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReleaseFlag {
    /// The queried release flag is false.
    Disabled,
    /// The queried release flag is true.
    Enabled,
}

/// `GitHub` release response fields used by this check.
#[derive(Debug)]
struct ReleaseView {
    /// Assets currently attached to the release.
    assets: Vec<ReleaseAsset>,
    /// Whether the release is still private as a draft.
    is_draft: ReleaseFlag,
    /// Whether the release is marked as a prerelease.
    is_prerelease: ReleaseFlag,
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

/// Return the exact native asset and checksum-sidecar set required for release.
fn expected_asset_names(required_assets: &[String]) -> BTreeSet<String> {
    return required_assets
        .iter()
        .flat_map(|asset| return [asset.clone(), format!("{asset}.sha256")])
        .collect();
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
    return expected_asset_names(required_assets)
        .into_iter()
        .filter(|asset| return !release_assets.contains(asset))
        .collect();
}

/// Parse the exact release asset name list from a `GitHub` JSON response.
///
/// # Errors
///
/// Returns an error when the assets field or an asset name is absent or invalid.
fn parse_release_assets(document: &Value, tag: &str) -> CheckResult<Vec<ReleaseAsset>> {
    let values = check_try!(
        document
            .get("assets")
            .and_then(Value::as_array)
            .ok_or_else(|| return format!("gh release view {tag} assets must be an array"))
    );
    let mut assets = Vec::with_capacity(values.len());
    for value in values {
        let name = check_try!(
            value
                .get("name")
                .and_then(Value::as_str)
                .filter(|name| return !name.is_empty())
                .ok_or_else(|| return format!("gh release view {tag} asset name is invalid"))
        );
        assets.push(ReleaseAsset {
            name: name.to_owned(),
        });
    }
    return Ok(assets);
}

/// Parse the bounded release state used by native asset verification.
///
/// # Errors
///
/// Returns an error when the response is malformed or lacks required fields.
fn parse_release_view(source: &[u8], tag: &str) -> CheckResult<ReleaseView> {
    let document = check_try!(
        from_slice::<Value>(source)
            .map_err(|error| return format!("parse gh release view {tag}: {error}"))
    );
    return Ok(ReleaseView {
        assets: check_try!(parse_release_assets(&document, tag)),
        is_draft: check_try!(release_flag(&document, "isDraft", tag)),
        is_prerelease: check_try!(release_flag(&document, "isPrerelease", tag)),
    });
}

/// Convert release assets into an exact ordered name set.
fn release_asset_names(release: &ReleaseView) -> BTreeSet<String> {
    return release
        .assets
        .iter()
        .map(|asset| return asset.name.clone())
        .collect();
}

/// Parse one required `GitHub` Boolean release field into an explicit flag.
///
/// # Errors
///
/// Returns an error when the field is absent or not Boolean.
fn release_flag(document: &Value, field: &str, tag: &str) -> CheckResult<ReleaseFlag> {
    return match document.get(field).and_then(Value::as_bool) {
        Some(false) => Ok(ReleaseFlag::Disabled),
        Some(true) => Ok(ReleaseFlag::Enabled),
        None => Err(format!(
            "gh release view {tag} field {field} must be Boolean"
        )),
    };
}

/// Read release state and published asset names for one `GitHub` release.
///
/// # Errors
///
/// Returns an error when `GitHub` CLI cannot run or its JSON response is invalid.
fn release_view(check: &ReleaseCheck) -> CheckResult<ReleaseView> {
    let output = check_try!(
        command(check.repo_root.as_path(), check.path.as_os_str(), "gh")
            .args([
                "release",
                "view",
                check.tag.as_str(),
                "--json",
                "assets,isDraft,isPrerelease",
            ])
            .output()
            .map_err(|error| return format!("run gh release view {}: {error}", check.tag))
    );
    if !output.status.success() {
        return Err(format!(
            "gh release view {} failed with status {}",
            check.tag, output.status
        ));
    }
    return parse_release_view(&output.stdout, check.tag.as_str());
}

/// Return release assets outside the exact public native asset contract.
fn unexpected_assets(required_assets: &[String], release_assets: &BTreeSet<String>) -> Vec<String> {
    let expected = expected_asset_names(required_assets);
    return release_assets
        .iter()
        .filter(|asset| return !expected.contains(*asset))
        .cloned()
        .collect();
}

/// Require a non-prerelease and reject drafts outside prepublication verification.
///
/// # Errors
///
/// Returns an error for prereleases or a draft outside the prepublication gate.
fn validate_release_state(check: &ReleaseCheck, release: &ReleaseView) -> CheckResult {
    if release.is_prerelease == ReleaseFlag::Enabled {
        return Err(format!("{} must not be a prerelease", check.tag));
    }
    if release.is_draft == ReleaseFlag::Disabled || matches!(check.draft_policy, DraftPolicy::Allow)
    {
        return Ok(());
    }
    return Err(format!("{} must be a published release", check.tag));
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
pub(super) fn wait_for_release(verification: ReleaseVerification) -> CheckResult {
    let deadline = check_try!(
        Instant::now()
            .checked_add(verification.wait_duration)
            .ok_or_else(|| return "release wait deadline overflow".to_owned())
    );
    let check = ReleaseCheck {
        deadline,
        draft_policy: verification.draft_policy,
        path: tool_path(),
        repo_root: verification.repo_root,
        required_assets: verification.required_assets,
        tag: verification.tag,
    };
    loop {
        let release = check_try!(release_view(&check));
        let release_assets = release_asset_names(&release);
        let unexpected = unexpected_assets(&check.required_assets, &release_assets);
        if !unexpected.is_empty() {
            return Err(format!(
                "Unexpected native Tovuk release assets for {}:\n{}",
                check.tag,
                unexpected.join("\n")
            ));
        }
        let missing = missing_assets(&check.required_assets, &release_assets);
        if missing.is_empty() {
            check_try!(validate_release_state(&check, &release));
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
