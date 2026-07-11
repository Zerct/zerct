//! npm `.tgz` package artifact policy.

use serde_json::{Map, Value, from_slice};

use std::path::Path;

use tovuk_public_checks::check_support::CheckResult;

use super::{
    WrapperEvidence,
    archive::read_tar_gz,
    policy::{require_file_name, require_license, require_native_targets},
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&json_string),
    size_of_val(&require_no_dependencies),
    size_of_val(&require_package_json),
    size_of_val(&validate),
];

/// Read one required JSON string field by key path.
///
/// # Errors
///
/// Returns an error when the parent or string field is absent.
fn json_string<'value>(
    value: &'value Value,
    parent: Option<&str>,
    key: &str,
    label: &str,
) -> CheckResult<&'value str> {
    let container = match parent {
        Some(parent_key) => check_try!(
            value
                .get(parent_key)
                .ok_or_else(|| return format!("npm package.json is missing {parent_key}"))
        ),
        None => value,
    };
    return container
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| return format!("npm package.json {label} must be a string"));
}

/// Require every dependency collection to be absent or empty.
///
/// # Errors
///
/// Returns an error when a dependency collection contains an entry.
fn require_no_dependencies(value: &Value) -> CheckResult {
    for field in [
        "bundledDependencies",
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        let Some(dependencies) = value.get(field) else {
            continue;
        };
        let empty = dependencies.as_object().is_some_and(Map::is_empty)
            || dependencies.as_array().is_some_and(Vec::is_empty);
        if !empty {
            return Err(format!("npm package.json {field} must be absent or empty"));
        }
    }
    return Ok(());
}

/// Require the packaged npm identity and launcher metadata.
///
/// # Errors
///
/// Returns an error when package JSON is invalid or metadata differs.
fn require_package_json(contents: &[u8], version: &str) -> CheckResult {
    let value = check_try!(
        from_slice::<Value>(contents)
            .map_err(|error| return format!("parse npm package.json: {error}"))
    );
    for (parent, key, expected, label) in [
        (None, "name", "tovuk", "name"),
        (None, "version", version, "version"),
        (None, "license", "MIT", "license"),
        (None, "type", "module", "module type"),
        (Some("bin"), "tovuk", "bin/tovuk.mjs", "tovuk binary"),
        (
            Some("scripts"),
            "postinstall",
            "node install.mjs",
            "postinstall script",
        ),
    ] {
        let actual = check_try!(json_string(&value, parent, key, label));
        if actual != expected {
            return Err(format!(
                "npm package.json {label} must be {expected}, got {actual}"
            ));
        }
    }
    return require_no_dependencies(&value);
}

/// Validate one npm package artifact.
///
/// # Errors
///
/// Returns an error when the archive file set, metadata, runtime adapters,
/// license, or native target manifest differs from the public npm contract.
pub(super) fn validate(path: &Path, version: &str) -> CheckResult<WrapperEvidence> {
    check_try!(require_file_name(
        path,
        format!("tovuk-{version}.tgz").as_str(),
        "npm",
    ));
    let archive = check_try!(read_tar_gz(path, "npm"));
    let expected = [
        "package/LICENSE",
        "package/README.md",
        "package/bin/tovuk.mjs",
        "package/install-policy.mjs",
        "package/install.mjs",
        "package/native-release-targets.json",
        "package/package.json",
    ]
    .map(str::to_owned);
    check_try!(archive.require_exact_files(&expected, "npm"));
    check_try!(require_package_json(
        check_try!(archive.file("package/package.json", "npm")),
        version,
    ));
    let license = check_try!(archive.file("package/LICENSE", "npm"));
    check_try!(require_license(license, "npm"));
    let native_targets = check_try!(archive.file("package/native-release-targets.json", "npm",));
    check_try!(require_native_targets(
        native_targets,
        "npm native target manifest"
    ));
    return Ok(WrapperEvidence {
        license: license.to_vec(),
        native_targets: native_targets.to_vec(),
    });
}
