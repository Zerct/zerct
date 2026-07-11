use alloc::collections::{BTreeMap, BTreeSet};

use core::slice;

use serde_json::{Value, from_slice};

use std::{
    env,
    fs::{create_dir_all, read as read_file, read_to_string, remove_dir_all, write as write_file},
    process::id as process_id,
};

use super::{
    MetadataSnapshot, TargetSnapshot,
    active::{load_lock_wide_metadata, prune_metadata_source},
    deny::{TemporaryFile, cargo_deny_arguments, lock_wide_arguments},
    graph::feature_union,
    policy::render_deny_config,
};

use tovuk_public_checks::{
    check_support::{CheckResult, repo_root},
    check_try,
};

/// Exact global and check arguments required for each cargo-deny target.
const EXPECTED_LOCK_DENY_ARGUMENTS: &[&str] = &[
    "--manifest-path",
    "fixture/Cargo.toml",
    "--workspace",
    "--metadata-path",
    "/tmp/metadata.json",
    "--config",
    "/tmp/deny.toml",
    "--all-features",
    "--locked",
    "check",
    "--deny",
    "warnings",
    "advisories",
    "licenses",
    "sources",
];

/// Exact target-active cargo-deny arguments.
const EXPECTED_TARGET_DENY_ARGUMENTS: &[&str] = &[
    "--manifest-path",
    "fixture/Cargo.toml",
    "--metadata-path",
    "/tmp/metadata.json",
    "--config",
    "/tmp/deny.toml",
    "--target",
    "x86_64-unknown-linux-gnu",
    "--all-features",
    "--locked",
    "check",
    "--deny",
    "warnings",
    "all",
];

/// Cargo metadata containing active and lock-only duplicate-version packages.
const METADATA_FIXTURE: &str = r#"{
  "packages": [
    {"dependencies":[{"features":[],"name":"active","rename":null},{"features":[],"name":"duplicate","rename":null},{"features":[],"name":"optional","rename":null}],"features":{"default":[]},"id":"path+file:///fixture#root@0.1.0","name":"root","source":null,"version":"0.1.0"},
    {"dependencies":[],"features":{"implicit":["dep:implicit"],"std":[]},"id":"registry+https://example.invalid/index#active@1.0.0","name":"active","source":"registry+https://example.invalid/index","version":"1.0.0"},
    {"dependencies":[],"features":{"std":[]},"id":"registry+https://example.invalid/index#duplicate@1.0.0","name":"duplicate","source":"registry+https://example.invalid/index","version":"1.0.0"},
    {"dependencies":[],"features":{"alloc":[]},"id":"registry+https://example.invalid/index#duplicate@2.0.0","name":"duplicate","source":"registry+https://example.invalid/index","version":"2.0.0"},
    {"dependencies":[],"features":{},"id":"registry+https://example.invalid/index#optional@9.0.0","name":"optional","source":"registry+https://example.invalid/index","version":"9.0.0"}
  ],
  "resolve": {
    "nodes": [
      {
        "id":"path+file:///fixture#root@0.1.0",
        "dependencies":[
          "registry+https://example.invalid/index#active@1.0.0",
          "registry+https://example.invalid/index#duplicate@1.0.0",
          "registry+https://example.invalid/index#duplicate@2.0.0",
          "registry+https://example.invalid/index#optional@9.0.0"
        ],
        "deps":[
          {"name":"active","pkg":"registry+https://example.invalid/index#active@1.0.0"},
          {"name":"duplicate","pkg":"registry+https://example.invalid/index#duplicate@1.0.0"},
          {"name":"duplicate","pkg":"registry+https://example.invalid/index#duplicate@2.0.0"},
          {"name":"optional","pkg":"registry+https://example.invalid/index#optional@9.0.0"}
        ],
        "features":["metadata-only"]
      },
      {"id":"registry+https://example.invalid/index#active@1.0.0","dependencies":[],"deps":[],"features":[]},
      {"id":"registry+https://example.invalid/index#duplicate@1.0.0","dependencies":[],"deps":[],"features":[]},
      {"id":"registry+https://example.invalid/index#duplicate@2.0.0","dependencies":[],"deps":[],"features":[]},
      {"id":"registry+https://example.invalid/index#optional@9.0.0","dependencies":[],"deps":[],"features":[]}
    ],
    "root":"path+file:///fixture#root@0.1.0"
  },
  "workspace_default_members":["path+file:///fixture#root@0.1.0"],
  "workspace_members":["path+file:///fixture#root@0.1.0"]
}"#;

