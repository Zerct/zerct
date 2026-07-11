//! Tracked fingerprint policy validation and cargo-deny rendering.

use serde_json::{from_str, to_string, to_string_pretty};

use std::{
    fs::{read_to_string, write as write_file},
    path::Path,
};

use tovuk_public_checks::{check_support::CheckResult, check_try};

use super::{
    BASE_DENY_CONFIG, DependencyPolicy, FEATURE_POLICY, FeatureUnion, MANIFESTS, PackagePolicy,
    RepositorySnapshots, TARGETS, TargetFingerprint,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0007] = [
    size_of_val(&check_policy_shape),
    size_of_val(&policy_from_snapshots),
    size_of_val(&read_policy),
    size_of_val(&render_deny_config),
    size_of_val(&require_fingerprints),
    size_of_val(&serialize_feature_line),
    size_of_val(&write_policy),
];

/// Require the tracked policy to cover exactly both manifests and five targets.
///
/// # Errors
///
/// Returns an error when the schema or ordered public surface drifts.
pub(super) fn check_policy_shape(policy: &DependencyPolicy) -> CheckResult {
    if policy.schema_version != 0x0002 {
        return Err(format!(
            "{FEATURE_POLICY} schemaVersion must be 2, got {}",
            policy.schema_version
        ));
    }
    let manifests_match = policy
        .packages
        .iter()
        .map(|package| return package.manifest.as_str())
        .eq(MANIFESTS.iter().copied());
    if !manifests_match {
        return Err(format!(
            "{FEATURE_POLICY} manifests must be {}",
            MANIFESTS.join(", ")
        ));
    }
    for package in &policy.packages {
        let targets_match = package
            .targets
            .iter()
            .map(|target| return target.triple.as_str())
            .eq(TARGETS.iter().copied());
        if !targets_match {
            return Err(format!(
                "{} targets must be {}",
                package.manifest,
                TARGETS.join(", ")
            ));
        }
    }
    return Ok(());
}

/// Build a compact policy from loaded target snapshots.
pub(super) fn policy_from_snapshots(all_snapshots: &RepositorySnapshots) -> DependencyPolicy {
    let packages = all_snapshots
        .iter()
        .map(|manifest_snapshots| {
            let targets = manifest_snapshots
                .snapshots
                .iter()
                .map(|snapshot| {
                    return TargetFingerprint {
                        sha256: snapshot.fingerprint.clone(),
                        triple: snapshot.triple.clone(),
                    };
                })
                .collect();
            return PackagePolicy {
                manifest: manifest_snapshots.manifest.clone(),
                targets,
            };
        })
        .collect();
    return DependencyPolicy {
        packages,
        schema_version: 0x0002,
    };
}

/// Read and parse the tracked dependency policy.
///
/// # Errors
///
/// Returns an error when the policy cannot be read or parsed.
pub(super) fn read_policy(repository: &Path) -> CheckResult<DependencyPolicy> {
    let source = check_try!(
        read_to_string(repository.join(FEATURE_POLICY))
            .map_err(|error| return format!("read {FEATURE_POLICY}: {error}"))
    );
    return from_str(source.as_str())
        .map_err(|error| return format!("parse {FEATURE_POLICY}: {error}"));
}

/// Render the shared base policy with target-reviewed feature allowances.
///
/// # Errors
///
/// Returns an error when the base config cannot be read or a value cannot serialize.
pub(super) fn render_deny_config(
    repository: &Path,
    features: &FeatureUnion,
) -> CheckResult<String> {
    let mut rendered = check_try!(
        read_to_string(repository.join(BASE_DENY_CONFIG))
            .map_err(|error| return format!("read {BASE_DENY_CONFIG}: {error}"))
    );
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }
    for (crate_spec, allowed) in features {
        let crate_literal = check_try!(
            to_string(crate_spec)
                .map_err(|error| return format!("serialize crate feature policy: {error}"))
        );
        rendered.push_str("\n[[bans.features]]\ncrate = ");
        rendered.push_str(crate_literal.as_str());
        rendered.push_str("\nallow = [\n");
        let feature_lines = check_try!(
            allowed
                .iter()
                .map(|feature| return serialize_feature_line(feature.as_str()))
                .collect::<CheckResult<Vec<_>>>()
        );
        rendered.push_str(feature_lines.join("").as_str());
        rendered.push_str("]\nexact = false\n");
    }
    return Ok(rendered);
}

/// Validate tracked fingerprints before any feature allowance is derived.
///
/// # Errors
///
/// Returns an error describing every target graph drift.
pub(super) fn require_fingerprints(
    policy: &DependencyPolicy,
    all_snapshots: &RepositorySnapshots,
) -> CheckResult {
    let manifest_pairs = policy.packages.iter().zip(all_snapshots);
    if let Some((package, manifest_snapshots)) = manifest_pairs
        .clone()
        .find(|pair| return pair.0.manifest != pair.1.manifest)
    {
        return Err(format!(
            "dependency policy manifest mismatch: {} != {}",
            package.manifest, manifest_snapshots.manifest
        ));
    }
    let drift = manifest_pairs
        .flat_map(|(package, manifest_snapshots)| {
            return package
                .targets
                .iter()
                .zip(&manifest_snapshots.snapshots)
                .map(|target_pair| {
                    let tracked = target_pair.0;
                    let actual = target_pair.1;
                    let triple_drifted = tracked.triple != actual.triple;
                    let fingerprint_drifted = tracked.sha256 != actual.fingerprint;
                    let message = format!(
                        "{} {}: expected {}, got {}",
                        manifest_snapshots.manifest,
                        actual.triple,
                        tracked.sha256,
                        actual.fingerprint
                    );
                    return (triple_drifted || fingerprint_drifted, message);
                });
        })
        .find(|comparison| return comparison.0)
        .map(|comparison| return comparison.1);
    let Some(message) = drift else {
        return Ok(());
    };
    return Err(format!(
        "Locked target dependency graphs drifted. Review the public graph, then run `cargo run --locked --manifest-path checks/Cargo.toml --bin check-dependency-policy -- --generate`:\n{message}"
    ));
}

/// Serialize one reviewed dependency feature as a cargo-deny array line.
///
/// # Errors
///
/// Returns an error when the feature cannot be serialized.
fn serialize_feature_line(feature: &str) -> CheckResult<String> {
    return to_string(feature)
        .map(|literal| return format!("  {literal},\n"))
        .map_err(|error| return format!("serialize dependency feature policy: {error}"));
}

/// Write the compact deterministic fingerprint policy.
///
/// # Errors
///
/// Returns an error when serialization or the atomic replacement fails.
pub(super) fn write_policy(repository: &Path, policy: &DependencyPolicy) -> CheckResult {
    let mut serialized = check_try!(
        to_string_pretty(policy)
            .map_err(|error| return format!("serialize {FEATURE_POLICY}: {error}"))
    );
    serialized.push('\n');
    return write_file(repository.join(FEATURE_POLICY), serialized)
        .map_err(|error| return format!("write {FEATURE_POLICY}: {error}"));
}
