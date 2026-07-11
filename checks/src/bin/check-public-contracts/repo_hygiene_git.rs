use alloc::collections::BTreeSet;

use crate::helpers::CheckResult;

use std::process::Command;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&existing_tracked_files),
    size_of_val(&git_status_success),
];

/// Contract implementation for `existing_tracked_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn existing_tracked_files() -> CheckResult<Vec<String>> {
    let deleted_files = check_try!(git_lines(&["ls-files", "--deleted"]))
        .into_iter()
        .collect::<BTreeSet<_>>();
    return Ok(check_try!(git_lines(&["ls-files"]))
        .into_iter()
        .filter(|path| return !deleted_files.contains(path))
        .collect());
}

/// Contract implementation for `git_lines`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn git_lines(args: &[&str]) -> CheckResult<Vec<String>> {
    let output = check_try!(
        Command::new("git")
            .args(args)
            .output()
            .map_err(|error| format!("run git {}: {error}", args.join(" ")))
    );
    if !output.status.success() {
        return Err(format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        ));
    }
    return Ok(String::from_utf8_lossy(output.stdout.as_slice())
        .lines()
        .filter(|line| return !line.is_empty())
        .map(str::to_owned)
        .collect());
}

/// Contract implementation for `git_status_success`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn git_status_success(args: &[&str]) -> CheckResult<bool> {
    return Command::new("git")
        .args(args)
        .status()
        .map(|status| return status.success())
        .map_err(|error| format!("run git {}: {error}", args.join(" ")));
}