/// Active tree excluding every optional duplicate-version package.
const TREE_WITHOUT_OPTIONAL_PACKAGES: &str =
    "0\troot v0.1.0 (/fixture)\tdefault\n1\tactive v1.0.0\tstd\n";

/// Active tree that deliberately contains both versions of one package.
const TREE_WITH_ACTIVE_DUPLICATES: &str =
    "0\troot v0.1.0 (/fixture)\tdefault\n1\tduplicate v1.0.0\tstd\n1\tduplicate v2.0.0\talloc\n";

/// Verify active duplicate versions reach cargo-deny under its deny policy.
///
/// # Errors
///
/// Returns an error when active versions or strict cargo-deny inputs drift.
#[test]
fn active_duplicate_versions_reach_strict_cargo_deny() -> CheckResult {
    let active = check_try!(prune_metadata_source(
        METADATA_FIXTURE.as_bytes(),
        TREE_WITH_ACTIVE_DUPLICATES
    ));
    let metadata = active.metadata;
    let versions = metadata
        .packages
        .iter()
        .filter(|package| return package.name == "duplicate")
        .map(|package| return package.version.as_str())
        .collect();
    check_try!(require_sequence(
        versions,
        &["1.0.0", "2.0.0"],
        "both active duplicate versions must reach cargo-deny"
    ));
    let repository = check_try!(repo_root());
    let config = check_try!(
        read_to_string(repository.join("deny.toml"))
            .map_err(|error| return format!("read deny.toml: {error}"))
    );
    let strict = config.contains("multiple-versions = \"deny\"")
        && config.contains("skip = []")
        && config.contains("skip-tree = []");
    if !strict {
        return Err("active duplicate versions must remain denied without skips".to_owned());
    }
    let arguments = cargo_deny_arguments(
        "fixture/Cargo.toml",
        "/tmp/deny.toml",
        "/tmp/metadata.json",
        "x86_64-unknown-linux-gnu",
    );
    check_try!(require_sequence(
        arguments.iter().map(String::as_str).collect(),
        EXPECTED_TARGET_DENY_ARGUMENTS,
        "target-specific cargo-deny argument scope drifted"
    ));
    return Ok(());
}

/// Verify indistinguishable package sources are rejected rather than guessed.
///
/// # Panics
///
/// Panics when ambiguous Cargo package identities are accepted.
#[test]
fn ambiguous_package_sources_fail_closed() {
    let ambiguous = METADATA_FIXTURE.replace("\"version\":\"2.0.0\"", "\"version\":\"1.0.0\"");
    let result = prune_metadata_source(ambiguous.as_bytes(), TREE_WITH_ACTIVE_DUPLICATES);
    assert!(
        result.is_err_and(|error| return error.contains("maps to 2 metadata sources")),
        "ambiguous name/version identities must fail closed"
    );
}

/// Verify generated policy is written only after cargo-deny succeeds.
///
/// # Errors
///
/// Returns an error when the generation commit boundary moves before checks.
#[test]
fn generation_defers_policy_write_until_after_deny() -> CheckResult {
    let source = include_str!("../check-dependency-policy.rs");
    let deny_position = check_try!(
        source
            .find("deny::run_cargo_deny")
            .ok_or_else(|| return "dependency-policy deny phase is missing".to_owned())
    );
    let write_position = check_try!(
        source
            .rfind("policy::write_policy")
            .ok_or_else(|| return "dependency-policy generation commit is missing".to_owned())
    );
    if deny_position < write_position {
        return Ok(());
    }
    return Err("generated policy can be written before cargo-deny succeeds".to_owned());
}

