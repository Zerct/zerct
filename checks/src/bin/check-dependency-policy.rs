//! Verify locked, target-specific dependency graphs and reviewed features.

#[path = "check_dependency_policy/active.rs"]
pub mod active;

extern crate alloc;

#[path = "check_dependency_policy/deny.rs"]
pub mod deny;

#[path = "check_dependency_policy/graph.rs"]
pub mod graph;

#[path = "check_dependency_policy/policy.rs"]
pub mod policy;

#[path = "check_dependency_policy/tree.rs"]
pub mod tree;

use alloc::collections::{BTreeMap, BTreeSet};

use flate2 as _;

use http as _;

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use tokio as _;

use url as _;

use serde::{Deserialize, Serialize};

use serde_json::Value;

use sha2 as _;

use std::{
    env,
    io::{Write as _, stderr, stdout},
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::{
    check_support::{CheckResult, repo_root, tool_path},
    check_try,
};

/// Shared cargo-deny configuration extended only in temporary files.
const BASE_DENY_CONFIG: &str = "deny.toml";

/// Tracked target fingerprint policy.
const FEATURE_POLICY: &str = "dependency-feature-policy.json";

/// Public Cargo manifests governed by dependency policy.
const MANIFESTS: &[&str] = &["checks/Cargo.toml", "crates/tovuk/Cargo.toml"];

/// Supported public release targets whose dependency graphs are reviewed.
const TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [size_of_val(&generate_requested), size_of_val(&run)];

/// One pruned metadata document and its cargo-deny serialization.
#[derive(Debug)]
struct ActiveMetadata {
    /// Parsed metadata used for canonical fingerprints and features.
    metadata: MetadataSnapshot,
    /// Exact pruned JSON supplied to cargo-deny.
    serialized: Vec<u8>,
}

/// Features explicitly declared by one package manifest.
type DeclaredFeatures = BTreeMap<String, Vec<String>>;

/// Tracked dependency feature policy document.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DependencyPolicy {
    /// Per-manifest target fingerprints.
    packages: Vec<PackagePolicy>,
    /// Policy schema version.
    schema_version: u32,
}

/// Feature set enabled for one exact external crate.
type EnabledFeatures = BTreeSet<String>;

/// Exact external crate features enabled across public targets.
type FeatureUnion = BTreeMap<String, EnabledFeatures>;

/// Locked metadata and every exact target snapshot for one manifest.
#[derive(Debug)]
struct LoadedSnapshots {
    /// Unpruned all-feature metadata covering every locked package.
    locked_metadata_json: Vec<u8>,
    /// Exact active metadata for every shipped target.
    snapshots: Vec<TargetSnapshot>,
}

/// One manifest and every loaded target snapshot.
#[derive(Debug)]
struct ManifestSnapshots {
    /// Unpruned all-feature metadata covering every locked package.
    locked_metadata_json: Vec<u8>,
    /// Governed repository-relative manifest.
    manifest: String,
    /// Locked metadata for every public target.
    snapshots: Vec<TargetSnapshot>,
}

/// One dependency declaration retained from Cargo package metadata.
#[derive(Debug, Deserialize, Serialize)]
struct MetadataDependency {
    /// Features requested on the dependency.
    features: Vec<String>,
    /// Dependency package name before an optional rename.
    name: String,
    /// Remaining dependency declaration metadata preserved for cargo-deny.
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
    /// Local dependency name when renamed.
    rename: Option<String>,
}

/// Cargo package identity needed for graph normalization.
#[derive(Debug, Deserialize, Serialize)]
struct MetadataPackage {
    /// Features explicitly declared by the package manifest.
    #[serde(rename = "features")]
    declared_features: DeclaredFeatures,
    /// Dependency declarations used to reconstruct named feature requests.
    dependencies: Vec<MetadataDependency>,
    /// Cargo package identifier.
    #[serde(rename = "id")]
    identifier: String,
    /// Package name.
    name: String,
    /// Remaining Cargo package metadata preserved for cargo-deny.
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
    /// Package source, absent for workspace packages.
    source: Option<String>,
    /// Package version.
    version: String,
}

/// Cargo dependency graph metadata.
#[derive(Debug, Deserialize, Serialize)]
struct MetadataResolve {
    /// Resolved package nodes.
    nodes: Vec<ResolveNode>,
    /// Remaining Cargo resolution metadata preserved for cargo-deny.
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
    /// Root package selected by the governed manifest.
    root: Option<String>,
}

/// Cargo metadata projection used by dependency policy.
#[derive(Debug, Deserialize, Serialize)]
struct MetadataSnapshot {
    /// Remaining top-level Cargo metadata preserved for cargo-deny.
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
    /// All packages available to the resolved graph.
    packages: Vec<MetadataPackage>,
    /// Target-filtered resolved graph.
    resolve: Option<MetadataResolve>,
}

