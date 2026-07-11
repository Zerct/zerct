//! Full public repository verification runner.

use flate2 as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use std::{
    ffi::OsString,
    io::{Write as _, stderr},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};
use tar as _;
use tovuk_public_checks::check_support::{
    CHECKS_MANIFEST, CheckResult, command as prepare_command, find_command, repo_root, tool_path,
};
use tovuk_public_checks::check_try;

/// Cargo commands that exercise compilation, tests, packaging, and formatting.
const CARGO_QUALITY_COMMANDS: &[&[&str]] = &[
    &["fmt", "--check", "--manifest-path", CLI_MANIFEST],
    &["fmt", "--check", "--manifest-path", CHECKS_MANIFEST],
    &[
        "check",
        "--locked",
        "--release",
        "--all-targets",
        "--all-features",
        "--manifest-path",
        CLI_MANIFEST,
    ],
    &[
        "check",
        "--locked",
        "--release",
        "--all-targets",
        "--all-features",
        "--manifest-path",
        CHECKS_MANIFEST,
    ],
    &[
        "test",
        "--locked",
        "--release",
        "--all-targets",
        "--all-features",
        "--manifest-path",
        CLI_MANIFEST,
    ],
    &[
        "test",
        "--locked",
        "--release",
        "--all-targets",
        "--all-features",
        "--manifest-path",
        CHECKS_MANIFEST,
    ],
    &[
        "build",
        "--locked",
        "--release",
        "--manifest-path",
        CLI_MANIFEST,
    ],
    &["package", "--locked", "--manifest-path", CLI_MANIFEST],
];

/// Checker binaries and arguments used by package and documentation validation.
const CHECK_BIN_COMMANDS: &[CheckBinCommand] = &[
    ("check-prose-style", &["--self-test"]),
    ("check-prose-style", &[]),
    ("check-github-actions", &[]),
    ("check-shell-style", &[]),
    ("check-toml-style", &[]),
];

/// Native CLI Cargo manifest checked by the runner.
const CLI_MANIFEST: &str = "crates/tovuk/Cargo.toml";

/// Public contract groups required by the full repository gate.
const PUBLIC_CONTRACT_COMMANDS: &[&[&str]] = &[&["package-versions"], &["cli-contract"], &["docs"]];

/// Python unit-test command arguments.
const PYTHON_TEST_ARGS: &[&str] = &[
    "-m",
    "unittest",
    "discover",
    "-s",
    "packages/tovuk-py/tests",
];

/// Pinned Ruff lint command arguments.
const RUFF_CHECK_ARGS: &[&str] = &[
    "--from",
    "ruff==0.15.21",
    "ruff",
    "check",
    "packages/tovuk-py",
];

/// Pinned Ruff formatting command arguments.
const RUFF_FORMAT_ARGS: &[&str] = &[
    "--from",
    "ruff==0.15.21",
    "ruff",
    "format",
    "--check",
    "packages/tovuk-py",
];

/// Arguments that make every Clippy invocation enforce the repository policy.
const STRICT_CLIPPY_ARGS: &[&str] = &[
    "--locked",
    "--release",
    "--all-targets",
    "--all-features",
    "--",
    "-D",
    "warnings",
    "-D",
    "clippy::all",
    "-D",
    "clippy::pedantic",
    "-D",
    "clippy::dbg_macro",
    "-D",
    "clippy::todo",
    "-D",
    "clippy::unimplemented",
    "-D",
    "clippy::panic",
    "-D",
    "clippy::unwrap_used",
    "-D",
    "clippy::expect_used",
    "-D",
    "clippy::large_futures",
    "-D",
    "clippy::large_include_file",
    "-D",
    "clippy::large_stack_frames",
    "-D",
    "clippy::mem_forget",
    "-D",
    "clippy::rc_buffer",
    "-D",
    "clippy::rc_mutex",
    "-D",
    "clippy::redundant_clone",
    "-D",
    "clippy::clone_on_ref_ptr",
];

/// Pinned strict Python type-check command arguments.
const TY_CHECK_ARGS: &[&str] = &[
    "--from",
    "ty==0.0.58",
    "ty",
    "check",
    "--project",
    "packages/tovuk-py",
    "--extra-search-path",
    "packages/tovuk-py/src",
    "--error-on-warning",
    "packages/tovuk-py/src",
    "packages/tovuk-py/tests",
];

