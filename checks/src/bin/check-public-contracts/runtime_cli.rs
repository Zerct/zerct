use crate::{
    helpers::{CheckResult, OutputChannel, write_line},
    retired_contracts::{RETIRED_CLI_COMMANDS, RETIRED_HELP_COMMANDS},
};

use std::process::{Command, Output};

/// Contract value named `REQUIRED_HELP_COMMANDS`.
const REQUIRED_HELP_COMMANDS: &[&str] = &[
    "tovuk scraper list",
    "tovuk scraper health",
    "tovuk request create",
    "tovuk request results",
    "tovuk pricing",
    "tovuk usage",
    "tovuk api-key create",
    "tovuk billing checkout",
    "tovuk support create",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&Invocation::require_module_compiles),
    size_of_val(&check),
    size_of_val(&check_native_runtime),
    size_of_val(&check_python_runtime),
];

/// Environment key-value pairs applied to one invocation.
type EnvironmentVariables<'invocation> = [(&'invocation str, &'invocation str)];

#[derive(Clone, Copy)]
/// Contract representation for `Invocation`.
struct Invocation<'invocation> {
    /// Contract data stored in `envs`.
    envs: &'invocation EnvironmentVariables<'invocation>,
    /// Contract data stored in `label`.
    label: &'invocation str,
    /// Contract data stored in `native_cli_env`.
    native_cli_env: Option<&'invocation str>,
    /// Contract data stored in `prefix_args`.
    prefix_args: &'invocation [&'invocation str],
    /// Contract data stored in `program`.
    program: &'invocation str,
}

impl<'invocation> Invocation<'invocation>
where
    Self: Copy + 'invocation,
{
    /// Contract implementation for `help_outputs`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn help_outputs(&self) -> CheckResult<Vec<String>> {
        return Ok(vec![
            check_try!(self.stdout(&[])),
            check_try!(self.stdout(&["help"])),
            check_try!(self.stdout(&["--help"])),
        ]);
    }

    /// Contract implementation for `output`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn output(&self, args: &[&str]) -> CheckResult<Output> {
        let mut command = Command::new(self.program);
        let _: &mut Command = command.args(self.prefix_args).args(args);
        for (key, value) in self.envs.iter().copied() {
            let _: &mut Command = command.env(key, value);
        }
        if let Some(native_cli) = self.native_cli_env {
            let _: &mut Command = command.env("TOVUK_NATIVE_BINARY", native_cli);
        }
        return command.output().map_err(|error| {
            return format!(
                "run {} {}: {error}",
                self.program,
                self.prefix_args
                    .iter()
                    .chain(args.iter())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        });
    }

    /// Contract implementation for `require_api_override_failure`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn require_api_override_failure(&self) -> CheckResult {
        return self.require_failure_code(
            &[
                "--json",
                "account",
                "show",
                "--api=https://api.example.test",
            ],
            "unknown_argument",
        );
    }

    /// Contract implementation for `require_failure_code`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn require_failure_code(&self, args: &[&str], expected_code: &str) -> CheckResult {
        let output = check_try!(self.output(args));
        if output.status.success() {
            return Err(format!("expected {} {args:?} to fail", self.label));
        }
        let stderr = String::from_utf8_lossy(output.stderr.as_slice());
        let expected = format!(r#""code": "{expected_code}""#);
        if stderr.contains(expected.as_str()) {
            return Ok(());
        }
        return Err(format!(
            "expected {} {args:?} stderr to contain {expected}; stderr: {stderr}",
            self.label
        ));
    }

    /// Contract implementation for `require_module_compiles`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    pub(super) fn require_module_compiles(python_bin: &str) -> CheckResult {
        let status = check_try!(
            Command::new(python_bin)
                .args(["-m", "compileall", "-q", "packages/tovuk-py/src"])
                .status()
                .map_err(|error| format!("run python compileall: {error}"))
        );
        if status.success() {
            return Ok(());
        }
        return Err(format!("python compileall failed with status {status}"));
    }

    /// Contract implementation for `require_retired_command_failures`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn require_retired_command_failures(&self) -> CheckResult {
        for retired_command in RETIRED_CLI_COMMANDS {
            check_try!(self.require_failure_code(&[retired_command, "--json"], "unknown_command"));
        }
        check_try!(
            self.require_failure_code(&["account", "update", "--json"], "unknown_account_command")
        );
        return Ok(());
    }

    /// Contract implementation for `require_unknown_argument_failure`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn require_unknown_argument_failure(&self) -> CheckResult {
        return self.require_failure_code(&["--json", "--definitely-unknown"], "unknown_argument");
    }

    /// Contract implementation for `stdout`.
    ///
    /// # Errors
    ///
    /// Returns an error when the contract requirement cannot be verified.
    fn stdout(&self, args: &[&str]) -> CheckResult<String> {
        let output = check_try!(self.output(args));
        if !output.status.success() {
            return Err(format!(
                "{} {args:?} failed with status {}; stderr: {}",
                self.label,
                output.status,
                String::from_utf8_lossy(output.stderr.as_slice())
            ));
        }
        return Ok(String::from_utf8_lossy(output.stdout.as_slice())
            .trim()
            .to_owned());
    }
}

