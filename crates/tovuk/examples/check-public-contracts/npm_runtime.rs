use std::{path::Path, process::Command};

use crate::helpers::{CheckResult, file_exists, must_abs};

pub(crate) fn check_native_runtime_contract() -> CheckResult {
    let repo_root = must_abs(".")?;
    let binary = std::env::var("TOVUK_NATIVE_BINARY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| native_binary_candidate(repo_root.as_str()));

    let Some(binary) = binary else {
        return Err("native Tovuk binary does not exist: ".to_owned());
    };
    if !file_exists(binary.as_str()) {
        return Err(format!("native Tovuk binary does not exist: {binary}"));
    }

    let status = Command::new(binary.as_str())
        .arg("--version")
        .status()
        .map_err(|error| format!("native Tovuk binary failed: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("native Tovuk binary failed with {status}"))
    }
}

fn native_binary_candidate(repo_root: &str) -> Option<String> {
    [
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
    .find(|candidate| file_exists(candidate))
    .map(|candidate| candidate.display().to_string())
}
