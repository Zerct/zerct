//! Build and verify exact public package artifacts in an ignored work area.

use std::{
    fs::{create_dir_all, remove_dir_all, remove_file, symlink_metadata},
    path::{Path, PathBuf},
};

use tovuk_public_checks::check_support::CheckResult;
use tovuk_public_checks::check_try;

use super::{PackageArtifactRunner, Runner};

/// Compile-time references preserve the named artifact-gate boundaries.
const _: [usize; 0x000e] = [
    size_of_val(&artifact_paths),
    size_of_val(&build_npm_package),
    size_of_val(&build_python_packages),
    size_of_val(&clear_path),
    size_of_val(&command_output),
    size_of_val(&create_artifact_root),
    size_of_val(&path_string),
    size_of_val(&public_version),
    size_of_val(&python_environment_executable),
    size_of_val(&run),
    size_of_val(&run_artifact_checker),
    size_of_val(&smoke_test_npm),
    size_of_val(&smoke_test_python),
    size_of_val(&verify_version_output),
];

/// Canonical paths for one synchronized package artifact set.
#[derive(Debug)]
struct ArtifactPaths {
    /// Cargo package archive.
    cargo_archive: PathBuf,
    /// npm package archive.
    npm_archive: PathBuf,
    /// Isolated artifact and smoke-test root.
    root: PathBuf,
    /// Python source distribution.
    sdist: PathBuf,
    /// Canonical synchronized package version.
    version: String,
    /// Python wheel.
    wheel: PathBuf,
}

impl PackageArtifactRunner for Runner {
    fn run_package_artifacts(&self) -> CheckResult {
        return run(self);
    }
}

/// Derive exact artifact paths from the synchronized public version.
fn artifact_paths(runner: &Runner, version: &str) -> ArtifactPaths {
    let root = runner
        .repo_root
        .join("checks")
        .join("target")
        .join("package-artifacts");
    return ArtifactPaths {
        cargo_archive: runner
            .repo_root
            .join("crates")
            .join("tovuk")
            .join("target")
            .join("package")
            .join(format!("tovuk-{version}.crate")),
        npm_archive: root.join(format!("tovuk-{version}.tgz")),
        root: root.clone(),
        sdist: root.join(format!("tovuk-{version}.tar.gz")),
        version: version.to_owned(),
        wheel: root.join(format!("tovuk-{version}-py3-none-any.whl")),
    };
}

/// Build the exact npm tarball into the isolated artifact directory.
///
/// # Errors
///
/// Returns an error when npm cannot create the exact package archive.
fn build_npm_package(runner: &Runner, paths: &ArtifactPaths) -> CheckResult {
    let artifact_root = check_try!(path_string(paths.root.as_path()));
    return runner.status_in(
        runner.repo_root.join("packages/tovuk").as_path(),
        "npm",
        &["pack", "--pack-destination", artifact_root.as_str()],
    );
}

/// Build the wheel from the sdist using pinned public Python tooling.
///
/// # Errors
///
/// Returns an error when the pinned build frontend cannot create both archives.
fn build_python_packages(runner: &Runner, paths: &ArtifactPaths) -> CheckResult {
    let artifact_root = check_try!(path_string(paths.root.as_path()));
    let status = check_try!(
        runner
            .command(
                runner.repo_root.as_path(),
                "uvx",
                &[
                    "--no-config",
                    "--default-index",
                    "https://pypi.org/simple",
                    "--from",
                    "build==1.5.1",
                    "pyproject-build",
                    "--installer",
                    "uv",
                    "--outdir",
                    artifact_root.as_str(),
                    "packages/tovuk-py",
                ],
            )
            .status()
            .map_err(|error| return format!("run pinned Python package build: {error}"))
    );
    return status
        .success()
        .then_some(())
        .ok_or_else(|| return format!("Python package build failed with status {status}"));
}

/// Remove a prior artifact location without following a local symlink.
///
/// # Errors
///
/// Returns an error when the path cannot be inspected or safely removed.
fn clear_path(path: &Path) -> CheckResult {
    let exists = check_try!(
        path.try_exists()
            .map_err(|error| return format!("inspect {}: {error}", path.display()))
    );
    if !exists {
        return Ok(());
    }
    let metadata = check_try!(
        symlink_metadata(path)
            .map_err(|error| return format!("inspect {}: {error}", path.display()))
    );
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        return remove_dir_all(path)
            .map_err(|error| return format!("remove {}: {error}", path.display()));
    }
    return remove_file(path).map_err(|error| return format!("remove {}: {error}", path.display()));
}

/// Run a command and return its one-line UTF-8 standard output.
///
/// # Errors
///
/// Returns an error when the command fails or its output is not UTF-8.
fn command_output(
    runner: &Runner,
    cwd: &Path,
    program: &str,
    args: &[&str],
) -> CheckResult<String> {
    let output = check_try!(
        runner
            .command(cwd, program, args)
            .output()
            .map_err(|error| return format!("run {program}: {error}"))
    );
    if !output.status.success() {
        return Err(format!("{program} failed with status {}", output.status));
    }
    return String::from_utf8(output.stdout)
        .map(|value| return value.trim().to_owned())
        .map_err(|error| return format!("{program} output must be UTF-8: {error}"));
}

/// Prepare one empty ignored artifact root.
///
/// # Errors
///
/// Returns an error when the prior root cannot be cleared or recreated.
fn create_artifact_root(paths: &ArtifactPaths) -> CheckResult {
    check_try!(clear_path(paths.root.as_path()));
    return create_dir_all(paths.root.as_path())
        .map_err(|error| return format!("create {}: {error}", paths.root.display()));
}

