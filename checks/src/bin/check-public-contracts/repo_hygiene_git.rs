use std::{collections::BTreeSet, process::Command};

use crate::helpers::CheckResult;

pub(crate) fn existing_tracked_files() -> CheckResult<Vec<String>> {
    let deleted_files = git_lines(&["ls-files", "--deleted"])?
        .into_iter()
        .collect::<BTreeSet<_>>();
    Ok(git_lines(&["ls-files"])?
        .into_iter()
        .filter(|path| !deleted_files.contains(path))
        .collect())
}

pub(crate) fn git_lines(args: &[&str]) -> CheckResult<Vec<String>> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("run git {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed with status {}",
            args.join(" "),
            output.status
        ));
    }
    Ok(String::from_utf8_lossy(output.stdout.as_slice())
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

pub(crate) fn git_status_success(args: &[&str]) -> CheckResult<bool> {
    Command::new("git")
        .args(args)
        .status()
        .map(|status| status.success())
        .map_err(|error| format!("run git {}: {error}", args.join(" ")))
}
