//! Sync generated native release target manifests.

use std::{env, fs, io, path::Path, process::ExitCode};

use tovuk_public_checks::check_support::{CheckResult, repo_root};

const SOURCE_MANIFEST: &str = "native-release-targets.json";
const GENERATED_MANIFESTS: &[&str] = &[
    "packages/tovuk/native-release-targets.json",
    "packages/tovuk-py/src/tovuk/native_release_targets.json",
];
const SYNC_COMMAND: &str = "cargo run --locked --quiet --manifest-path checks/Cargo.toml --bin sync-native-release-targets --";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> CheckResult {
    let mode = mode()?;
    let repo_root = repo_root()?;
    let source_bytes = fs::read(repo_root.join(SOURCE_MANIFEST))
        .map_err(|error| format!("read {SOURCE_MANIFEST}: {error}"))?;

    for generated_manifest in GENERATED_MANIFESTS {
        sync_manifest(
            repo_root.as_path(),
            source_bytes.as_slice(),
            generated_manifest,
            mode,
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum Mode {
    Sync,
    Check,
}

fn mode() -> CheckResult<Mode> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 1 {
        return Err("usage: sync-native-release-targets [sync|--check]".to_owned());
    }
    match args.first().map(String::as_str) {
        None | Some("sync") => Ok(Mode::Sync),
        Some("--check" | "check") => Ok(Mode::Check),
        Some(_) => Err("usage: sync-native-release-targets [sync|--check]".to_owned()),
    }
}

fn sync_manifest(
    repo_root: &Path,
    source_bytes: &[u8],
    generated_manifest: &str,
    mode: Mode,
) -> CheckResult {
    let generated_path = repo_root.join(generated_manifest);
    if generated_matches(generated_path.as_path(), source_bytes)? {
        return Ok(());
    }

    match mode {
        Mode::Check => Err(format!("{generated_manifest} is stale; run {SYNC_COMMAND}")),
        Mode::Sync => {
            let parent = generated_path
                .parent()
                .ok_or_else(|| format!("{generated_manifest} must have a parent directory"))?;
            fs::create_dir_all(parent)
                .map_err(|error| format!("create {}: {error}", parent.display()))?;
            fs::write(generated_path.as_path(), source_bytes)
                .map_err(|error| format!("write {generated_manifest}: {error}"))
        }
    }
}

fn generated_matches(generated_path: &Path, source_bytes: &[u8]) -> CheckResult<bool> {
    match fs::read(generated_path) {
        Ok(generated_bytes) => Ok(generated_bytes == source_bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("read {}: {error}", generated_path.display())),
    }
}
