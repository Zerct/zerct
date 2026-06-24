use std::process::{Command, Output};

use crate::helpers::CheckResult;

const REQUIRED_HELP_COMMANDS: &[&str] = &[
    "tovuk scraper list",
    "tovuk scraper health",
    "tovuk request create",
    "tovuk request results",
    "tovuk pricing",
    "tovuk usage",
    "tovuk billing checkout",
    "tovuk support create",
    "tovuk abuse list --operator",
];

const RETIRED_HELP_COMMANDS: &[&str] = &[
    "tovuk deploy",
    "tovuk service",
    "tovuk storage",
    "tovuk sqlite",
    "tovuk queue",
];

const RETIRED_COMMANDS: &[&str] = &[
    "new",
    "check",
    "dev",
    "deploy",
    "service",
    "logs",
    "sqlite",
    "kv",
    "queue",
    "cron",
    "state",
    "binding",
    "limits",
    "env",
    "secrets",
    "domains",
    "storage",
    "nodes",
    "init",
    "install",
    "preview",
    "capabilities",
    "me",
    "activity",
    "services",
    "overview",
    "deploys",
    "builds",
    "status",
    "inspect",
    "platform",
    "caps",
    "limit",
    "files",
    "media",
];

#[derive(Clone, Copy)]
struct Invocation<'a> {
    label: &'a str,
    program: &'a str,
    prefix_args: &'a [&'a str],
    envs: &'a [(&'a str, &'a str)],
    native_cli_env: Option<&'a str>,
}

pub(crate) fn check(native_cli: &str, python_bin: &str) -> CheckResult {
    let native = Invocation {
        label: "native CLI",
        program: native_cli,
        prefix_args: &[],
        envs: &[],
        native_cli_env: None,
    };
    let python = Invocation {
        label: "Python CLI",
        program: python_bin,
        prefix_args: &["-m", "tovuk"],
        envs: &[("PYTHONPATH", "packages/tovuk-py/src")],
        native_cli_env: Some(native_cli),
    };

    let native_version = native.stdout(&["--version"])?;
    println!("{native_version}");
    assert_help_contract(native.label, &native.help_outputs()?)?;
    require_equal_output(
        native.label,
        native.stdout(&["-V"])?.as_str(),
        native_version.as_str(),
    )?;
    require_equal_output(
        native.label,
        native
            .stdout(&["--api=https://api.example.test", "--version"])?
            .as_str(),
        native_version.as_str(),
    )?;
    native.require_unknown_argument_failure()?;
    native.require_retired_command_failures()?;

    Invocation::require_module_compiles(python_bin)?;
    require_equal_output(
        python.label,
        python.stdout(&["--version"])?.as_str(),
        native_version.as_str(),
    )?;
    assert_help_contract(python.label, &python.help_outputs()?)?;
    require_equal_output(
        python.label,
        python
            .stdout(&["--api=https://api.example.test", "--version"])?
            .as_str(),
        native_version.as_str(),
    )?;
    python.require_unknown_argument_failure()?;
    python.require_retired_command_failures()?;

    println!("Checked runtime CLI help and retired command behavior.");
    Ok(())
}

impl Invocation<'_> {
    fn output(&self, args: &[&str]) -> CheckResult<Output> {
        let mut command = Command::new(self.program);
        command.args(self.prefix_args).args(args);
        for (key, value) in self.envs {
            command.env(key, value);
        }
        if let Some(native_cli) = self.native_cli_env {
            command.env("TOVUK_NATIVE_BINARY", native_cli);
        }
        command.output().map_err(|error| {
            format!(
                "run {} {}: {error}",
                self.program,
                self.prefix_args
                    .iter()
                    .chain(args.iter())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
    }

    fn stdout(&self, args: &[&str]) -> CheckResult<String> {
        let output = self.output(args)?;
        if !output.status.success() {
            return Err(format!(
                "{} {:?} failed with status {}; stderr: {}",
                self.label,
                args,
                output.status,
                String::from_utf8_lossy(output.stderr.as_slice())
            ));
        }
        Ok(String::from_utf8_lossy(output.stdout.as_slice())
            .trim()
            .to_owned())
    }

    fn help_outputs(&self) -> CheckResult<Vec<String>> {
        Ok(vec![
            self.stdout(&[])?,
            self.stdout(&["help"])?,
            self.stdout(&["--help"])?,
        ])
    }

    fn require_unknown_argument_failure(&self) -> CheckResult {
        self.require_failure_code(&["--json", "--definitely-unknown"], "unknown_argument")
    }

    fn require_retired_command_failures(&self) -> CheckResult {
        for retired_command in RETIRED_COMMANDS {
            self.require_failure_code(&[retired_command, "--json"], "unknown_command")?;
        }
        Ok(())
    }

    fn require_failure_code(&self, args: &[&str], expected_code: &str) -> CheckResult {
        let output = self.output(args)?;
        if output.status.success() {
            return Err(format!("expected {} {:?} to fail", self.label, args));
        }
        let stderr = String::from_utf8_lossy(output.stderr.as_slice());
        let expected = format!(r#""code": "{expected_code}""#);
        if stderr.contains(expected.as_str()) {
            return Ok(());
        }
        Err(format!(
            "expected {} {:?} stderr to contain {expected}; stderr: {stderr}",
            self.label, args
        ))
    }

    fn require_module_compiles(python_bin: &str) -> CheckResult {
        let status = Command::new(python_bin)
            .args(["-m", "compileall", "-q", "packages/tovuk-py/src"])
            .status()
            .map_err(|error| format!("run python compileall: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("python compileall failed with status {status}"))
        }
    }
}

fn assert_help_contract(label: &str, outputs: &[String]) -> CheckResult {
    for output in outputs {
        for command in REQUIRED_HELP_COMMANDS {
            if !output.contains(command) {
                return Err(format!("expected {label} help to contain: {command}"));
            }
        }
        for command in RETIRED_HELP_COMMANDS {
            if output.contains(command) {
                return Err(format!("expected {label} help to omit: {command}"));
            }
        }
    }
    Ok(())
}

fn require_equal_output(label: &str, actual: &str, expected: &str) -> CheckResult {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} output mismatch: expected {expected:?}, got {actual:?}"
        ))
    }
}
