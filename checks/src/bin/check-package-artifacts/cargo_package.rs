//! Cargo `.crate` package artifact policy.

use core::str::from_utf8;

use serde_json::{Value, from_slice};

use std::path::Path;

use tovuk_public_checks::check_support::{CheckResult, command, repo_root, tool_path};

use super::{
    LicenseEvidence,
    archive::{PackageArchive, read_tar_gz},
    policy::{require_file_name, require_license, require_toml_package},
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&current_git_head),
    size_of_val(&is_allowed_file),
    size_of_val(&require_manifests),
    size_of_val(&require_source_files),
    size_of_val(&require_vcs_info),
    size_of_val(&validate),
];

/// Read and validate the current release commit identifier.
///
/// # Errors
///
/// Returns an error when Git fails or its commit identifier is malformed.
fn current_git_head() -> CheckResult<String> {
    let repository = check_try!(repo_root());
    let output = check_try!(
        command(repository.as_path(), tool_path().as_os_str(), "git")
            .args(["rev-parse", "HEAD"])
            .output()
            .map_err(|error| return format!("run git rev-parse HEAD: {error}"))
    );
    if !output.status.success() {
        return Err(format!(
            "git rev-parse HEAD failed with status {}",
            output.status
        ));
    }
    let head = check_try!(
        from_utf8(output.stdout.as_slice())
            .map_err(|error| return format!("git HEAD is not UTF-8: {error}"))
    )
    .trim();
    if head.len() != 0x28
        || !head
            .bytes()
            .all(|byte| return byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("git HEAD must be 40 lowercase hexadecimal characters".to_owned());
    }
    return Ok(head.to_owned());
}

/// Return whether a Cargo package member belongs to the public source package.
fn is_allowed_file(relative_path: &str) -> bool {
    return matches!(
        relative_path,
        ".cargo_vcs_info.json"
            | "Cargo.lock"
            | "Cargo.toml"
            | "Cargo.toml.orig"
            | "LICENSE"
            | "README.md"
    ) || (relative_path.starts_with("src/")
        && Path::new(relative_path)
            .extension()
            .is_some_and(|extension| return extension.eq_ignore_ascii_case("rs")));
}

/// Require the normalized and original Cargo package identity metadata.
///
/// # Errors
///
/// Returns an error when a manifest is missing or its identity differs.
fn require_manifests(archive: &PackageArchive, root: &str, version: &str) -> CheckResult {
    for relative in ["Cargo.toml", "Cargo.toml.orig"] {
        let path = format!("{root}/{relative}");
        check_try!(require_toml_package(
            check_try!(archive.file(path.as_str(), "Cargo")),
            version,
            relative,
        ));
    }
    return Ok(());
}

/// Require the expected Cargo-generated files and public Rust sources only.
///
/// # Errors
///
/// Returns an error when a required file is absent or an extra file exists.
fn require_source_files(archive: &PackageArchive, root: &str) -> CheckResult {
    for relative in [
        ".cargo_vcs_info.json",
        "Cargo.lock",
        "Cargo.toml",
        "Cargo.toml.orig",
        "LICENSE",
        "README.md",
        "src/main.rs",
    ] {
        let member = format!("{root}/{relative}");
        if check_try!(archive.file(member.as_str(), "Cargo")).is_empty() {
            return Err(format!("Cargo archive member {member} must not be empty"));
        }
    }
    let prefix = format!("{root}/");
    for member in archive.files().keys() {
        let relative = check_try!(
            member
                .strip_prefix(prefix.as_str())
                .ok_or_else(|| return format!("Cargo member {member} is outside {root}"))
        );
        if !is_allowed_file(relative) {
            return Err(format!("Cargo archive contains unexpected file {member}"));
        }
    }
    return Ok(());
}

/// Require clean Cargo VCS metadata bound to the current release commit.
///
/// # Errors
///
/// Returns an error when metadata is malformed, dirty, or names another commit.
fn require_vcs_info(contents: &[u8]) -> CheckResult {
    let value = check_try!(
        from_slice::<Value>(contents)
            .map_err(|error| return format!("parse Cargo .cargo_vcs_info.json: {error}"))
    );
    let root = check_try!(
        value
            .as_object()
            .filter(|object| return object.len() == 0x2)
            .ok_or_else(
                || return "Cargo VCS metadata must contain only git and path_in_vcs".to_owned()
            )
    );
    if root.get("path_in_vcs").and_then(Value::as_str) != Some("crates/tovuk") {
        return Err("Cargo VCS path_in_vcs must be crates/tovuk".to_owned());
    }
    let git = check_try!(
        root.get("git")
            .and_then(Value::as_object)
            .filter(|object| return object.len() == 0x1)
            .ok_or_else(|| return "Cargo VCS git metadata must contain only sha1".to_owned())
    );
    let sha = check_try!(
        git.get("sha1")
            .and_then(Value::as_str)
            .ok_or_else(|| return "Cargo VCS sha1 must be a string".to_owned())
    );
    let head = check_try!(current_git_head());
    return (sha == head).then_some(()).ok_or_else(|| {
        return format!("Cargo VCS sha1 {sha} does not match release commit {head}");
    });
}

/// Validate one Cargo package artifact.
///
/// # Errors
///
/// Returns an error when the archive name, root, source set, metadata, or
/// license differs from the public Cargo package contract.
pub(super) fn validate(path: &Path, version: &str) -> CheckResult<LicenseEvidence> {
    check_try!(require_file_name(
        path,
        format!("tovuk-{version}.crate").as_str(),
        "Cargo",
    ));
    let archive = check_try!(read_tar_gz(path, "Cargo"));
    let root = format!("tovuk-{version}");
    check_try!(archive.require_root(root.as_str(), "Cargo"));
    check_try!(require_source_files(&archive, root.as_str()));
    check_try!(require_manifests(&archive, root.as_str(), version));
    let vcs_path = format!("{root}/.cargo_vcs_info.json");
    check_try!(require_vcs_info(check_try!(
        archive.file(vcs_path.as_str(), "Cargo")
    )));
    let license_path = format!("{root}/LICENSE");
    let license = check_try!(archive.file(license_path.as_str(), "Cargo"));
    check_try!(require_license(license, "Cargo"));
    return Ok(LicenseEvidence {
        license: license.to_vec(),
    });
}
