use crate::helpers::{CheckResult, file_exists, must_abs};

use std::{env::var, path::Path, process::Command};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&check_native_runtime_contract),
    size_of_val(&native_binary_candidate),
];

/// Contract implementation for `check_native_runtime_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_native_runtime_contract() -> CheckResult {
    let repo_root = check_try!(must_abs("."));
    let binary = var("TOVUK_NATIVE_BINARY")
        .ok()
        .filter(|value| return !value.trim().is_empty())
        .or_else(|| return native_binary_candidate(repo_root.as_str()));

    let Some(resolved_binary) = binary else {
        return Err("native Tovuk binary does not exist: ".to_owned());
    };
    if !file_exists(resolved_binary.as_str()) {
        return Err(format!(
            "native Tovuk binary does not exist: {resolved_binary}"
        ));
    }

    let status = check_try!(
        Command::new(resolved_binary.as_str())
            .arg("--version")
            .status()
            .map_err(|error| format!("native Tovuk binary failed: {error}"))
    );
    if status.success() {
        return Ok(());
    }
    return Err(format!("native Tovuk binary failed with {status}"));
}

/// Contract implementation for `native_binary_candidate`.
pub(super) fn native_binary_candidate(repo_root: &str) -> Option<String> {
    return [
        Path::new(repo_root)
            .join("crates")
            .join("tovuk")
            .join("target")
            .join("release")
            .join("tovuk"),
        Path::new(repo_root)
            .join("packages")
            .join("tovuk")
            .join("bin")
            .join("tovuk-native"),
    ]
    .into_iter()
    .find(|candidate| return file_exists(candidate))
    .map(|candidate| return candidate.display().to_string());
}
