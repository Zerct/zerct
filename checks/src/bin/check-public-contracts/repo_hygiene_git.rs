use crate::helpers::CheckResult;

use std::{path::Path, process::Command};

use tovuk_public_checks::check_support::{git_tracked_files, repo_root};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0008] = [
    size_of_val(&existing_tracked_files),
    size_of_val(&git_status_success),
    size_of_val(&git_status_success_in),
    size_of_val(&is_ordinary_index_entry),
    size_of_val(&require_ordinary_index_entries_in),
    size_of_val(&require_snapshot_alignment),
    size_of_val(&require_snapshot_alignment_in),
    size_of_val(&snapshot_alignment),
];

/// Git comparison and diagnostic label for one hygiene snapshot.
struct SnapshotAlignment {
    /// Git arguments that prove worktree equality.
    args: &'static [&'static str],
    /// Human-readable snapshot label.
    label: &'static str,
}

/// Contract implementation for `existing_tracked_files`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn existing_tracked_files() -> CheckResult<Vec<String>> {
    let repository = check_try!(repo_root());
    return git_tracked_files(repository.as_path());
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
    return git_status_success_in(Path::new("."), args);
}

/// Return whether one Git command succeeds in a selected repository.
///
/// # Errors
///
/// Returns an error when Git cannot be started.
fn git_status_success_in(repository: &Path, args: &[&str]) -> CheckResult<bool> {
    return Command::new("git")
        .args(args)
        .current_dir(repository)
        .status()
        .map(|status| return status.success())
        .map_err(|error| format!("run git {}: {error}", args.join(" ")));
}

/// Return whether one NUL-delimited `git ls-files` record is ordinary.
fn is_ordinary_index_entry(entry: &[u8]) -> bool {
    let Some(body) = entry.strip_prefix(b"H ") else {
        return false;
    };
    let Some(separator) = body.iter().position(|byte| return *byte == b'\t') else {
        return false;
    };
    let (metadata, path_with_separator) = body.split_at(separator);
    let Some((path_separator, path)) = path_with_separator.split_first() else {
        return false;
    };
    if *path_separator != b'\t' {
        return false;
    }
    let mut fields = metadata.split(|byte| return *byte == b' ');
    let (Some(mode), Some(object), Some(stage)) = (fields.next(), fields.next(), fields.next())
    else {
        return false;
    };
    let ordinary_modes = [b"100644".as_slice(), b"100755".as_slice()];
    let ordinary_object = matches!(object.len(), 0x0028 | 0x0040)
        && object.iter().all(u8::is_ascii_hexdigit)
        && object.iter().any(|byte| return *byte != b'0');
    return ordinary_modes.contains(&mode)
        && ordinary_object
        && stage == b"0"
        && fields.next().is_none()
        && !path.is_empty();
}

/// Require every tracked entry to have ordinary stage-zero index state.
///
/// # Errors
///
/// Returns an error for special index flags, nonordinary entries, malformed
/// machine output, or a failed Git command.
fn require_ordinary_index_entries_in(repository: &Path) -> CheckResult {
    let args = [
        "ls-files",
        "--cached",
        "--stage",
        "-t",
        "-v",
        "-f",
        "-z",
        "--full-name",
        "--sparse",
        "--",
    ];
    let output = check_try!(
        Command::new("git")
            .args(args)
            .current_dir(repository)
            .output()
            .map_err(|error| return format!("run git {}: {error}", args.join(" ")))
    );
    if !output.status.success() {
        return Err(format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        ));
    }
    if output.stdout.is_empty() {
        return Ok(());
    }
    let Some(entries) = output.stdout.strip_suffix(b"\0") else {
        return Err("git ls-files returned unterminated machine output".to_owned());
    };
    if entries
        .split(|byte| return *byte == b'\0')
        .any(|entry| return !is_ordinary_index_entry(entry))
    {
        return Err("Git index contains a nonordinary entry or special flag".to_owned());
    }
    return Ok(());
}

/// Require worktree bytes to match the Git snapshot checked by the active gate.
///
/// # Errors
///
/// Returns an error when pre-commit bytes differ from the index, pre-push bytes
/// differ from `HEAD`, or the requested snapshot is unknown.
pub(super) fn require_snapshot_alignment(snapshot: &str) -> CheckResult {
    return require_snapshot_alignment_in(Path::new("."), snapshot);
}

/// Require worktree bytes to match one Git snapshot in a selected repository.
///
/// # Errors
///
/// Returns an error when Git cannot verify the snapshot or bytes differ.
fn require_snapshot_alignment_in(repository: &Path, snapshot: &str) -> CheckResult {
    let alignment = check_try!(snapshot_alignment(snapshot));
    check_try!(require_ordinary_index_entries_in(repository));
    return check_try!(git_status_success_in(repository, alignment.args))
        .then_some(())
        .ok_or_else(|| {
            return format!(
                "tracked worktree bytes must match {} before repository hygiene checks",
                alignment.label
            );
        });
}

/// Resolve one supported repository snapshot into its Git comparison policy.
///
/// # Errors
///
/// Returns an error when the snapshot name is unknown.
fn snapshot_alignment(snapshot: &str) -> CheckResult<SnapshotAlignment> {
    return match snapshot {
        "head" => Ok(SnapshotAlignment {
            args: &["diff", "--quiet", "HEAD", "--"],
            label: "HEAD",
        }),
        "index" => Ok(SnapshotAlignment {
            args: &["diff", "--quiet", "--"],
            label: "the Git index",
        }),
        other => Err(format!("unknown repository hygiene snapshot {other:?}")),
    };
}

#[cfg(test)]
#[path = "repo_hygiene_git_tests/verification.rs"]
mod tests;
