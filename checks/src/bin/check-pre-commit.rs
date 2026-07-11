//! Fast, tracked pre-commit verification for the public repository.

use flate2 as _;
use reqwest as _;
use serde as _;
use serde_json as _;
use sha2 as _;
use std::{
    ffi::OsString,
    io::{Write as _, stderr},
    path::PathBuf,
    process::{Command, ExitCode},
};
use tar as _;

use tovuk_public_checks::check_support::{
    CHECKS_MANIFEST, CheckResult, command as prepare_command, repo_root, tool_path,
};
use tovuk_public_checks::check_try;

/// Cargo commands that catch formatting and compiler regressions before commit.
const CARGO_COMMANDS: &[CommandArgs] = &[
    &["fmt", "--check", "--manifest-path", CHECKS_MANIFEST],
    &["fmt", "--check", "--manifest-path", CLI_MANIFEST],
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
        "check",
        "--locked",
        "--release",
        "--all-targets",
        "--all-features",
        "--manifest-path",
        CLI_MANIFEST,
    ],
];

/// Rust-native policy checks kept in the fast local gate.
const CHECK_BIN_COMMANDS: &[CheckBinCommand] = &[
    ("sync-native-release-targets", &["--check"]),
    ("check-public-contracts", &["repo-hygiene"]),
    ("check-github-actions", &[]),
    ("check-prose-style", &["--self-test"]),
    ("check-prose-style", &[]),
    ("check-shell-style", &[]),
    ("check-toml-style", &[]),
];

/// Native CLI Cargo manifest checked by the hook.
const CLI_MANIFEST: &str = "crates/tovuk/Cargo.toml";

/// One Rust checker binary invocation.
type CheckBinCommand = (&'static str, CommandArgs);

/// One immutable command argument list.
type CommandArgs = &'static [&'static str];

/// Repository command runner with an explicit executable search path.
struct Runner {
    /// Trusted executable search path inherited by child commands.
    path: OsString,
    /// Public repository root.
    repo_root: PathBuf,
}

impl Runner {
    /// Build a command rooted in the public repository.
    fn command(&self, program: &str) -> Command {
        return prepare_command(self.repo_root.as_path(), self.path.as_os_str(), program);
    }

    /// Run one command and require a successful exit status.
    ///
    /// # Errors
    ///
    /// Returns an error when the command cannot start or exits unsuccessfully.
    fn run(&self, program: &str, args: &[&str]) -> CheckResult {
        let status = check_try!(
            self.command(program)
                .args(args)
                .status()
                .map_err(|error| return format!("run {program}: {error}"))
        );
        return status
            .success()
            .then_some(())
            .ok_or_else(|| return format!("{program} failed with status {status}"));
    }

    /// Run one Rust checker binary.
    ///
    /// # Errors
    ///
    /// Returns an error when the checker cannot run successfully.
    fn run_check_bin(&self, binary: &str, args: &[&str]) -> CheckResult {
        let mut command_args = vec![
            "run",
            "--locked",
            "--quiet",
            "--manifest-path",
            CHECKS_MANIFEST,
            "--bin",
            binary,
            "--",
        ];
        command_args.extend_from_slice(args);
        return self.run("cargo", command_args.as_slice());
    }

    /// Execute the complete fast pre-commit gate.
    ///
    /// # Errors
    ///
    /// Returns the first failed formatting, compiler, or policy check.
    fn run_fast_gate(&self) -> CheckResult {
        for args in CARGO_COMMANDS.iter().copied() {
            check_try!(self.run("cargo", args));
        }
        for (binary, args) in CHECK_BIN_COMMANDS.iter().copied() {
            check_try!(self.run_check_bin(binary, args));
        }
        check_try!(self.run(
            "npm",
            &["--prefix", "packages/tovuk", "run", "format:check"]
        ));
        check_try!(self.run("npm", &["--prefix", "packages/tovuk", "run", "lint"]));
        check_try!(self.run(
            "uvx",
            &[
                "--from",
                "ruff==0.15.21",
                "ruff",
                "format",
                "--check",
                "packages/tovuk-py",
            ],
        ));
        return self.run(
            "uvx",
            &[
                "--from",
                "ruff==0.15.21",
                "ruff",
                "check",
                "packages/tovuk-py",
            ],
        );
    }
}

fn main() -> ExitCode {
    let result = (|| -> CheckResult {
        let runner = Runner {
            path: tool_path(),
            repo_root: check_try!(repo_root()),
        };
        return runner.run_fast_gate();
    })();
    return match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            drop(writeln!(stderr().lock(), "{error}"));
            ExitCode::FAILURE
        }
    };
}
