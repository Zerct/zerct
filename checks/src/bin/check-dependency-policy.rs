//! Verify locked, target-specific dependency graphs and reviewed features.

extern crate alloc;

#[path = "check_dependency_policy/deny.rs"]
pub mod deny;

#[path = "check_dependency_policy/graph.rs"]
pub mod graph;

#[path = "check_dependency_policy/policy.rs"]
pub mod policy;

use alloc::collections::{BTreeMap, BTreeSet};

use flate2 as _;

use reqwest as _;

use serde::{Deserialize, Serialize};

use serde_json as _;

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

/// One manifest and every loaded target snapshot.
#[derive(Debug)]
struct ManifestSnapshots {
    /// Governed repository-relative manifest.
    manifest: String,
    /// Locked metadata for every public target.
    snapshots: Vec<TargetSnapshot>,
}

/// Cargo package identity needed for graph normalization.
#[derive(Debug, Deserialize)]
struct MetadataPackage {
    /// Cargo package identifier.
    #[serde(rename = "id")]
    identifier: String,
    /// Package name.
    name: String,
    /// Package source, absent for workspace packages.
    source: Option<String>,
    /// Package version.
    version: String,
}

/// Cargo dependency graph metadata.
#[derive(Debug, Deserialize)]
struct MetadataResolve {
    /// Resolved package nodes.
    nodes: Vec<ResolveNode>,
}

/// Cargo metadata projection used by dependency policy.
#[derive(Debug, Deserialize)]
struct MetadataSnapshot {
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

/// One package node from Cargo's resolved graph.
#[derive(Debug, Deserialize)]
struct ResolveNode {
    /// Resolved dependency package identifiers.
    dependencies: Vec<String>,
    /// Enabled features for this package and target.
    features: Vec<String>,
    /// Cargo package identifier.
    #[serde(rename = "id")]
    identifier: String,
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
    let generate = check_try!(generate_requested());
    let repository = check_try!(repo_root());
    let path = tool_path();
    let mut all_snapshots = Vec::new();
    for manifest in MANIFESTS {
        all_snapshots.push(ManifestSnapshots {
            manifest: (*manifest).to_owned(),
            snapshots: check_try!(graph::load_snapshots(
                repository.as_path(),
                path.as_os_str(),
                manifest
            )),
        });
    }
    if generate {
        let generated_policy = policy::policy_from_snapshots(&all_snapshots);
        check_try!(policy::write_policy(
            repository.as_path(),
            &generated_policy
        ));
    } else {
        let tracked_policy = check_try!(policy::read_policy(repository.as_path()));
        check_try!(policy::check_policy_shape(&tracked_policy));
        check_try!(policy::require_fingerprints(
            &tracked_policy,
            &all_snapshots
        ));
    }
    for manifest_snapshots in &all_snapshots {
        check_try!(deny::run_cargo_deny(
            repository.as_path(),
            path.as_os_str(),
            manifest_snapshots.manifest.as_str(),
            &manifest_snapshots.snapshots,
        ));
    }
    return Ok(());
}

#[cfg(test)]
#[path = "check_dependency_policy_tests/verification.rs"]
mod tests;