/// Return one path as a strict UTF-8 command argument.
///
/// # Errors
///
/// Returns an error when the path is not UTF-8.
fn path_string(path: &Path) -> CheckResult<String> {
    return path
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| return format!("{} must be UTF-8", path.display()));
}

/// Read the canonical version through the public Rust contract checker.
///
/// # Errors
///
/// Returns an error when the version contract fails or emits invalid output.
fn public_version(runner: &Runner) -> CheckResult<String> {
    let version = check_try!(command_output(
        runner,
        runner.repo_root.as_path(),
        "cargo",
        &[
            "run",
            "--locked",
            "--quiet",
            "--manifest-path",
            "checks/Cargo.toml",
            "--bin",
            "check-public-contracts",
            "--",
            "public-version",
        ],
    ));
    if version.is_empty() || version.lines().count() != 0x1 {
        return Err("public version checker must return exactly one nonempty line".to_owned());
    }
    return Ok(version);
}

/// Return the host-specific Python executable inside one virtual environment.
fn python_environment_executable(environment: &Path) -> PathBuf {
    return if cfg!(windows) {
        environment.join("Scripts/python.exe")
    } else {
        environment.join("bin/python")
    };
}

/// Build, inspect, and smoke-test every publishable package artifact.
///
/// # Errors
///
/// Returns the first artifact build, policy, install, or runtime failure.
fn run(runner: &Runner) -> CheckResult {
    let version = check_try!(public_version(runner));
    let paths = artifact_paths(runner, version.as_str());
    check_try!(create_artifact_root(&paths));
    check_try!(build_npm_package(runner, &paths));
    check_try!(build_python_packages(runner, &paths));
    check_try!(run_artifact_checker(runner, &paths));
    check_try!(smoke_test_npm(runner, &paths));
    return smoke_test_python(runner, &paths);
}

/// Run the strict Rust archive checker over the synchronized artifact set.
///
/// # Errors
///
/// Returns an error when any artifact violates the exact archive policy.
fn run_artifact_checker(runner: &Runner, paths: &ArtifactPaths) -> CheckResult {
    let arguments = [
        check_try!(path_string(paths.cargo_archive.as_path())),
        check_try!(path_string(paths.npm_archive.as_path())),
        check_try!(path_string(paths.wheel.as_path())),
        check_try!(path_string(paths.sdist.as_path())),
        paths.version.clone(),
    ];
    let borrowed = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    return runner.run_check_bin("check-package-artifacts", borrowed.as_slice());
}

/// Install the packed npm wrapper without network scripts and run its launcher.
///
/// # Errors
///
/// Returns an error when installation or the packed launcher fails.
fn smoke_test_npm(runner: &Runner, paths: &ArtifactPaths) -> CheckResult {
    let install_root = paths.root.join("npm-install");
    let install_root_string = check_try!(path_string(install_root.as_path()));
    let npm_archive = check_try!(path_string(paths.npm_archive.as_path()));
    check_try!(runner.run(
        "npm",
        &[
            "install",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--prefix",
            install_root_string.as_str(),
            npm_archive.as_str(),
        ],
    ));
    let package_root = install_root.join("node_modules/tovuk");
    let installer = check_try!(path_string(package_root.join("install.mjs").as_path()));
    check_try!(runner.status_in(runner.repo_root.as_path(), "node", &[installer.as_str()],));
    let launcher = check_try!(path_string(package_root.join("bin/tovuk.mjs").as_path()));
    let actual = check_try!(command_output(
        runner,
        runner.repo_root.as_path(),
        "node",
        &[launcher.as_str(), "--version"],
    ));
    return verify_version_output(actual.as_str(), paths);
}

/// Install the wheel without dependencies or an index and run its module entrypoint.
///
/// # Errors
///
/// Returns an error when wheel installation or its module entrypoint fails.
fn smoke_test_python(runner: &Runner, paths: &ArtifactPaths) -> CheckResult {
    let environment = paths.root.join("python-venv");
    let environment_string = check_try!(path_string(environment.as_path()));
    let python = check_try!(path_string(runner.python_bin.as_path()));
    check_try!(runner.run(
        "uv",
        &[
            "venv",
            "--no-config",
            "--no-project",
            "--no-python-downloads",
            "--python",
            python.as_str(),
            environment_string.as_str(),
        ],
    ));
    let environment_python = python_environment_executable(environment.as_path());
    let environment_python_string = check_try!(path_string(environment_python.as_path()));
    let wheel = check_try!(path_string(paths.wheel.as_path()));
    check_try!(runner.run(
        "uv",
        &[
            "pip",
            "install",
            "--no-config",
            "--no-deps",
            "--no-index",
            "--python",
            environment_python_string.as_str(),
            "--strict",
            wheel.as_str(),
        ],
    ));
    let actual = check_try!(command_output(
        runner,
        runner.repo_root.as_path(),
        environment_python_string.as_str(),
        &["-m", "tovuk", "--version"],
    ));
    return verify_version_output(actual.as_str(), paths);
}

/// Require a packed wrapper to report the synchronized public version exactly.
///
/// # Errors
///
/// Returns an error when the wrapper reports a different version.
fn verify_version_output(actual: &str, paths: &ArtifactPaths) -> CheckResult {
    if actual == paths.version {
        return Ok(());
    }
    return Err(format!(
        "package artifact runtime reported {actual:?}, expected {:?}",
        paths.version
    ));
}
