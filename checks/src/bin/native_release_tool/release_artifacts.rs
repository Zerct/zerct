//! Validate native build artifacts and prepare deterministic checksum sidecars.

extern crate alloc;

use alloc::collections::BTreeSet;

use std::{
    fs::{DirEntry, metadata, read_dir},
    io::{Result as InputOutputResult, Write as _, stdout},
    path::Path,
};

use super::{checksum::write_sha256, matrix_entry, read_matrix, read_package_version};

/// Result returned by native artifact preparation operations.
pub(crate) type ArtifactResult<Value = ()> = Result<Value, String>;

/// Exact set of native binary asset names.
pub(crate) type AssetNames = BTreeSet<String>;

/// Native artifact operations used by the release workflow boundary.
pub(super) trait ReleaseArtifactOperations {
    /// Derive the exact binary asset names for a target manifest and crate version.
    ///
    /// # Errors
    ///
    /// Returns an error when either manifest is invalid or asset names collide.
    fn expected_asset_names(
        &self,
        manifest_path: &Path,
        crate_manifest_path: &Path,
    ) -> ArtifactResult<AssetNames>;

    /// Validate one artifact and write its checksum sidecar.
    ///
    /// # Errors
    ///
    /// Returns an error when the artifact is empty or its checksum cannot be written.
    fn prepare_asset(&self, directory: &Path, asset_name: &str) -> ArtifactResult;

    /// Validate the downloaded matrix artifacts and write their checksum sidecars.
    ///
    /// # Errors
    ///
    /// Returns an error when artifact contents differ from the tracked target set
    /// or a checksum cannot be written atomically.
    fn prepare_release(
        &self,
        artifact_directory: &Path,
        manifest_path: &Path,
        crate_manifest_path: &Path,
    ) -> ArtifactResult;

    /// Validate one directory entry and return its UTF-8 regular-file name.
    ///
    /// # Errors
    ///
    /// Returns an error when the entry cannot be read or is not a regular file.
    fn tracked_asset_name(
        &self,
        directory: &Path,
        entry_result: InputOutputResult<DirEntry>,
    ) -> ArtifactResult<String>;

    /// Read exact regular-file names from an artifact directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory cannot be read or contains an unsafe entry.
    fn tracked_asset_names(&self, directory: &Path) -> ArtifactResult<AssetNames>;

    /// Print the exact binary asset names, one per line.
    ///
    /// # Errors
    ///
    /// Returns an error when manifests are invalid or standard output cannot be written.
    fn write_asset_names(&self, manifest_path: &Path, crate_manifest_path: &Path)
    -> ArtifactResult;

    /// Print the release tag derived from a crate manifest.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest is invalid or standard output cannot be written.
    fn write_tag(&self, crate_manifest_path: &Path) -> ArtifactResult;
}

/// Stateless implementation of native artifact preparation.
pub(super) struct ReleaseArtifacts;

impl ReleaseArtifactOperations for ReleaseArtifacts {
    fn expected_asset_names(
        &self,
        manifest_path: &Path,
        crate_manifest_path: &Path,
    ) -> ArtifactResult<AssetNames> {
        let manifest = check_try!(read_matrix(manifest_path));
        let version = check_try!(read_package_version(crate_manifest_path));
        if manifest.targets.is_empty() {
            return Err("native release target manifest must not be empty".to_owned());
        }
        let target_count = manifest.targets.len();
        let names = manifest
            .targets
            .into_iter()
            .map(|target| return matrix_entry(target, version.as_str()).asset_name)
            .collect::<AssetNames>();
        if names.len() != target_count {
            return Err("native release target manifest produces duplicate asset names".to_owned());
        }
        return Ok(names);
    }

    fn prepare_asset(&self, directory: &Path, asset_name: &str) -> ArtifactResult {
        let asset_path = directory.join(asset_name);
        let asset_metadata = check_try!(metadata(asset_path.as_path()).map_err(|error| {
            return format!("read metadata for {}: {error}", asset_path.display());
        }));
        if asset_metadata.len() == 0x0 {
            return Err(format!(
                "native asset {} must not be empty",
                asset_path.display()
            ));
        }
        drop(check_try!(write_sha256(asset_path.as_path())));
        return Ok(());
    }

    fn prepare_release(
        &self,
        artifact_directory: &Path,
        manifest_path: &Path,
        crate_manifest_path: &Path,
    ) -> ArtifactResult {
        let expected = check_try!(self.expected_asset_names(manifest_path, crate_manifest_path));
        let actual = check_try!(self.tracked_asset_names(artifact_directory));
        if actual != expected {
            return Err(format!(
                "native artifact names must be exactly {expected:?}, got {actual:?}"
            ));
        }
        return expected
            .iter()
            .try_for_each(|asset_name| return self.prepare_asset(artifact_directory, asset_name));
    }

    fn tracked_asset_name(
        &self,
        directory: &Path,
        entry_result: InputOutputResult<DirEntry>,
    ) -> ArtifactResult<String> {
        let entry = check_try!(
            entry_result
                .map_err(|error| return format!("read {} entry: {error}", directory.display()))
        );
        let file_type = check_try!(entry.file_type().map_err(|error| {
            return format!("read file type for {}: {error}", entry.path().display());
        }));
        if file_type.is_symlink() || !entry.path().is_file() {
            return Err(format!(
                "native artifact {} must be a regular non-symlink file",
                entry.path().display()
            ));
        }
        return entry.file_name().into_string().map_err(|name| {
            return format!(
                "native artifact name must be UTF-8: {}",
                name.to_string_lossy()
            );
        });
    }

    fn tracked_asset_names(&self, directory: &Path) -> ArtifactResult<AssetNames> {
        let entries = check_try!(
            read_dir(directory)
                .map_err(|error| return format!("read {}: {error}", directory.display()))
        );
        return entries
            .map(|entry_result| return self.tracked_asset_name(directory, entry_result))
            .collect::<ArtifactResult<AssetNames>>();
    }

    fn write_asset_names(
        &self,
        manifest_path: &Path,
        crate_manifest_path: &Path,
    ) -> ArtifactResult {
        let asset_names = check_try!(self.expected_asset_names(manifest_path, crate_manifest_path));
        for asset_name in asset_names {
            check_try!(
                writeln!(stdout().lock(), "{asset_name}")
                    .map_err(|error| return format!("write native asset name: {error}"))
            );
        }
        return Ok(());
    }

    fn write_tag(&self, crate_manifest_path: &Path) -> ArtifactResult {
        let version = check_try!(read_package_version(crate_manifest_path));
        return writeln!(stdout().lock(), "v{version}")
            .map_err(|error| return format!("write release tag: {error}"));
    }
}
