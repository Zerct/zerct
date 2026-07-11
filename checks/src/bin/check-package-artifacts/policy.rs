//! Shared package metadata, filename, and public-leakage policy.

use core::str::from_utf8;

use serde_json::{Value, from_slice};

use std::{ffi::OsStr, path::Path};

use tovuk_public_checks::check_support::CheckResult;

/// Exact path components forbidden in public package archives.
const FORBIDDEN_COMPONENTS: &[&str] = &[
    ".agents",
    ".aws",
    ".cache",
    ".cargo",
    ".claude",
    ".codex",
    ".config",
    ".cursor",
    ".ds_store",
    ".envrc",
    ".git",
    ".git-credentials",
    ".github",
    ".idea",
    ".mypy_cache",
    ".netrc",
    ".nox",
    ".npmrc",
    ".pytest_cache",
    ".pypirc",
    ".ruff_cache",
    ".ssh",
    ".tovuk",
    ".tox",
    ".venv",
    ".vscode",
    "agents.md",
    "agents.override.md",
    "build",
    "claude.md",
    "credentials",
    "credentials.toml",
    "dist",
    "gemini.md",
    "hosts.yml",
    "id_ed25519",
    "id_rsa",
    "node_modules",
    "npm-support-request.md",
    "publishing.md",
    "session-token",
    "target",
    "terraform.tfstate",
    "thumbs.db",
    "venv",
];

/// Filename suffixes forbidden in public package archives.
const FORBIDDEN_SUFFIXES: &[&str] = &[
    ".egg-info",
    ".jks",
    ".key",
    ".keystore",
    ".log",
    ".p12",
    ".pem",
    ".pfx",
    ".pyc",
    ".pyo",
    ".secret",
    ".tfvars",
];

/// Public package license identifier.
const LICENSE_IDENTIFIER: &str = "MIT";

/// Text signatures that must never be present in published packages.
const SECRET_SIGNATURES: &[&str] = &[
    "-----BEGIN EC PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----",
    "-----BEGIN PRIVATE KEY-----",
    "-----BEGIN RSA PRIVATE KEY-----",
    "github_pat_",
    "ghp_",
    "sk_live_",
    "xoxb-",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000d] = [
    size_of_val(&reject_sensitive_content),
    size_of_val(&reject_sensitive_path),
    size_of_val(&require_equal_bytes),
    size_of_val(&require_file_name),
    size_of_val(&require_license),
    size_of_val(&require_metadata),
    size_of_val(&require_wheel_metadata_field),
    size_of_val(&require_native_targets),
    size_of_val(&require_python_project),
    size_of_val(&require_python_version),
    size_of_val(&require_toml_package),
    size_of_val(&require_version),
    size_of_val(&require_wheel_file_name),
];

/// Reject secret signatures in a packaged file.
///
/// # Errors
///
/// Returns an error when text contains a recognized private-key or token
/// signature.
pub(super) fn reject_sensitive_content(path: &str, contents: &[u8]) -> CheckResult {
    let Ok(text) = from_utf8(contents) else {
        return Ok(());
    };
    for signature in SECRET_SIGNATURES {
        if text.contains(signature) {
            return Err(format!(
                "package member {path} contains forbidden secret signature {signature}"
            ));
        }
    }
    return Ok(());
}

/// Reject secret, local-state, and agent-only archive paths.
///
/// # Errors
///
/// Returns an error when a member path names local configuration, credentials,
/// generated state, or repository-only agent guidance.
pub(super) fn reject_sensitive_path(path: &str) -> CheckResult {
    for component in path.split('/') {
        let lower = component.to_ascii_lowercase();
        let forbidden = lower.starts_with(".env")
            || FORBIDDEN_COMPONENTS.contains(&lower.as_str())
            || FORBIDDEN_SUFFIXES
                .iter()
                .any(|suffix| return lower.ends_with(suffix));
        if forbidden {
            return Err(format!("package archive contains forbidden member {path}"));
        }
    }
    return Ok(());
}