/// Contract implementation for `assert_help_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn assert_help_contract(label: &str, outputs: &[String]) -> CheckResult {
    for output in outputs {
        if let Some(command) = REQUIRED_HELP_COMMANDS
            .iter()
            .find(|command| return !output.contains(**command))
        {
            return Err(format!("expected {label} help to contain: {command}"));
        }
        if let Some(command) = RETIRED_HELP_COMMANDS
            .iter()
            .find(|command| return output.contains(**command))
        {
            return Err(format!("expected {label} help to omit: {command}"));
        }
    }
    return Ok(());
}

/// Contract implementation for `check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check(native_cli: &str, python_bin: &str) -> CheckResult {
    let native = Invocation {
        envs: &[],
        label: "native CLI",
        native_cli_env: None,
        prefix_args: &[],
        program: native_cli,
    };
    let python = Invocation {
        envs: &[("PYTHONPATH", "packages/tovuk-py/src")],
        label: "Python CLI",
        native_cli_env: Some(native_cli),
        prefix_args: &["-m", "tovuk"],
        program: python_bin,
    };

    let native_version = check_try!(check_native_runtime(&native, native.label));
    check_try!(check_python_runtime(
        &python,
        python.label,
        python_bin,
        native_version.as_str()
    ));
    check_try!(write_line(
        OutputChannel::Regular,
        "Checked runtime CLI help and retired command behavior.",
    ));
    return Ok(());
}

/// Check the native CLI runtime and return its canonical version output.
///
/// # Errors
///
/// Returns an error when native runtime behavior drifts.
fn check_native_runtime<'invocation>(
    native: &Invocation<'invocation>,
    label: &'invocation str,
) -> CheckResult<String> {
    let native_version = check_try!(native.stdout(&["--version"]));
    check_try!(write_line(OutputChannel::Regular, native_version.as_str()));
    check_try!(assert_help_contract(
        label,
        &check_try!(native.help_outputs())
    ));
    check_try!(require_equal_output(
        label,
        check_try!(native.stdout(&["-V"])).as_str(),
        native_version.as_str(),
    ));
    check_try!(native.require_unknown_argument_failure());
    check_try!(native.require_api_override_failure());
    check_try!(native.require_retired_command_failures());
    return Ok(native_version);
}

/// Check the Python launcher against the native CLI runtime contract.
///
/// # Errors
///
/// Returns an error when Python launcher behavior drifts.
fn check_python_runtime<'invocation>(
    python: &Invocation<'invocation>,
    label: &'invocation str,
    python_bin: &str,
    native_version: &str,
) -> CheckResult {
    check_try!(Invocation::require_module_compiles(python_bin));
    check_try!(require_equal_output(
        label,
        check_try!(python.stdout(&["--version"])).as_str(),
        native_version,
    ));
    check_try!(assert_help_contract(
        label,
        &check_try!(python.help_outputs())
    ));
    check_try!(python.require_unknown_argument_failure());
    check_try!(python.require_api_override_failure());
    check_try!(python.require_retired_command_failures());
    return Ok(());
}

/// Contract implementation for `require_equal_output`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn require_equal_output(label: &str, actual: &str, expected: &str) -> CheckResult {
    if actual == expected {
        return Ok(());
    }
    return Err(format!(
        "{label} output mismatch: expected {expected:?}, got {actual:?}"
    ));
}
