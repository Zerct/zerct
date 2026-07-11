//! Repository prose style checks for public Tovuk text files.

use core::fmt::Display;

use flate2 as _;

use reqwest as _;

use serde as _;

use serde_json as _;

use sha2 as _;

use std::{
    env,
    ffi::OsStr,
    fs::read as read_file,
    io::{Write as _, stderr, stdout},
    path::Path,
    process::{Command, ExitCode},
};

use tar as _;

use tovuk_public_checks as _;

/// Executable prose style check.
trait Check {
    /// Execute the check.
    ///
    /// # Errors
    ///
    /// Returns an error when the check fails or an input cannot be read.
    fn execute(&self) -> CheckResult;
}

/// Result type used by the prose style checker.
type CheckResult<Value = ()> = Result<Value, String>;

/// A prose style violation at a tracked source location.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    /// One-based byte column containing the violation.
    column: usize,
    /// Repository-relative file containing the violation.
    file: String,
    /// One-based line containing the violation.
    line: usize,
    /// Human-readable description of the violation.
    message: &'static str,
}

/// Repository prose style check.
#[derive(Clone, Copy, Debug)]
struct RepositoryCheck;

impl Check for RepositoryCheck {
    fn execute(&self) -> CheckResult {
        let files = match self.git_files() {
            Ok(files) => files,
            Err(error) => return Err(error),
        };
        let mut findings = Vec::new();
        let mut text_file_count = 0;

        let scan_result = files.into_iter().try_for_each(|file| {
            return self.scan_file(file.as_str(), &mut text_file_count, &mut findings);
        });
        if let Err(error) = scan_result {
            return Err(error);
        }

        if findings.is_empty() {
            return write_stdout(format_args!(
                "Checked {text_file_count} text files for em dashes."
            ));
        }

        if let Err(error) = write_stderr(format_args!("Style check failed.")) {
            return Err(error);
        }
        if let Err(error) = write_stderr(format_args!(
            "Em dash is banned in every tracked text file."
        )) {
            return Err(error);
        }
        let report_result = findings.into_iter().try_for_each(|finding| {
            return write_stderr(format_args!(
                "{}:{}:{}: {}",
                finding.file, finding.line, finding.column, finding.message
            ));
        });
        if let Err(error) = report_result {
            return Err(error);
        }
        return Err("prose style check failed".to_owned());
    }
}

impl RepositoryInputs for RepositoryCheck {
    fn git_files(&self) -> CheckResult<Vec<String>> {
        let output = match Command::new("git").args(["ls-files", "--cached"]).output() {
            Ok(output) => output,
            Err(error) => return Err(format!("run git ls-files: {error}")),
        };
        if !output.status.success() {
            return Err(format!("git ls-files failed with status {}", output.status));
        }

        return Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| return !line.is_empty())
            .map(str::to_owned)
            .collect());
    }

    fn has_ignored_binary_extension(&self, file: &str) -> bool {
        return matches!(
            Path::new(file).extension().and_then(OsStr::to_str),
            Some(
                "avif"
                    | "gif"
                    | "ico"
                    | "jpeg"
                    | "jpg"
                    | "otf"
                    | "pdf"
                    | "png"
                    | "ttf"
                    | "webp"
                    | "woff"
                    | "woff2",
            )
        );
    }

    fn scan_file(
        &self,
        file: &str,
        text_file_count: &mut usize,
        findings: &mut Vec<Finding>,
    ) -> CheckResult {
        if self.has_ignored_binary_extension(file) {
            return Ok(());
        }
        let path = Path::new(file);
        let path_exists = match path.try_exists() {
            Ok(path_exists) => path_exists,
            Err(error) => return Err(format!("inspect {file}: {error}")),
        };
        if !path_exists {
            return Ok(());
        }
        let contents = match read_file(file) {
            Ok(contents) => contents,
            Err(error) => return Err(format!("read {file}: {error}")),
        };
        if contents.contains(&0) {
            return Ok(());
        }

        let text = String::from_utf8_lossy(&contents);
        *text_file_count = text_file_count.saturating_add(0x1);
        for (line_index, line) in text.replace("\r\n", "\n").split('\n').enumerate() {
            findings.extend(line_findings(file, line_index, line));
        }
        return Ok(());
    }
}

/// Repository input operations used by the prose style check.
trait RepositoryInputs {
    /// Return tracked repository file paths.
    ///
    /// # Errors
    ///
    /// Returns an error when Git cannot list tracked files.
    fn git_files(&self) -> CheckResult<Vec<String>>;

    /// Return whether a path has a known binary extension.
    fn has_ignored_binary_extension(&self, file: &str) -> bool;

    /// Scan one tracked file and append any prose findings.
    ///
    /// # Errors
    ///
    /// Returns an error when an existing tracked file cannot be read.
    fn scan_file(
        &self,
        file: &str,
        text_file_count: &mut usize,
        findings: &mut Vec<Finding>,
    ) -> CheckResult;
}

/// Prose style checker self-test.
#[derive(Clone, Copy, Debug)]
struct SelfTest;

impl Check for SelfTest {
    fn execute(&self) -> CheckResult {
        let findings = line_findings("self-test", 0, "bad punctuation \u{2014} stop");
        if findings.len() == 0x1
            && findings
                .first()
                .is_some_and(|finding| return finding.column == 0x11)
        {
            return write_stdout(format_args!("Style checker self-test passed."));
        }
        return Err("self-test em dash fixture failed".to_owned());
    }
}

/// Find prohibited prose on one line.
fn line_findings(file: &str, line_index: usize, line: &str) -> Vec<Finding> {
    return line
        .match_indices('\u{2014}')
        .map(|(index, _matched)| {
            return Finding {
                column: index.saturating_add(0x1),
                file: file.to_owned(),
                line: line_index.saturating_add(0x1),
                message: "em dash is not allowed in any tracked text file",
            };
        })
        .collect();
}

fn main() -> ExitCode {
    let arguments = env::args().skip(0x1).collect::<Vec<_>>();
    let check_result = if arguments.is_empty() {
        RepositoryCheck.execute()
    } else if arguments.len() == 0x1
        && arguments
            .first()
            .is_some_and(|argument| return argument == "--self-test")
    {
        SelfTest.execute()
    } else {
        Err("usage: check-prose-style [--self-test]".to_owned())
    };

    match check_result {
        Ok(()) => return ExitCode::SUCCESS,
        Err(message) => {
            return match write_stderr(format_args!("{message}")) {
                Ok(()) | Err(_) => ExitCode::FAILURE,
            };
        }
    }
}

/// Write one diagnostic line to standard error.
///
/// # Errors
///
/// Returns an error when the process standard error stream cannot be written.
fn write_stderr<Diagnostic>(arguments: Diagnostic) -> CheckResult
where
    Diagnostic: Display,
{
    let mut writer = stderr().lock();
    return match writeln!(writer, "{arguments}") {
        Ok(()) => Ok(()),
        Err(error) => Err(format!("write stderr: {error}")),
    };
}

/// Write one status line to standard output.
///
/// # Errors
///
/// Returns an error when the process standard output stream cannot be written.
fn write_stdout<Status>(arguments: Status) -> CheckResult
where
    Status: Display,
{
    let mut writer = stdout().lock();
    return match writeln!(writer, "{arguments}") {
        Ok(()) => Ok(()),
        Err(error) => Err(format!("write stdout: {error}")),
    };
}
