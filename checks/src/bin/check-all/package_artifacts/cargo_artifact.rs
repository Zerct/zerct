//! Isolated build and installation proof for the publishable Cargo archive.

use flate2::read::GzDecoder;

use std::{
    env,
    fs::{File, create_dir_all, symlink_metadata, write},
    path::{Path, PathBuf},
};

use tar::{Archive as TarArchive, EntryType as TarEntryType};

use tovuk_public_checks::check_support::CheckResult;
use tovuk_public_checks::check_try;

use super::{ArtifactPaths, Runner, command_output, path_string, verify_version_output};

/// Exact repository Rust toolchain selector used outside the worktree.
const RUST_TOOLCHAIN_ARGUMENT: &str = "+1.97.0";

/// Compile-time references preserve the standalone Cargo gate boundaries.
const _: [usize; 0x000d] = [
    size_of_val(&cargo_binary),
    size_of_val(&cargo_check),
    size_of_val(&cargo_install),
    size_of_val(&cargo_status),
    size_of_val(&create_sandbox),
    size_of_val(&escape_toml_path),
    size_of_val(&extract_crate),
    size_of_val(&filesystem_root),
    size_of_val(&require_extractable_member),
    size_of_val(&run),
    size_of_val(&run_installed),
    size_of_val(&vendor_dependencies),
    size_of_val(&write_vendor_config),
];

/// Paths participating in one isolated Cargo package proof.
#[derive(Debug)]
struct CargoSandbox {
    /// Empty Cargo home containing only the generated vendored-source policy.
    home: PathBuf,
    /// Isolated binary installation prefix.
    install: PathBuf,
    /// Exact Cargo configuration published in the crate.
    package_config: PathBuf,
    /// External working directory preventing repository config discovery.
    process_directory: PathBuf,
    /// Safely extracted package source root.
    source: PathBuf,
    /// Disposable compilation target directory.
    target: PathBuf,
    /// Offline registry source tree generated from the locked package graph.
    vendor: PathBuf,
}

/// Return the installed platform-specific `tovuk` executable path.
fn cargo_binary(sandbox: &CargoSandbox) -> PathBuf {
    return if cfg!(windows) {
        sandbox.install.join("bin/tovuk.exe")
    } else {
        sandbox.install.join("bin/tovuk")
    };
}

/// Compile the extracted package in locked, offline release mode.
///
/// # Errors
///
/// Returns an error when the standalone package cannot compile.
fn cargo_check(runner: &Runner, sandbox: &CargoSandbox) -> CheckResult {
    let config = check_try!(path_string(sandbox.package_config.as_path()));
    let manifest = check_try!(path_string(sandbox.source.join("Cargo.toml").as_path()));
    return cargo_status(
        runner,
        sandbox,
        &[
            RUST_TOOLCHAIN_ARGUMENT,
            "--config",
            config.as_str(),
            "check",
            "--locked",
            "--offline",
            "--release",
            "--manifest-path",
            manifest.as_str(),
        ],
    );
}

/// Install the extracted package into an isolated prefix without a registry.
///
/// # Errors
///
/// Returns an error when the locked offline installation fails.
fn cargo_install(runner: &Runner, sandbox: &CargoSandbox) -> CheckResult {
    let config = check_try!(path_string(sandbox.package_config.as_path()));
    let install = check_try!(path_string(sandbox.install.as_path()));
    let source = check_try!(path_string(sandbox.source.as_path()));
    let target = check_try!(path_string(sandbox.target.as_path()));
    return cargo_status(
        runner,
        sandbox,
        &[
            RUST_TOOLCHAIN_ARGUMENT,
            "--config",
            config.as_str(),
            "install",
            "--quiet",
            "--locked",
            "--offline",
            "--path",
            source.as_str(),
            "--root",
            install.as_str(),
            "--target-dir",
            target.as_str(),
        ],
    );
}

