//! Locked Cargo graph loading and normalization.

use alloc::collections::{BTreeMap, BTreeSet};

use core::{fmt::Write as _, iter::once};

use sha2::{Digest as _, Sha256};

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tovuk_public_checks::{
    check_support::{CheckResult, command},
    check_try,
};

use super::{
    FeatureUnion, LoadedSnapshots, MetadataDependency, MetadataPackage, MetadataSnapshot,
    PackageIdentities, ResolveNode, TARGETS, TargetSnapshot, active,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0012] = [
    size_of_val(&canonical_graph),
    size_of_val(&cargo_metadata),
    size_of_val(&cross_package_feature),
    size_of_val(&declared_dependency_requests),
    size_of_val(&dependency_name),
    size_of_val(&dependency_names_match),
    size_of_val(&feature_requests),
    size_of_val(&feature_union),
    size_of_val(&fingerprint),
    size_of_val(&load_snapshots),
    size_of_val(&merge_snapshot_features),
    size_of_val(&named_feature_reference),
    size_of_val(&named_feature_references),
    size_of_val(&package_identities),
    size_of_val(&resolve_feature_request),
    size_of_val(&resolved_feature_requests),
    size_of_val(&reviewed_features),
    size_of_val(&sole_dependency_marker),
];

/// One parsed cross-package feature request.
#[derive(Debug)]
struct CrossPackageFeature {
    /// Local dependency name.
    dependency: String,
    /// Requested dependency feature.
    feature: String,
}

/// Owned inputs shared by Cargo metadata and Cargo tree.
#[derive(Debug)]
pub(super) struct GraphContext {
    /// Governed repository-relative manifest.
    manifest: String,
    /// Tool search path.
    path: OsString,
    /// Public repository root.
    repository: PathBuf,
    /// Shipped Rust target triple.
    triple: String,
}

impl GraphContext {
    /// Return the governed repository-relative manifest.
    pub(super) const fn manifest(&self) -> &str {
        return self.manifest.as_str();
    }

    /// Return the tool search path.
    pub(super) fn path(&self) -> &OsStr {
        return self.path.as_os_str();
    }

    /// Return the public repository root.
    pub(super) fn repository(&self) -> &Path {
        return self.repository.as_path();
    }

    /// Return the shipped Rust target triple.
    pub(super) const fn triple(&self) -> &str {
        return self.triple.as_str();
    }
}

/// Named features requested for active packages by exact Cargo identifier.
type NamedFeatureRequests = BTreeMap<String, BTreeSet<String>>;

/// One resolved cross-package feature request.
#[derive(Debug)]
struct ResolvedFeatureRequest {
    /// Requested dependency feature.
    feature: String,
    /// Exact target package identifier.
    package_identifier: String,
}

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

