//! Compatible Python interpreter discovery and validation.

use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, Output},
};

use tovuk_public_checks::check_support::{CheckResult, find_command};
use tovuk_public_checks::check_try;

use super::Runner;

/// Minimum Python version supported by the public wrapper package.
const MINIMUM_PYTHON_VERSION: PythonVersion = PythonVersion {
    major: 0x0003,
    minor: 0x000b,
};

/// Parsed Python interpreter version used for compatibility ordering.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PythonVersion {
    /// Major interpreter version.
    major: u16,
    /// Minor interpreter version.
    minor: u16,
}

impl TryFrom<&Output> for PythonVersion {
    type Error = String;

    fn try_from(value: &Output) -> CheckResult<Self> {
        let bytes = if value.stdout.is_empty() {
            value.stderr.as_slice()
        } else {
            value.stdout.as_slice()
        };
        let text = check_try!(
            str::from_utf8(bytes)
                .map_err(|error| return format!("Python version is not UTF-8: {error}"))
        );
        let version_text = check_try!(text.trim().strip_prefix("Python ").ok_or_else(|| {
            return format!("unexpected Python version response {text:?}");
        }));
        let (major, remainder) = check_try!(version_text.split_once('.').ok_or_else(|| {
            return format!("Python version is missing its major component: {version_text:?}");
        }));
        let (minor, _) = check_try!(remainder.split_once('.').ok_or_else(|| {
            return format!("Python version is missing its minor component: {version_text:?}");
        }));
        return Ok(Self {
            major: check_try!(parse_python_component(major, "major")),
            minor: check_try!(parse_python_component(minor, "minor")),
        });
    }
}

impl TryFrom<(PathBuf, OsString)> for Runner {
    type Error = String;

    fn try_from(value: (PathBuf, OsString)) -> CheckResult<Self> {
        let (repo_root, path) = value;
        let python_bin = check_try!(find_command(
            path.as_os_str(),
            &[
                "python3.14",
                "python3.13",
                "python3.12",
                "python3.11",
                "python3",
            ],
        ));
        let output = check_try!(
            Command::new(python_bin.as_path())
                .arg("--version")
                .output()
                .map_err(|error| return format!("inspect {}: {error}", python_bin.display()))
        );
        if !output.status.success() {
            return Err(format!(
                "{} --version failed with status {}",
                python_bin.display(),
                output.status
            ));
        }
        let version = check_try!(PythonVersion::try_from(&output));
        if version < MINIMUM_PYTHON_VERSION {
            return Err(format!(
                "{} reports Python {}.{}, but Python 3.11 or newer is required",
                python_bin.display(),
                version.major,
                version.minor
            ));
        }
        return Ok(Self {
            native_cli: repo_root.join("crates/tovuk/target/release/tovuk"),
            path,
            python_bin,
            repo_root,
        });
    }
}

/// Parse one unsigned Python version component.
///
/// # Errors
///
/// Returns an error when the component is not an unsigned integer.
fn parse_python_component(value: &str, name: &str) -> CheckResult<u16> {
    return value
        .parse::<u16>()
        .map_err(|error| return format!("invalid Python {name} component {value:?}: {error}"));
}