/// Verify implicit dependency features never enter exact allowances.
///
/// # Errors
///
/// Returns an error when Cargo's declared dependency marker survives filtering.
#[test]
fn implicit_dependency_features_are_not_allowed() -> CheckResult {
    let active = check_try!(prune_metadata_source(
        METADATA_FIXTURE.as_bytes(),
        "0\troot v0.1.0 (/fixture)\tdefault\n1\tactive v1.0.0\tstd,implicit\n",
    ));
    let snapshot = TargetSnapshot {
        fingerprint: "fixture".to_owned(),
        metadata: active.metadata,
        metadata_json: active.serialized,
        triple: "x86_64-unknown-linux-gnu".to_owned(),
    };
    let policy = check_try!(feature_union(slice::from_ref(&snapshot)));
    let allowed = check_try!(
        policy
            .get("active@1.0.0")
            .ok_or_else(|| return "active fixture feature policy is missing".to_owned())
    );
    return require_sequence(
        allowed.iter().map(String::as_str).collect(),
        &["std"],
        "implicit dependency feature entered exact allowances",
    );
}

/// Verify lock-only optional edges cannot affect a target fingerprint.
///
/// # Errors
///
/// Returns an error when any inactive edge or feature remains.
#[test]
fn inactive_optional_edges_are_pruned() -> CheckResult {
    let active = check_try!(prune_metadata_source(
        METADATA_FIXTURE.as_bytes(),
        TREE_WITHOUT_OPTIONAL_PACKAGES,
    ));
    let resolve = check_try!(
        active
            .metadata
            .resolve
            .ok_or_else(|| return "pruned metadata lost its resolve graph".to_owned())
    );
    let root = check_try!(
        resolve
            .nodes
            .first()
            .ok_or_else(|| return "pruned metadata lost its root node".to_owned())
    );
    let dependencies = root.dependencies.iter().map(String::as_str).collect();
    check_try!(require_sequence(
        dependencies,
        &["registry+https://example.invalid/index#active@1.0.0"],
        "pruned metadata retained an inactive edge"
    ));
    let features = root.features.iter().map(String::as_str).collect();
    return require_sequence(
        features,
        &["default"],
        "pruned metadata retained an inactive feature",
    );
}

/// Verify lock-only optional packages cannot affect a target fingerprint.
///
/// # Errors
///
/// Returns an error when any inactive package remains.
#[test]
fn inactive_optional_packages_are_pruned() -> CheckResult {
    let active = check_try!(prune_metadata_source(
        METADATA_FIXTURE.as_bytes(),
        TREE_WITHOUT_OPTIONAL_PACKAGES,
    ));
    let package_names = active
        .metadata
        .packages
        .iter()
        .map(|package| return package.name.as_str())
        .collect();
    check_try!(require_sequence(
        package_names,
        &["root", "active"],
        "inactive optional package remained in metadata"
    ));
    let source = check_try!(
        String::from_utf8(active.serialized)
            .map_err(|error| return format!("decode pruned metadata: {error}"))
    );
    if source.contains("optional@9.0.0") {
        return Err("serialized metadata retained a lock-only package".to_owned());
    }
    return Ok(());
}

/// Verify lock-wide audits exclude only active-graph ban checks.
///
/// # Errors
///
/// Returns an error when lock-wide cargo-deny arguments drift.
#[test]
fn lock_wide_arguments_are_exact() -> CheckResult {
    let arguments =
        lock_wide_arguments("fixture/Cargo.toml", "/tmp/deny.toml", "/tmp/metadata.json");
    return require_sequence(
        arguments.iter().map(String::as_str).collect(),
        EXPECTED_LOCK_DENY_ARGUMENTS,
        "lock-wide cargo-deny argument scope drifted",
    );
}

/// Verify every metadata package becomes a lock-wide graph root.
///
/// # Errors
///
/// Returns an error when inactive packages are absent from lock-wide roots.
#[test]
fn lock_wide_metadata_promotes_inactive_packages() -> CheckResult {
    let serialized = check_try!(load_lock_wide_metadata(METADATA_FIXTURE.as_bytes()));
    let metadata = check_try!(
        from_slice::<MetadataSnapshot>(serialized.as_slice())
            .map_err(|error| return format!("parse lock-wide fixture: {error}"))
    );
    let members = check_try!(
        metadata
            .other
            .get("workspace_members")
            .and_then(Value::as_array)
            .ok_or_else(|| return "lock-wide workspace members are missing".to_owned())
    );
    let defaults = check_try!(
        metadata
            .other
            .get("workspace_default_members")
            .and_then(Value::as_array)
            .ok_or_else(|| return "lock-wide default members are missing".to_owned())
    );
    let includes_optional = members.iter().filter_map(Value::as_str).any(|identifier| {
        return identifier.ends_with("#optional@9.0.0");
    });
    if members.len() != metadata.packages.len() || members != defaults || !includes_optional {
        return Err("lock-wide roots do not include every inactive package".to_owned());
    }
    return Ok(());
}

