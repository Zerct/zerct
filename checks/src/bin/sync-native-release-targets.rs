//! Sync generated native release target manifests.

/// Propagate a failed target sync without the question-mark operator.
macro_rules! check_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => return Err(error.into()),
        }
    };
}

use flate2 as _;

use http as _;

use http_body_util as _;

use hyper as _;

use hyper_rustls as _;

use hyper_util as _;

use rustls as _;

use tokio as _;

use url as _;

use serde as _;

use serde_json as _;

use sha2 as _;

use std::{
    env::args,
    fs::{create_dir_all, read, write},
    io::{Write as _, stderr},
    path::Path,
    process::ExitCode,
};

use tar as _;

use tovuk_public_checks::check_support::{CheckResult, repo_root};

/// Contract value named `GENERATED_MANIFESTS`.
const GENERATED_MANIFESTS: &[&str] = &[
    "packages/tovuk/native-release-targets.json",
    "packages/tovuk-py/src/tovuk/native_release_targets.json",
];

/// Contract value named `SOURCE_MANIFEST`.
const SOURCE_MANIFEST: &str = "native-release-targets.json";

/// Contract value named `SYNC_COMMAND`.
const SYNC_COMMAND: &str = "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin sync-native-release-targets --";

const _: [usize; 0x4] = [
    size_of_val(&generated_matches),
    size_of_val(&mode),
    size_of_val(&run),
    size_of_val(&sync_manifest),
];

#[derive(Clone, Copy, Debug)]
/// Contract representation for `Mode`.
enum Mode {
    /// The `Check` contract variant.
    Check,
    /// The `Sync` contract variant.
    Sync,
}

/// Contract implementation for `generated_matches`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn generated_matches(generated_path: &Path, source_bytes: &[u8]) -> CheckResult<bool> {
    let exists = check_try!(
        generated_path
            .try_exists()
            .map_err(|error| format!("inspect {}: {error}", generated_path.display()))
    );
    if !exists {
        return Ok(false);
    }
    return read(generated_path)
        .map(|generated_bytes| return generated_bytes == source_bytes)
        .map_err(|error| format!("read {}: {error}", generated_path.display()));
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => return ExitCode::SUCCESS,
        Err(error) => {
            let _write_result = writeln!(stderr().lock(), "{error}");
            return ExitCode::FAILURE;
        }
    }
}

/// Contract implementation for `mode`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn mode() -> CheckResult<Mode> {
    let arguments = args().skip(0x1).collect::<Vec<_>>();
    if arguments.len() > 0x1 {
        return Err("usage: sync-native-release-targets [sync|--check]".to_owned());
    }
    match arguments.first().map(String::as_str) {
        None | Some("sync") => return Ok(Mode::Sync),
        Some("--check" | "check") => return Ok(Mode::Check),
        Some(_) => return Err("usage: sync-native-release-targets [sync|--check]".to_owned()),
    }
}

/// Contract implementation for `run`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn run() -> CheckResult {
    let mode = check_try!(mode());
    let repo_root = check_try!(repo_root());
    let source_bytes = check_try!(
        read(repo_root.join(SOURCE_MANIFEST))
            .map_err(|error| format!("read {SOURCE_MANIFEST}: {error}"))
    );

    for generated_manifest in GENERATED_MANIFESTS {
        check_try!(sync_manifest(
            repo_root.as_path(),
            source_bytes.as_slice(),
            generated_manifest,
            mode,
        ));
    }
    return Ok(());
}

/// Contract implementation for `sync_manifest`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn sync_manifest(
    repo_root: &Path,
    source_bytes: &[u8],
    generated_manifest: &str,
    mode: Mode,
) -> CheckResult {
    let generated_path = repo_root.join(generated_manifest);
    if check_try!(generated_matches(generated_path.as_path(), source_bytes)) {
        return Ok(());
    }

    match mode {
        Mode::Check => return Err(format!("{generated_manifest} is stale; run {SYNC_COMMAND}")),
        Mode::Sync => {
            let parent = check_try!(
                generated_path
                    .parent()
                    .ok_or_else(|| format!("{generated_manifest} must have a parent directory"))
            );
            check_try!(
                create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))
            );
            return write(generated_path.as_path(), source_bytes)
                .map_err(|error| format!("write {generated_manifest}: {error}"));
        }
    }
}