/// Require byte-for-byte synchronization between artifacts.
///
/// # Errors
///
/// Returns an error when the packaged files differ.
pub(super) fn require_equal_bytes(left: &[u8], right: &[u8], label: &str) -> CheckResult {
    return (left == right)
        .then_some(())
        .ok_or_else(|| return format!("{label} must match exactly"));
}

/// Require one exact artifact filename.
///
/// # Errors
///
/// Returns an error when the artifact path is not UTF-8 or its filename differs.
pub(super) fn require_file_name(path: &Path, expected: &str, label: &str) -> CheckResult {
    let actual = check_try!(
        path.file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| return format!("{label} archive filename must be UTF-8"))
    );
    return (actual == expected)
        .then_some(())
        .ok_or_else(|| return format!("{label} archive must be named {expected}, got {actual}"));
}

/// Require a complete nonempty MIT license file.
///
/// # Errors
///
/// Returns an error when the license is not UTF-8 or lacks the MIT grant.
pub(super) fn require_license(contents: &[u8], label: &str) -> CheckResult {
    let text = check_try!(
        from_utf8(contents)
            .map_err(|error| return format!("{label} LICENSE is not UTF-8: {error}"))
    );
    for required in [
        "MIT License",
        "Permission is hereby granted",
        "THE SOFTWARE IS PROVIDED",
    ] {
        if !text.contains(required) {
            return Err(format!("{label} LICENSE is missing {required}"));
        }
    }
    return Ok(());
}

/// Require Python core metadata name, version, and license fields.
///
/// # Errors
///
/// Returns an error when any canonical metadata field is missing or differs.
pub(super) fn require_metadata(contents: &[u8], version: &str, label: &str) -> CheckResult {
    let text = check_try!(
        from_utf8(contents)
            .map_err(|error| return format!("{label} metadata is not UTF-8: {error}"))
    );
    for (field, expected) in [
        ("Name", "tovuk"),
        ("Version", version),
        ("License-Expression", LICENSE_IDENTIFIER),
    ] {
        check_try!(require_wheel_metadata_field(text, field, expected, label));
    }
    return Ok(());
}

/// Require a nonempty native target JSON manifest.
///
/// # Errors
///
/// Returns an error when the document is invalid or has no target entries.
pub(super) fn require_native_targets(contents: &[u8], label: &str) -> CheckResult {
    let value = check_try!(
        from_slice::<Value>(contents).map_err(|error| return format!("parse {label}: {error}"))
    );
    let targets = check_try!(
        value
            .get("targets")
            .and_then(Value::as_array)
            .ok_or_else(|| return format!("{label} must contain a targets array"))
    );
    return (!targets.is_empty())
        .then_some(())
        .ok_or_else(|| return format!("{label} targets array must not be empty"));
}

/// Require Python project identity fields from the packaged `pyproject.toml`.
///
/// # Errors
///
/// Returns an error when a project field is absent or differs.
pub(super) fn require_python_project(contents: &[u8], version: &str, label: &str) -> CheckResult {
    let text = check_try!(
        from_utf8(contents).map_err(|error| return format!("{label} is not UTF-8: {error}"))
    );
    for (key, expected) in [
        ("name", "tovuk"),
        ("version", version),
        ("license", LICENSE_IDENTIFIER),
    ] {
        let actual = check_try!(toml_table_value(text, "project", key, label));
        if actual != expected {
            return Err(format!(
                "{label} project {key} must be {expected}, got {actual}"
            ));
        }
    }
    return Ok(());
}