/// Run locked all-feature Cargo metadata as the package identity superset.
///
/// # Errors
///
/// Returns an error when Cargo fails.
pub(super) fn cargo_metadata(
    repository: &Path,
    path: &OsStr,
    manifest: &str,
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

/// Parse one strong or weak cross-package feature request.
fn cross_package_feature(member: &str) -> Option<CrossPackageFeature> {
    let Some((dependency, feature)) = member.split_once('/') else {
        return None;
    };
    let name = dependency.strip_suffix('?').unwrap_or(dependency);
    if name.is_empty() || feature.is_empty() {
        return None;
    }
    return Some(CrossPackageFeature {
        dependency: name.to_owned(),
        feature: feature.to_owned(),
    });
}

/// Merge features requested directly by active dependency declarations.
fn declared_dependency_requests(
    package: &MetadataPackage,
    node: &ResolveNode,
    requests: &mut NamedFeatureRequests,
) {
    for resolved in &node.dependency_details {
        let matching = package.dependencies.iter().filter(|dependency| {
            let name = dependency_name(dependency);
            return dependency_names_match(name, resolved.name.as_str());
        });
        for dependency in matching {
            requests
                .entry(resolved.package_identifier.clone())
                .or_default()
                .extend(dependency.features.iter().cloned());
        }
    }
}

/// Return the local name used by one dependency declaration.
fn dependency_name(dependency: &MetadataDependency) -> &str {
    return dependency
        .rename
        .as_deref()
        .unwrap_or(dependency.name.as_str());
}

/// Compare Cargo dependency names across hyphen and underscore normalization.
fn dependency_names_match(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    return left
        .chars()
        .zip(right.chars())
        .all(|(left_character, right_character)| {
            return left_character == right_character
                || left_character == '-' && right_character == '_'
                || left_character == '_' && right_character == '-';
        });
}

/// Reconstruct named feature requests entering each active package.
///
/// # Errors
///
/// Returns an error when active metadata is missing a package or resolution.
fn feature_requests(snapshot: &TargetSnapshot) -> CheckResult<NamedFeatureRequests> {
    let packages = snapshot
        .metadata
        .packages
        .iter()
        .map(|package| return (package.identifier.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    let resolve = check_try!(
        snapshot
            .metadata
            .resolve
            .as_ref()
            .ok_or_else(|| return "cargo metadata did not return a resolve graph".to_owned())
    );
    let mut requests = BTreeMap::new();
    for node in &resolve.nodes {
        let package = check_try!(
            packages
                .get(node.identifier.as_str())
                .copied()
                .ok_or_else(|| return format!("active package {} is missing", node.identifier))
        );
        declared_dependency_requests(package, node, &mut requests);
        resolved_feature_requests(package, node, &mut requests);
    }
    return Ok(requests);
}

/// Derive reviewed external feature unions across all target snapshots.
///
/// # Errors
///
/// Returns an error when a resolved package is absent from Cargo's package list.
pub(super) fn feature_union(snapshots: &[TargetSnapshot]) -> CheckResult<FeatureUnion> {
    let mut union = FeatureUnion::new();
    for snapshot in snapshots {
        check_try!(merge_snapshot_features(snapshot, &mut union));
    }
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
) -> CheckResult<LoadedSnapshots> {
    let metadata_output = check_try!(cargo_metadata(repository, path, manifest));
    let mut snapshots = Vec::new();
    for triple in TARGETS {
        let context = GraphContext {
            manifest: manifest.to_owned(),
            path: path.to_owned(),
            repository: repository.to_path_buf(),
            triple: (*triple).to_owned(),
        };
        let active_metadata = check_try!(active::load_active_metadata(
            &context,
            metadata_output.stdout.as_slice()
        ));
        let canonical = check_try!(canonical_graph(&active_metadata.metadata));
        snapshots.push(TargetSnapshot {
            fingerprint: check_try!(fingerprint(canonical.as_str())),
            metadata: active_metadata.metadata,
            metadata_json: active_metadata.serialized,
            triple: (*triple).to_owned(),
        });
    }
    return Ok(LoadedSnapshots {
        locked_metadata_json: check_try!(active::load_lock_wide_metadata(
            metadata_output.stdout.as_slice()
        )),
        snapshots,
    });
}

/// Merge cargo-deny's explicit feature semantics for one active snapshot.
///
/// # Errors
///
/// Returns an error when active metadata is missing a package or resolution.
fn merge_snapshot_features(snapshot: &TargetSnapshot, union: &mut FeatureUnion) -> CheckResult {
    let requests = check_try!(feature_requests(snapshot));
    let packages = snapshot
        .metadata
        .packages
        .iter()
        .map(|package| return (package.identifier.as_str(), package))
        .collect::<BTreeMap<_, _>>();
    let resolve = check_try!(
        snapshot
            .metadata
            .resolve
            .as_ref()
            .ok_or_else(|| return "cargo metadata did not return a resolve graph".to_owned())
    );
    for node in &resolve.nodes {
        let package = check_try!(
            packages
                .get(node.identifier.as_str())
                .copied()
                .ok_or_else(|| return format!("active package {} is missing", node.identifier))
        );
        let declared = reviewed_features(package, node, requests.get(node.identifier.as_str()));
        if package.source.is_none() || declared.is_empty() {
            continue;
        }
        let crate_spec = format!("{}@{}", package.name, package.version);
        union.entry(crate_spec).or_default().extend(declared);
    }
    return Ok(());
}

/// Return one same-package named feature reference.
fn named_feature_reference(member: &str) -> Option<&str> {
    if member.starts_with("dep:") || member.contains('/') {
        return None;
    }
    return Some(member);
}

/// Collect same-package named features reached by enabled manifest features.
fn named_feature_references(package: &MetadataPackage, node: &ResolveNode) -> BTreeSet<String> {
    let enabled = node
        .features
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    return node
        .features
        .iter()
        .filter_map(|feature| return package.declared_features.get(feature.as_str()))
        .flatten()
        .filter_map(|member| return named_feature_reference(member))
        .filter(|feature| return enabled.contains(*feature))
        .map(str::to_owned)
        .collect();
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

/// Resolve one cross-package feature request against active dependency edges.
fn resolve_feature_request(
    node: &ResolveNode,
    request: CrossPackageFeature,
) -> Option<ResolvedFeatureRequest> {
    let resolved = node.dependency_details.iter().find(|candidate| {
        return dependency_names_match(request.dependency.as_str(), candidate.name.as_str());
    });
    return resolved.map(|dependency| {
        return ResolvedFeatureRequest {
            feature: request.feature,
            package_identifier: dependency.package_identifier.clone(),
        };
    });
}

/// Merge enabled cross-package feature references into target requests.
fn resolved_feature_requests(
    package: &MetadataPackage,
    node: &ResolveNode,
    requests: &mut NamedFeatureRequests,
) {
    let resolved = node
        .features
        .iter()
        .filter_map(|enabled| return package.declared_features.get(enabled.as_str()))
        .flatten()
        .filter_map(|member| return cross_package_feature(member))
        .filter_map(|request| return resolve_feature_request(node, request));
    for request in resolved {
        requests
            .entry(request.package_identifier)
            .or_default()
            .extend(once(request.feature));
    }
}

/// Retain named manifest features while excluding Cargo's dependency markers.
fn reviewed_features(
    package: &MetadataPackage,
    node: &ResolveNode,
    requested: Option<&BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut references = named_feature_references(package, node);
    if let Some(external) = requested {
        references.extend(external.iter().cloned());
    }
    return node
        .features
        .iter()
        .filter(|feature| {
            let name = feature.as_str();
            return package.declared_features.get(name).is_some_and(|members| {
                return references.contains(name) || !sole_dependency_marker(name, members);
            });
        })
        .cloned()
        .collect();
}

/// Detect Cargo's flattened marker for an activated optional dependency.
fn sole_dependency_marker(feature: &str, members: &[String]) -> bool {
    return members.len() == 0x0001
        && members
            .first()
            .and_then(|member| return member.strip_prefix("dep:"))
            == Some(feature);
}