/// Cargo package identifier to stable public fingerprint identity.
type PackageIdentities = BTreeMap<String, String>;

/// Policy for one public Cargo manifest.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PackagePolicy {
    /// Repository-relative Cargo manifest path.
    manifest: String,
    /// Target-specific graph fingerprints.
    targets: Vec<TargetFingerprint>,
}

/// Loaded target snapshots for all governed manifests.
type RepositorySnapshots = [ManifestSnapshots];

/// Detailed Cargo dependency edge retained for cargo-deny.
#[derive(Debug, Deserialize, Serialize)]
struct ResolveDependency {
    /// Dependency name used by resolved feature references.
    name: String,
    /// Remaining dependency-edge metadata preserved for cargo-deny.
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
    /// Resolved package identifier at the edge destination.
    #[serde(rename = "pkg")]
    package_identifier: String,
}

/// One package node from Cargo's resolved graph.
#[derive(Debug, Deserialize, Serialize)]
struct ResolveNode {
    /// Resolved dependency package identifiers.
    dependencies: Vec<String>,
    /// Detailed resolved dependency edges.
    #[serde(rename = "deps")]
    dependency_details: Vec<ResolveDependency>,
    /// Enabled features for this package and target.
    features: Vec<String>,
    /// Cargo package identifier.
    #[serde(rename = "id")]
    identifier: String,
    /// Remaining Cargo node metadata preserved for cargo-deny.
    #[serde(flatten)]
    other: BTreeMap<String, Value>,
}

/// Tracked fingerprint for one target triple.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TargetFingerprint {
    /// Lowercase SHA-256 of the normalized graph.
    sha256: String,
    /// Rust target triple.
    triple: String,
}

/// Loaded target snapshot and its normalized fingerprint.
#[derive(Debug)]
struct TargetSnapshot {
    /// Normalized graph SHA-256.
    fingerprint: String,
    /// Parsed Cargo metadata.
    metadata: MetadataSnapshot,
    /// Pruned target-specific Cargo metadata supplied to cargo-deny.
    metadata_json: Vec<u8>,
    /// Target triple used to resolve metadata.
    triple: String,
}

/// Parse the optional explicit fingerprint-generation argument.
///
/// # Errors
///
/// Returns an error when any unsupported argument is supplied.
fn generate_requested() -> CheckResult<bool> {
    let arguments = env::args().skip(0x0001).collect::<Vec<_>>();
    return match (arguments.first().map(String::as_str), arguments.len()) {
        (None, 0x0000) => Ok(false),
        (Some("--generate"), 0x0001) => Ok(true),
        (None | Some(_), _) => Err("usage: check-dependency-policy [--generate]".to_owned()),
    };
}

fn main() -> ExitCode {
    let result = run();
    match result {
        Ok(()) => {
            drop(writeln!(
                stdout().lock(),
                "Dependency policy passed for both public Cargo manifests."
            ));
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            drop(writeln!(stderr().lock(), "{error}"));
            return ExitCode::FAILURE;
        }
    }
}

/// Execute dependency policy validation or explicit fingerprint generation.
///
/// # Errors
///
/// Returns an error when arguments, fingerprints, metadata, or cargo-deny fail.
fn run() -> CheckResult {
    let repository = check_try!(repo_root());
    let path = tool_path();
    let snapshots = check_try!(
        MANIFESTS
            .iter()
            .map(|manifest| -> CheckResult<ManifestSnapshots> {
                let loaded = check_try!(graph::load_snapshots(
                    repository.as_path(),
                    path.as_os_str(),
                    manifest
                ));
                return Ok(ManifestSnapshots {
                    locked_metadata_json: loaded.locked_metadata_json,
                    manifest: (*manifest).to_owned(),
                    snapshots: loaded.snapshots,
                });
            })
            .collect::<CheckResult<Vec<_>>>()
    );
    let generated_policy = if check_try!(generate_requested()) {
        Some(policy::policy_from_snapshots(&snapshots))
    } else {
        let tracked = check_try!(policy::read_policy(repository.as_path()));
        check_try!(policy::check_policy_shape(&tracked));
        check_try!(policy::require_fingerprints(&tracked, &snapshots));
        None
    };
    check_try!(
        snapshots
            .iter()
            .try_for_each(|manifest_snapshots| return deny::run_cargo_deny(
                repository.as_path(),
                path.as_os_str(),
                manifest_snapshots,
            ))
    );
    check_try!(generated_policy.as_ref().map_or(Ok(()), |generated| {
        return policy::write_policy(repository.as_path(), generated);
    }));
    return Ok(());
}

#[cfg(test)]
#[path = "check_dependency_policy_tests/verification.rs"]
mod tests;