/// Require the Python runtime version constant to match release metadata.
///
/// # Errors
///
/// Returns an error when the source is not UTF-8 or lacks the exact assignment.
pub(super) fn require_python_version(contents: &[u8], version: &str, label: &str) -> CheckResult {
    let text = check_try!(
        from_utf8(contents).map_err(|error| return format!("{label} is not UTF-8: {error}"))
    );
    let assignment = format!("__version__ = \"{version}\"");
    return text
        .lines()
        .any(|line| return line.trim() == assignment)
        .then_some(())
        .ok_or_else(|| return format!("{label} must define {assignment}"));
}

/// Require Cargo package identity fields from the normalized manifest.
///
/// # Errors
///
/// Returns an error when a package field is absent or differs.
pub(super) fn require_toml_package(contents: &[u8], version: &str, label: &str) -> CheckResult {
    let text = check_try!(
        from_utf8(contents).map_err(|error| return format!("{label} is not UTF-8: {error}"))
    );
    for (key, expected) in [
        ("name", "tovuk"),
        ("version", version),
        ("license", LICENSE_IDENTIFIER),
    ] {
        let actual = check_try!(toml_table_value(text, "package", key, label));
        if actual != expected {
            return Err(format!(
                "{label} package {key} must be {expected}, got {actual}"
            ));
        }
    }
    return Ok(());
}

/// Require a canonical three-component numeric package version.
///
/// # Errors
///
/// Returns an error when the version is empty, noncanonical, or contains a
/// leading zero.
pub(super) fn require_version(version: &str) -> CheckResult {
    if version.is_empty() || version.len() > 0x40 {
        return Err("package version must contain between 1 and 64 bytes".to_owned());
    }
    let components = version.split('.').collect::<Vec<_>>();
    if components.len() != 0x3 {
        return Err("package version must have exactly three numeric components".to_owned());
    }
    for component in components {
        if component.is_empty()
            || !component.bytes().all(|byte| return byte.is_ascii_digit())
            || (component.len() > 0x1 && component.starts_with('0'))
        {
            return Err(format!(
                "package version component {component:?} is not canonical"
            ));
        }
    }
    return Ok(());
}

/// Require the project-specific pure-Python wheel filename.
///
/// # Errors
///
/// Returns an error when the wheel filename differs from the release contract.
pub(super) fn require_wheel_file_name(path: &Path, version: &str) -> CheckResult {
    return require_file_name(
        path,
        format!("tovuk-{version}-py3-none-any.whl").as_str(),
        "Python wheel",
    );
}

/// Require one unfolded Python core metadata header.
///
/// # Errors
///
/// Returns an error when the field is absent or differs.
fn require_wheel_metadata_field(
    text: &str,
    field: &str,
    expected: &str,
    label: &str,
) -> CheckResult {
    let prefix = format!("{field}:");
    let actual = text
        .lines()
        .take_while(|line| return !line.is_empty())
        .find_map(|line| {
            return line
                .strip_prefix(prefix.as_str())
                .map(|value| return value.trim());
        });
    return match actual {
        Some(value) if value == expected => Ok(()),
        Some(value) => Err(format!(
            "{label} metadata {field} must be {expected}, got {value}"
        )),
        None => Err(format!("{label} metadata is missing {field}")),
    };
}

/// Extract one quoted scalar from a named TOML table.
///
/// # Errors
///
/// Returns an error when the table field is absent or not a quoted string.
fn toml_table_value<'text>(
    text: &'text str,
    table: &str,
    key: &str,
    label: &str,
) -> CheckResult<&'text str> {
    let prefix = format!("{key} = ");
    let heading = format!("[{table}]");
    let mut in_table = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_table = line == heading;
            continue;
        }
        if !in_table {
            continue;
        }
        let Some(raw) = line.strip_prefix(prefix.as_str()) else {
            continue;
        };
        let quoted = raw
            .strip_prefix('"')
            .and_then(|value| return value.strip_suffix('"'));
        return quoted.ok_or_else(|| {
            return format!("{label} {table} {key} must be a quoted string");
        });
    }
    return Err(format!("{label} is missing {table} {key}"));
}