/// Verify dependency markers requested by name remain exact allowances.
///
/// # Errors
///
/// Returns an error when a direct dependency feature request is discarded.
#[test]
fn named_dependency_features_are_allowed() -> CheckResult {
    let metadata = METADATA_FIXTURE.replace(
        "{\"features\":[],\"name\":\"active\",\"rename\":null}",
        "{\"features\":[\"implicit\"],\"name\":\"active\",\"rename\":null}",
    );
    let active = check_try!(prune_metadata_source(
        metadata.as_bytes(),
        "0\troot v0.1.0 (/fixture)\tdefault\n1\tactive v1.0.0\tstd,implicit\n",
    ));
    let snapshot = TargetSnapshot {
        fingerprint: "fixture".to_owned(),
        metadata: active.metadata,
        metadata_json: active.serialized,
        triple: "x86_64-unknown-linux-gnu".to_owned(),
    };
    let policy = check_try!(feature_union(slice::from_ref(&snapshot)));
    let allowed = check_try!(
        policy
            .get("active@1.0.0")
            .ok_or_else(|| return "active fixture feature policy is missing".to_owned())
    );
    return require_sequence(
        allowed.iter().map(String::as_str).collect(),
        &["implicit", "std"],
        "named dependency feature was discarded",
    );
}

/// Verify exclusive creation cannot replace an occupied candidate.
///
/// # Errors
///
/// Returns an error when fixture I/O or collision handling fails.
#[test]
fn occupied_candidate_is_preserved() -> CheckResult {
    let directory = env::temp_dir().join(format!("tovuk-dependency-policy-test-{}", process_id()));
    check_try!(
        create_dir_all(directory.as_path())
            .map_err(|error| return format!("create {}: {error}", directory.display()))
    );
    let candidate = directory.join("occupied.toml");
    check_try!(
        write_file(candidate.as_path(), b"trusted")
            .map_err(|error| return format!("write {}: {error}", candidate.display()))
    );
    if check_try!(TemporaryFile::create_new(candidate.clone(), b"untrusted")).is_some() {
        return Err("an occupied path was replaced".to_owned());
    }
    let preserved = check_try!(
        read_file(candidate.as_path())
            .map_err(|error| return format!("read {}: {error}", candidate.display()))
    );
    check_try!(
        remove_dir_all(directory.as_path())
            .map_err(|error| return format!("remove {}: {error}", directory.display()))
    );
    if preserved != b"trusted" {
        return Err("collision handling changed trusted bytes".to_owned());
    }
    return Ok(());
}

/// Verify generated cargo-deny feature policy rejects every unlisted feature.
///
/// # Errors
///
/// Returns an error when policy rendering is invalid or not exact.
#[test]
fn rendered_feature_policy_is_exact() -> CheckResult {
    let repository = check_try!(repo_root());
    let features = BTreeMap::from([(
        "dependency@1.0.0".to_owned(),
        BTreeSet::from(["std".to_owned()]),
    )]);
    let rendered = check_try!(render_deny_config(repository.as_path(), &features));
    let ring_ban = "{ crate = \"ring\", reason = \"Use AWS-LC for public TLS cryptography.\" }";
    let exact_features = rendered.contains("exact = true") && !rendered.contains("exact = false");
    if exact_features && rendered.contains(ring_ban) {
        return Ok(());
    }
    return Err("feature policy must be exact and require the active AWS-LC provider".to_owned());
}

/// Require an exact ordered sequence of string values.
///
/// # Errors
///
/// Returns the supplied error message when the sequence differs.
fn require_sequence(actual: Vec<&str>, expected: &[&str], message: &str) -> CheckResult {
    if actual.into_iter().eq(expected.iter().copied()) {
        return Ok(());
    }
    return Err(message.to_owned());
}