/// Run Cargo with isolated configuration, target state, and network disabled.
///
/// # Errors
///
/// Returns an error when Cargo cannot start or exits unsuccessfully.
fn cargo_status(runner: &Runner, sandbox: &CargoSandbox, args: &[&str]) -> CheckResult {
    let status = check_try!(
        runner
            .command(sandbox.process_directory.as_path(), "cargo", args)
            .env("CARGO_HOME", sandbox.home.as_os_str())
            .env("CARGO_NET_OFFLINE", "true")
            .env("CARGO_TARGET_DIR", sandbox.target.as_os_str())
            .env_remove("CARGO_BUILD_TARGET")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .env_remove("RUSTC_WRAPPER")
            .env_remove("RUSTFLAGS")
            .status()
            .map_err(|error| return format!("run isolated Cargo: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("isolated Cargo failed with status {status}"));
}

/// Prepare a safe extraction root and all isolated Cargo paths.
///
/// # Errors
///
/// Returns an error when extraction, directory creation, or config writing fails.
fn create_sandbox(runner: &Runner, paths: &ArtifactPaths) -> CheckResult<CargoSandbox> {
    let extraction = paths.root.join("cargo-extracted");
    check_try!(create_dir_all(extraction.as_path()).map_err(|error| {
        return format!(
            "create Cargo extraction root {}: {error}",
            extraction.display()
        );
    }));
    check_try!(extract_crate(paths, extraction.as_path()));
    let source = extraction.join(format!("tovuk-{}", paths.version));
    let metadata = check_try!(symlink_metadata(source.as_path()).map_err(|error| {
        return format!("inspect extracted Cargo root {}: {error}", source.display());
    }));
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("extracted Cargo package root must be a real directory".to_owned());
    }
    let sandbox = CargoSandbox {
        home: paths.root.join("cargo-home"),
        install: paths.root.join("cargo-install"),
        package_config: source.join(".cargo/config.toml"),
        process_directory: check_try!(filesystem_root(runner)),
        source,
        target: paths.root.join("cargo-target"),
        vendor: paths.root.join("cargo-vendor"),
    };
    check_try!(create_dir_all(sandbox.home.as_path()).map_err(|error| {
        return format!(
            "create isolated Cargo home {}: {error}",
            sandbox.home.display()
        );
    }));
    check_try!(write_vendor_config(&sandbox));
    return Ok(sandbox);
}

/// Encode an absolute filesystem path as a TOML basic-string value.
///
/// # Errors
///
/// Returns an error when the path is not UTF-8 or contains control characters.
fn escape_toml_path(path: &Path) -> CheckResult<String> {
    let value = check_try!(path_string(path));
    if value.chars().any(char::is_control) {
        return Err(format!(
            "Cargo vendor path {} contains control characters",
            path.display()
        ));
    }
    return Ok(value.replace('\\', "\\\\").replace('"', "\\\""));
}

/// Extract regular crate members with `tar` traversal protection enabled.
///
/// # Errors
///
/// Returns an error when the archive is unreadable, unsafe, or cannot extract.
fn extract_crate(paths: &ArtifactPaths, destination: &Path) -> CheckResult {
    let archive_path = paths.cargo_archive.as_path();
    let file = check_try!(File::open(archive_path).map_err(|error| {
        return format!("open Cargo archive {}: {error}", archive_path.display());
    }));
    let mut archive = TarArchive::new(GzDecoder::new(file));
    let entries = check_try!(archive.entries().map_err(|error| {
        return format!("read Cargo archive {}: {error}", archive_path.display());
    }));
    let expected_root = PathBuf::from(format!("tovuk-{}", paths.version));
    for entry_result in entries {
        let mut entry = check_try!(entry_result.map_err(|error| {
            return format!("read Cargo archive entry: {error}");
        }));
        let entry_type = entry.header().entry_type();
        let member = check_try!(
            entry
                .path()
                .map(|path| return path.into_owned())
                .map_err(|error| return format!("read Cargo archive member path: {error}"))
        );
        check_try!(require_extractable_member(
            entry_type,
            member.as_path(),
            expected_root.as_path(),
        ));
        let unpacked = check_try!(entry.unpack_in(destination).map_err(|error| {
            return format!("extract Cargo archive member {}: {error}", member.display());
        }));
        if !unpacked {
            return Err(format!(
                "Cargo refused unsafe archive member {}",
                member.display()
            ));
        }
    }
    return Ok(());
}

/// Return a working directory outside the repository config hierarchy.
///
/// # Errors
///
/// Returns an error when no filesystem root exists or it is inside the repo.
fn filesystem_root(runner: &Runner) -> CheckResult<PathBuf> {
    let root = check_try!(
        env::temp_dir()
            .ancestors()
            .last()
            .map(Path::to_path_buf)
            .ok_or_else(
                || return "could not identify an external Cargo working directory".to_owned(),
            )
    );
    if root.starts_with(runner.repo_root.as_path()) {
        return Err("isolated Cargo working directory must be outside the repository".to_owned());
    }
    for relative in [".cargo/config", ".cargo/config.toml"] {
        let candidate = root.join(relative);
        if check_try!(candidate.try_exists().map_err(|error| {
            return format!(
                "inspect external Cargo config {}: {error}",
                candidate.display()
            );
        })) {
            return Err(format!("external Cargo working root contains {relative}"));
        }
    }
    return Ok(root);
}

/// Require one regular archive member below the exact package root.
///
/// # Errors
///
/// Returns an error when a member is non-regular or has another root.
fn require_extractable_member(
    entry_type: TarEntryType,
    member: &Path,
    expected_root: &Path,
) -> CheckResult {
    if !(entry_type.is_file() || entry_type.is_dir()) {
        return Err("Cargo archive extraction accepts only files and directories".to_owned());
    }
    return member
        .starts_with(expected_root)
        .then_some(())
        .ok_or_else(|| {
            return format!(
                "Cargo archive member {} has an unexpected root",
                member.display()
            );
        });
}

/// Prove the package checks, installs, and reports its version in isolation.
///
/// # Errors
///
/// Returns the first extraction, vendoring, build, install, or runtime failure.
pub(super) fn run(runner: &Runner, paths: &ArtifactPaths) -> CheckResult {
    let sandbox = check_try!(create_sandbox(runner, paths));
    check_try!(vendor_dependencies(runner, &sandbox));
    check_try!(cargo_check(runner, &sandbox));
    check_try!(cargo_install(runner, &sandbox));
    return run_installed(runner, paths, &sandbox);
}

/// Run the independently installed CLI and require the package version.
///
/// # Errors
///
/// Returns an error when the installed binary fails or reports version drift.
fn run_installed(runner: &Runner, paths: &ArtifactPaths, sandbox: &CargoSandbox) -> CheckResult {
    let binary = check_try!(path_string(cargo_binary(sandbox).as_path()));
    let actual = check_try!(command_output(
        runner,
        sandbox.process_directory.as_path(),
        binary.as_str(),
        &["--version"],
    ));
    return verify_version_output(actual.as_str(), paths);
}

/// Materialize the registry-backed lock graph from the existing Cargo cache.
///
/// # Errors
///
/// Returns an error when a dependency is absent from cache or vendoring fails.
fn vendor_dependencies(runner: &Runner, sandbox: &CargoSandbox) -> CheckResult {
    let config = check_try!(path_string(sandbox.package_config.as_path()));
    let manifest = check_try!(path_string(sandbox.source.join("Cargo.toml").as_path()));
    let vendor = check_try!(path_string(sandbox.vendor.as_path()));
    let args = [
        RUST_TOOLCHAIN_ARGUMENT,
        "--config",
        config.as_str(),
        "vendor",
        "--quiet",
        "--locked",
        "--offline",
        "--versioned-dirs",
        "--manifest-path",
        manifest.as_str(),
        vendor.as_str(),
    ];
    let status = check_try!(
        runner
            .command(sandbox.process_directory.as_path(), "cargo", &args)
            .env("CARGO_NET_OFFLINE", "true")
            .env_remove("CARGO_TARGET_DIR")
            .status()
            .map_err(|error| return format!("vendor standalone Cargo dependencies: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("offline Cargo vendoring failed with status {status}"));
}

/// Write the only non-package Cargo policy used by isolated compilation.
///
/// # Errors
///
/// Returns an error when the vendor path cannot be encoded or written.
fn write_vendor_config(sandbox: &CargoSandbox) -> CheckResult {
    let vendor = check_try!(escape_toml_path(sandbox.vendor.as_path()));
    let config = format!(
        "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n[source.vendored-sources]\ndirectory = \"{vendor}\"\n"
    );
    let path = sandbox.home.join("config.toml");
    return write(path.as_path(), config).map_err(|error| {
        return format!("write isolated Cargo config {}: {error}", path.display());
    });
}
