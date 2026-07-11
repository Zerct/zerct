//! Locked Cargo graph loading and normalization.

use alloc::collections::BTreeMap;

use core::fmt::Write as _;

use serde_json::from_slice;

use sha2::{Digest as _, Sha256};

use std::{
    ffi::OsStr,
    path::Path,
    process::{Command, Output},
};

use tovuk_public_checks::{
    check_support::{CheckResult, command},
    check_try,
};

use super::{FeatureUnion, MetadataSnapshot, PackageIdentities, TARGETS, TargetSnapshot};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&canonical_graph),
    size_of_val(&cargo_metadata),
    size_of_val(&feature_union),
    size_of_val(&fingerprint),
    size_of_val(&load_snapshots),
    size_of_val(&package_identities),
];

/// Normalize target-specific Cargo metadata into stable graph records.
///
/// # Errors
///
/// Returns an error when the metadata lacks a resolve graph or package entry.
fn canonical_graph(metadata: &MetadataSnapshot) -> CheckResult<String> {
    let identities = package_identities(metadata);
    let resolve = check_try!(
        metadata
            .resolve
            .as_ref()
            .ok_or_else(|| return "cargo metadata did not return a resolve graph".to_owned())
    );
    let nodes = resolve.nodes.as_slice();
    let mut records = Vec::new();
    for node in nodes {
        let identity = check_try!(
            identities
                .get(node.identifier.as_str())
                .ok_or_else(|| format!("cargo metadata package {} is missing", node.identifier))
        );
        records.push(format!("package\t{identity}"));
        records.extend(
            node.features
                .iter()
                .map(|feature| return format!("feature\t{identity}\t{feature}")),
        );
        let dependency_records = node
            .dependencies
            .iter()
            .map(|dependency_id| {
                let dependency = check_try!(identities.get(dependency_id.as_str()).ok_or_else(
                    || return format!("cargo metadata dependency {dependency_id} is missing"),
                ));
                return Ok(format!("dependency\t{identity}\t{dependency}"));
            })
            .collect::<CheckResult<Vec<_>>>();
        records.extend(check_try!(dependency_records));
    }
    records.sort();
    return Ok(records.join("\n"));
}

/// Run Cargo metadata for one manifest and optional target.
///
/// # Errors
///
/// Returns an error when Cargo fails.
pub(super) fn cargo_metadata(
    repository: &Path,
    path: &OsStr,
    manifest: &str,
    target: Option<&str>,
) -> CheckResult<Output> {
    let mut prepared = command(repository, path, "cargo");
    let _: &mut Command = prepared.args([
        "metadata",
        "--locked",
        "--manifest-path",
        manifest,
        "--all-features",
        "--format-version",
        "1",
    ]);
    if let Some(triple) = target {
        let _: &mut Command = prepared.args(["--filter-platform", triple]);
    }
    let output = check_try!(
        prepared
            .output()
            .map_err(|error| return format!("run cargo metadata for {manifest}: {error}"))
    );
    if output.status.success() {
        return Ok(output);
    }
    return Err(format!(
        "cargo metadata for {manifest} failed with status {}:\n{}",
        output.status,
        String::from_utf8_lossy(output.stderr.as_slice())
    ));
}

/// Derive reviewed external feature unions across all target snapshots.
///
/// # Errors
///
/// Returns an error when a resolved package is absent from Cargo's package list.
pub(super) fn feature_union(snapshots: &[TargetSnapshot]) -> CheckResult<FeatureUnion> {
    let mut union = FeatureUnion::new();
    let merge_result = snapshots.iter().try_for_each(|snapshot| -> CheckResult {
        let packages = snapshot
            .metadata
            .packages
            .iter()
            .map(|package| return (package.identifier.as_str(), package))
            .collect::<BTreeMap<_, _>>();
        let nodes = check_try!(
            snapshot
                .metadata
                .resolve
                .as_ref()
                .map(|resolve| return resolve.nodes.as_slice())
                .ok_or_else(|| return "cargo metadata did not return a resolve graph".to_owned())
        );
        nodes
            .iter()
            .filter(|node| return !node.features.is_empty())
            .filter_map(|node| {
                return packages
                    .get(node.identifier.as_str())
                    .map(|package| return (node, *package));
            })
            .filter(|node_package| return node_package.1.source.is_some())
            .for_each(|node_package| {
                let node = node_package.0;
                let package = node_package.1;
                let crate_spec = format!("{}@{}", package.name, package.version);
                union
                    .entry(crate_spec)
                    .or_default()
                    .extend(node.features.iter().cloned());
            });
        return Ok(());
    });
    check_try!(merge_result);
    return Ok(union);
}

/// Hash a canonical target-specific graph.
///
/// # Errors
///
/// Returns an error when digest formatting fails.
fn fingerprint(canonical: &str) -> CheckResult<String> {
    let digest = Sha256::digest(canonical.as_bytes());
    let mut hexadecimal = String::with_capacity(0x0040);
    for byte in digest {
        check_try!(
            write!(hexadecimal, "{byte:02x}")
                .map_err(|error| return format!("encode dependency fingerprint: {error}"))
        );
    }
    return Ok(hexadecimal);
}

/// Load all locked target snapshots for one manifest.
///
/// # Errors
///
/// Returns an error when Cargo metadata is invalid or cannot be normalized.
pub(super) fn load_snapshots(
    repository: &Path,
    path: &OsStr,
    manifest: &str,
) -> CheckResult<Vec<TargetSnapshot>> {
    let mut snapshots = Vec::new();
    for triple in TARGETS {
        let output = check_try!(cargo_metadata(repository, path, manifest, Some(triple)));
        let metadata = check_try!(
            from_slice::<MetadataSnapshot>(output.stdout.as_slice()).map_err(|error| {
                return format!("parse cargo metadata for {manifest} {triple}: {error}");
            })
        );
        let canonical = check_try!(canonical_graph(&metadata));
        snapshots.push(TargetSnapshot {
            fingerprint: check_try!(fingerprint(canonical.as_str())),
            metadata,
            triple: (*triple).to_owned(),
        });
    }
    return Ok(snapshots);
}

/// Build stable package identities without filesystem-specific manifest paths.
fn package_identities(metadata: &MetadataSnapshot) -> PackageIdentities {
    return metadata
        .packages
        .iter()
        .map(|package| {
            let source = package.source.as_deref().unwrap_or("workspace");
            let identity = format!("{}@{}|{source}", package.name, package.version);
            return (package.identifier.clone(), identity);
        })
        .collect();
}