/// One checker binary invocation in the canonical full gate.
type CheckBinCommand = (&'static str, &'static [&'static str]);

/// Repository paths and executables shared by all verification stages.
struct Runner {
    /// Release-mode native CLI used by runtime contract checks.
    native_cli: PathBuf,
    /// Trusted executable search path.
    path: OsString,
    /// Python interpreter used by wrapper runtime checks.
    python_bin: PathBuf,
    /// Root of the public Git worktree.
    repo_root: PathBuf,
}

impl Runner {
    /// Build a command with the runner's trusted environment.
    fn command(&self, cwd: &Path, program: &str, args: &[&str]) -> Command {
        let mut prepared = prepare_command(cwd, self.path.as_os_str(), program);
        let _: &mut Command = prepared
            .args(args)
            .env("TOVUK_NATIVE_BINARY", self.native_cli.as_os_str());
        return prepared;
    }

    /// Run a command at the repository root.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot start or exits unsuccessfully.
    fn run(&self, program: &str, args: &[&str]) -> CheckResult {
        return self.status_in(self.repo_root.as_path(), program, args);
    }

    /// Run Clippy with the repository's strict argument set.
    ///
    /// # Errors
    ///
    /// Returns an error when Clippy fails.
    fn run_cargo_clippy(&self, manifest: &str) -> CheckResult {
        let mut args = vec!["clippy", "--manifest-path", manifest];
        args.extend_from_slice(STRICT_CLIPPY_ARGS);
        return self.run("cargo", args.as_slice());
    }

    /// Run Cargo compilation, test, lint, and package gates.
    ///
    /// # Errors
    ///
    /// Returns the first failing Cargo or cargo-machete invocation.
    fn run_cargo_quality_gates(&self) -> CheckResult {
        for args in CARGO_QUALITY_COMMANDS.iter().copied() {
            check_try!(self.run("cargo", args));
        }
        check_try!(self.run_cargo_clippy(CLI_MANIFEST));
        check_try!(self.run_cargo_clippy(CHECKS_MANIFEST));
        check_try!(self.run_in("crates/tovuk", "cargo-machete", &[]));
        check_try!(self.run_in("checks", "cargo-machete", &[]));
        return Ok(());
    }

    /// Run one checker binary from the checks crate.
    ///
    /// # Errors
    ///
    /// Returns an error when the checker cannot run successfully.
    fn run_check_bin(&self, bin: &str, args: &[&str]) -> CheckResult {
        let mut command_args = vec![
            "run",
            "--locked",
            "--quiet",
            "--manifest-path",
            CHECKS_MANIFEST,
            "--bin",
            bin,
            "--",
        ];
        command_args.extend_from_slice(args);
        return self.run("cargo", command_args.as_slice());
    }

    /// Run a command in a repository-relative directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot run successfully.
    fn run_in(&self, relative_dir: &str, program: &str, args: &[&str]) -> CheckResult {
        let cwd = self.repo_root.join(relative_dir);
        return self.status_in(cwd.as_path(), program, args);
    }

    /// Run package, documentation, policy, and runtime contract checks.
    ///
    /// # Errors
    ///
    /// Returns the first failing package or documentation check.
    fn run_package_and_docs_checks(&self) -> CheckResult {
        check_try!(self.run_python_quality_gates());
        check_try!(self.run("npm", &["--prefix", "packages/tovuk", "run", "check"]));
        for args in PUBLIC_CONTRACT_COMMANDS.iter().copied() {
            check_try!(self.run_public_contracts(args));
        }
        for (bin, args) in CHECK_BIN_COMMANDS.iter().copied() {
            check_try!(self.run_check_bin(bin, args));
        }
        check_try!(self.run("typos", &["--config", ".typos.toml", "."]));
        check_try!(self.run_check_bin("check-openapi", &[]));
        check_try!(self.run("ruby", &["-c", "Formula/tovuk.rb"]));
        check_try!(
            find_command(self.path.as_os_str(), &["brew"]).map_or(Ok(()), |_| {
                return self.run("brew", &["style", "Formula/tovuk.rb"]);
            })
        );
        let native_cli = self.native_cli.display().to_string();
        let python_bin = self.python_bin.display().to_string();
        return self.run_public_contracts(&[
            "runtime-cli",
            native_cli.as_str(),
            python_bin.as_str(),
        ]);
    }

    /// Run the public contract checker with selected arguments.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract checker fails.
    fn run_public_contracts(&self, args: &[&str]) -> CheckResult {
        return self.run_check_bin("check-public-contracts", args);
    }

    /// Run Python tests, formatting, linting, and strict type checking.
    ///
    /// # Errors
    ///
    /// Returns the first failed Python quality gate.
    fn run_python_quality_gates(&self) -> CheckResult {
        let status = check_try!(
            self.command(self.repo_root.as_path(), "python3", PYTHON_TEST_ARGS)
                .env("PYTHONPATH", "packages/tovuk-py/src")
                .status()
                .map_err(|error| return format!("run python3: {error}"))
        );
        if !status.success() {
            return Err(format!("python3 failed with status {status}"));
        }
        check_try!(self.run("uvx", RUFF_FORMAT_ARGS));
        check_try!(self.run("uvx", RUFF_CHECK_ARGS));
        return self.run("uvx", TY_CHECK_ARGS);
    }

    /// Run a command in a selected working directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot start or exits unsuccessfully.
    fn status_in(&self, cwd: &Path, program: &str, args: &[&str]) -> CheckResult {
        let status = check_try!(
            self.command(cwd, program, args)
                .status()
                .map_err(|error| return format!("run {program}: {error}"))
        );
        return status
            .success()
            .then_some(())
            .ok_or_else(|| return format!("{program} failed with status {status}"));
    }

    /// Verify generated native target manifests and repository hygiene.
    ///
    /// # Errors
    ///
    /// Returns an error when generated files or hygiene checks fail.
    fn verify_generated_manifests(&self) -> CheckResult {
        check_try!(self.run_check_bin("sync-native-release-targets", &["--check"]));
        return self.run_public_contracts(&["repo-hygiene"]);
    }
}

fn main() -> ExitCode {
    let result = (|| -> CheckResult {
        let repository = check_try!(repo_root());
        let path = tool_path();
        let python_bin = check_try!(find_command(path.as_os_str(), &["python3.11", "python3"]));
        let runner = Runner {
            native_cli: repository.join("crates/tovuk/target/release/tovuk"),
            path,
            python_bin,
            repo_root: repository,
        };
        check_try!(runner.verify_generated_manifests());
        check_try!(runner.run_cargo_quality_gates());
        check_try!(runner.run_check_bin("check-dependency-policy", &[]));
        return runner.run_package_and_docs_checks();
    })();
    return match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            drop(writeln!(stderr().lock(), "{error}"));
            ExitCode::FAILURE
        }
    };
}
