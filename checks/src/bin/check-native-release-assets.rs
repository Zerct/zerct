//! Verify native release assets and checksums before wrapper publishes.

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::{self, ExitCode},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
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
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 2 {
        return Err("usage: check-native-release-assets [version] [wait_seconds]".to_owned());
    }
    let version = match args.first().map(String::as_str) {
        None | Some("") => read_crate_version(repo_root.as_path())?,
        Some(value) => value.to_owned(),
    };
    let wait_seconds = args
        .get(1)
        .map_or(Ok(0), |value| parse_wait_seconds(value.as_str()))?;
    let required_assets = required_assets(repo_root.as_path(), version.as_str())?;
    let tag = format!("v{version}");
    let deadline = Instant::now() + Duration::from_secs(wait_seconds);

    loop {
        let release_assets =
            release_asset_names(repo_root.as_path(), path.as_os_str(), tag.as_str())?;
        let missing = missing_assets(&required_assets, &release_assets);
        if missing.is_empty() {
            verify_asset_checksums(
                repo_root.as_path(),
                path.as_os_str(),
                tag.as_str(),
                &required_assets,
            )?;
            println!("All native Tovuk release assets exist and match checksums for {tag}.");
            return Ok(());
        }

        if Instant::now() >= deadline {
            return Err(format!(
                "Missing native Tovuk release assets for {tag}:\n{}",
                missing.join("\n")
            ));
        }

        thread::sleep(Duration::from_secs(20));
    }
}

#[derive(Deserialize)]
struct NativeReleaseTargets {
    targets: Vec<NativeTarget>,
}

#[derive(Deserialize)]
struct NativeTarget {
    asset_ext: String,
    triple: String,
}

fn read_crate_version(repo_root: &Path) -> CheckResult<String> {
    let manifest_path = repo_root.join("crates").join("tovuk").join("Cargo.toml");
    let source = fs::read_to_string(manifest_path.as_path())
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    for line in source.lines() {
        let trimmed = line.trim();
        let Some(raw_version) = trimmed.strip_prefix("version = ") else {
            continue;
        };
        let version = raw_version.trim().trim_matches('"');
        if !version.is_empty() {
            return Ok(version.to_owned());
        }
    }
    Err(format!(
        "{} must contain a version",
        manifest_path.display()
    ))
}

fn parse_wait_seconds(value: &str) -> CheckResult<u64> {
    value
        .parse::<u64>()
        .map_err(|error| format!("wait_seconds must be an unsigned integer: {error}"))
}

fn required_assets(repo_root: &Path, version: &str) -> CheckResult<Vec<String>> {
    let manifest_path = repo_root.join("native-release-targets.json");
    let source = fs::read_to_string(manifest_path.as_path())
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let manifest = serde_json::from_str::<NativeReleaseTargets>(source.as_str())
        .map_err(|error| format!("parse {}: {error}", manifest_path.display()))?;
    let mut assets = manifest
        .targets
        .into_iter()
        .map(|target| format!("tovuk-{version}-{}{}", target.triple, target.asset_ext))
        .collect::<Vec<_>>();
    assets.sort();
    Ok(assets)
}

#[derive(Deserialize)]
struct ReleaseView {
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
}

fn release_asset_names(
    repo_root: &Path,
    path: &std::ffi::OsStr,
    tag: &str,
) -> CheckResult<BTreeSet<String>> {
    let output = command(repo_root, path, "gh")
        .args(["release", "view", tag, "--json", "assets"])
        .output()
        .map_err(|error| format!("run gh release view {tag}: {error}"))?;
    if !output.status.success() {
        return Ok(BTreeSet::new());
    }
    let release = serde_json::from_slice::<ReleaseView>(&output.stdout)
        .map_err(|error| format!("parse gh release view {tag}: {error}"))?;
    Ok(release.assets.into_iter().map(|asset| asset.name).collect())
}

fn missing_assets(required_assets: &[String], release_assets: &BTreeSet<String>) -> Vec<String> {
    required_assets
        .iter()
        .flat_map(|asset| [asset.clone(), format!("{asset}.sha256")])
        .filter(|asset| !release_assets.contains(asset))
        .collect()
}

fn verify_asset_checksums(
    repo_root: &Path,
    path: &std::ffi::OsStr,
    tag: &str,
    required_assets: &[String],
) -> CheckResult {
    let temp_dir = TempDir::new(repo_root.join("target").as_path())?;
    for asset in required_assets {
        download_release_asset(repo_root, path, tag, asset, temp_dir.path())?;
        let checksum_asset = format!("{asset}.sha256");
        download_release_asset(
            repo_root,
            path,
            tag,
            checksum_asset.as_str(),
            temp_dir.path(),
        )?;
        verify_asset_checksum(
            temp_dir.path().join(asset).as_path(),
            temp_dir.path().join(checksum_asset).as_path(),
            asset,
        )?;
    }
    Ok(())
}

fn download_release_asset(
    repo_root: &Path,
    path: &std::ffi::OsStr,
    tag: &str,
    asset: &str,
    download_dir: &Path,
) -> CheckResult {
    let status = command(repo_root, path, "gh")
        .args([
            "release",
            "download",
            tag,
            "--dir",
            download_dir
                .to_str()
                .ok_or_else(|| format!("{} must be UTF-8", download_dir.display()))?,
            "--clobber",
            "--pattern",
            asset,
        ])
        .status()
        .map_err(|error| format!("download {asset}: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("gh release download {asset} failed with status {status}"))
}

fn verify_asset_checksum(asset_path: &Path, checksum_path: &Path, asset_name: &str) -> CheckResult {
    let checksum_source = fs::read_to_string(checksum_path)
        .map_err(|error| format!("read {}: {error}", checksum_path.display()))?;
    let line = checksum_source
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| format!("{asset_name}.sha256 is empty"))?;
    let parts = line.split_whitespace().collect::<Vec<_>>();
    let Some(digest) = parts.first().map(|part| part.to_ascii_lowercase()) else {
        return Err(format!("{asset_name}.sha256 is empty"));
    };
    if digest.len() != 64
        || !digest
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(format!(
            "{asset_name}.sha256 does not contain a SHA-256 digest"
        ));
    }
    if parts.len() > 1 {
        let listed_asset = Path::new(parts[1..].join(" ").trim_start_matches('*'))
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        if listed_asset != asset_name {
            return Err(format!(
                "{asset_name}.sha256 names {listed_asset}, expected {asset_name}"
            ));
        }
    }

    let asset_bytes =
        fs::read(asset_path).map_err(|error| format!("read {}: {error}", asset_path.display()))?;
    let actual = sha256_hex(asset_bytes.as_slice());
    if actual == digest {
        Ok(())
    } else {
        Err(format!(
            "{asset_name} checksum mismatch: expected {digest}, got {actual}"
        ))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let digest_bytes: &[u8] = digest.as_ref();
    let mut encoded = String::with_capacity(digest_bytes.len() * 2);
    for byte in digest_bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(parent: &Path) -> CheckResult<Self> {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system time before Unix epoch: {error}"))?
            .as_nanos();
        let path = parent.join(format!(
            "native-release-assets-{}-{timestamp}",
            process::id()
        ));
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
