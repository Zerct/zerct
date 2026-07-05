//! Full public repository verification runner.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

use tovuk_public_checks::check_support::{
    CHECKS_MANIFEST, CheckResult, find_command, repo_root, tool_path,
};

const CLI_MANIFEST: &str = "crates/tovuk/Cargo.toml";
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

fn main() -> ExitCode {
    match Runner::new().and_then(|runner| runner.run_all()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

struct Runner {
    native_cli: PathBuf,
    path: OsString,
    python_bin: PathBuf,
    repo_root: PathBuf,
}

impl Runner {
    fn new() -> CheckResult<Self> {
        let repo_root = repo_root()?;
        let path = tool_path();
        let python_bin = find_command(path.as_os_str(), &["python3.11", "python3"])?;
        let native_cli = repo_root
            .join("crates")
            .join("tovuk")
            .join("target")
            .join("release")
            .join("tovuk");
        Ok(Self {
            native_cli,
            path,
            python_bin,
            repo_root,
        })
    }

    fn run_all(&self) -> CheckResult {
        self.sync_generated_manifests()?;
        self.run_cargo_quality_gates()?;
        self.run_dependency_policy()?;
        self.run_package_and_docs_checks()
    }

    fn sync_generated_manifests(&self) -> CheckResult {
        self.run("scripts/sync-native-release-targets.sh", &[])?;
        self.run("scripts/sync-native-release-targets.sh", &["--check"])?;
        self.run_public_contracts(&["repo-hygiene"])
    }

    fn run_cargo_quality_gates(&self) -> CheckResult {
        self.run(
            "cargo",
            &["fmt", "--check", "--manifest-path", CLI_MANIFEST],
        )?;
        self.run(
            "cargo",
            &["fmt", "--check", "--manifest-path", CHECKS_MANIFEST],
        )?;
        self.run(
            "cargo",
            &[
                "check",
                "--locked",
                "--release",
                "--all-targets",
                "--all-features",
                "--manifest-path",
                CLI_MANIFEST,
            ],
        )?;
        self.run(
            "cargo",
            &[
                "check",
                "--locked",
                "--release",
                "--all-targets",
                "--all-features",
                "--manifest-path",
                CHECKS_MANIFEST,
            ],
        )?;
        self.run(
            "cargo",
            &[
                "test",
                "--locked",
                "--release",
                "--all-targets",
                "--all-features",
                "--manifest-path",
                CLI_MANIFEST,
            ],
        )?;
        self.run(
            "cargo",
            &[
                "test",
                "--locked",
                "--release",
                "--all-targets",
                "--all-features",
                "--manifest-path",
                CHECKS_MANIFEST,
            ],
        )?;
        self.run_cargo_clippy(CLI_MANIFEST)?;
        self.run_cargo_clippy(CHECKS_MANIFEST)?;
        self.run(
            "cargo",
            &[
                "build",
                "--locked",
                "--release",
                "--manifest-path",
                CLI_MANIFEST,
            ],
        )?;
        self.run(
            "cargo",
            &[
                "package",
                "--locked",
                "--manifest-path",
                CLI_MANIFEST,
                "--allow-dirty",
            ],
        )?;
        self.run_in("crates/tovuk", "cargo-machete", &[])?;
        self.run_in("checks", "cargo-machete", &[])?;
        Ok(())
    }

    fn run_dependency_policy(&self) -> CheckResult {
        fs::create_dir_all(self.repo_root.join("target"))
            .map_err(|error| format!("create target directory: {error}"))?;
        self.write_command_stdout(
            "cargo",
            &[
                "metadata",
                "--locked",
                "--manifest-path",
                CLI_MANIFEST,
                "--all-features",
                "--format-version",
                "1",
            ],
            "target/tovuk-cargo-deny-metadata.json",
        )?;
        self.run(
            "cargo",
            &[
                "deny",
                "--manifest-path",
                CLI_MANIFEST,
                "check",
                "--config",
                "deny.toml",
                "--metadata-path",
                "target/tovuk-cargo-deny-metadata.json",
                "all",
            ],
        )?;
        self.write_command_stdout(
            "cargo",
            &[
                "metadata",
                "--locked",
                "--manifest-path",
                CHECKS_MANIFEST,
                "--all-features",
                "--format-version",
                "1",
            ],
            "target/tovuk-public-checks-cargo-deny-metadata.json",
        )?;
        self.run(
            "cargo",
            &[
                "deny",
                "--manifest-path",
                CHECKS_MANIFEST,
                "check",
                "--config",
                "deny.toml",
                "--metadata-path",
                "target/tovuk-public-checks-cargo-deny-metadata.json",
                "all",
            ],
        )
    }

    fn run_package_and_docs_checks(&self) -> CheckResult {
        self.run("npm", &["--prefix", "packages/tovuk", "run", "check"])?;
        self.run_public_contracts(&["package-versions"])?;
        self.run_public_contracts(&["cli-contract"])?;
        self.run_public_contracts(&["docs"])?;
        self.run_check_bin("check-prose-style", &["--self-test"])?;
        self.run_check_bin("check-prose-style", &[])?;
        self.run_check_bin("check-github-actions", &[])?;
        self.run_check_bin("check-shell-style", &[])?;
        self.run_check_bin("check-toml-style", &[])?;
        self.run("typos", &["--config", ".typos.toml", "."])?;
        self.run_check_bin("check-openapi", &[])?;
        self.run("ruby", &["-c", "Formula/tovuk.rb"])?;
        if find_command(self.path.as_os_str(), &["brew"]).is_ok() {
            self.run("brew", &["style", "Formula/tovuk.rb"])?;
        }
        let native_cli = self.native_cli.display().to_string();
        let python_bin = self.python_bin.display().to_string();
        self.run_public_contracts(&["runtime-cli", native_cli.as_str(), python_bin.as_str()])
    }

    fn run_cargo_clippy(&self, manifest: &str) -> CheckResult {
        let mut args = vec!["clippy", "--manifest-path", manifest];
        args.extend_from_slice(STRICT_CLIPPY_ARGS);
        self.run("cargo", args.as_slice())
    }

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
        self.run("cargo", command_args.as_slice())
    }

    fn run_public_contracts(&self, args: &[&str]) -> CheckResult {
        self.run_check_bin("check-public-contracts", args)
    }

    fn run_in(&self, relative_dir: &str, program: &str, args: &[&str]) -> CheckResult {
        let cwd = self.repo_root.join(relative_dir);
        self.status_in(cwd.as_path(), program, args)
    }

    fn run(&self, program: &str, args: &[&str]) -> CheckResult {
        self.status_in(self.repo_root.as_path(), program, args)
    }

    fn status_in(&self, cwd: &Path, program: &str, args: &[&str]) -> CheckResult {
        let status = self
            .command(cwd, program, args)
            .status()
            .map_err(|error| format!("run {program}: {error}"))?;
        status
            .success()
            .then_some(())
            .ok_or_else(|| format!("{program} failed with status {status}"))
    }

    fn write_command_stdout(&self, program: &str, args: &[&str], output_path: &str) -> CheckResult {
        let output = self
            .command(self.repo_root.as_path(), program, args)
            .output()
            .map_err(|error| format!("run {program}: {error}"))?;
        if !output.status.success() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            return Err(format!("{program} failed with status {}", output.status));
        }
        fs::write(self.repo_root.join(output_path), output.stdout)
            .map_err(|error| format!("write {output_path}: {error}"))
    }

    fn command(&self, cwd: &Path, program: &str, args: &[&str]) -> Command {
        let mut command =
            tovuk_public_checks::check_support::command(cwd, self.path.as_os_str(), program);
        command
            .args(args)
            .env("TOVUK_NATIVE_BINARY", self.native_cli.as_os_str());
        command
    }
}
