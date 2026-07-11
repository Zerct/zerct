//! Fail-closed Git graph-integrity checks shared by local and CI scanning.

use crate::helpers::CheckResult;

use std::{
    fs::symlink_metadata,
    os::unix::fs::FileTypeExt as _,
    path::{Path, PathBuf},
    process::Command,
};

use super::git;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0003] = [
    size_of_val(&graft_path),
    size_of_val(&is_ancestor),
    size_of_val(&require_integrity),
];

/// Resolve Git's effective legacy graft file path.
///
/// # Errors
///
/// Returns an error when Git returns an empty or invalid path.
fn graft_path(repository: &Path) -> CheckResult<PathBuf> {
    let value = check_try!(git::git_text(
        repository,
        &["rev-parse", "--git-path", "info/grafts"],
        "git rev-parse graft path",
    ));
    if value.is_empty() {
        return Err("Git returned an empty graft path".to_owned());
    }
    let path = PathBuf::from(value);
    return if path.is_absolute() {
        Ok(path)
    } else {
        Ok(repository.join(path))
    };
}

/// Return whether one commit is an ancestor of another stored commit.
///
/// # Errors
///
/// Returns an error when Git cannot determine the ancestry relation.
pub(super) fn is_ancestor(
    repository: &Path,
    ancestor: &str,
    descendant: &str,
) -> CheckResult<bool> {
    let status = check_try!(
        Command::new("git")
            .args(["merge-base", "--is-ancestor", ancestor, descendant])
            .current_dir(repository)
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .status()
            .map_err(|error| return format!("run git merge-base --is-ancestor: {error}"))
    );
    return match status.code() {
        Some(0x0000) => Ok(true),
        Some(0x0001) => Ok(false),
        _other => Err(format!("git merge-base --is-ancestor failed with {status}")),
    };
}

/// Require complete history without any legacy graph-rewriting graft file.
///
/// # Errors
///
/// Returns an error for shallow history, unreadable graft metadata, a special
/// graft path, or a nonempty regular graft file.
pub(super) fn require_integrity(repository: &Path) -> CheckResult {
    check_try!(git::require_complete_history(repository));
    let path = check_try!(graft_path(repository));
    let metadata = match symlink_metadata(path.as_path()) {
        Ok(metadata) => metadata,
        Err(error) => {
            let exists = check_try!(path.try_exists().map_err(|inspection_error| {
                return format!(
                    "inspect absent Git graft path {} after {error}: {inspection_error}",
                    path.display()
                );
            }));
            if exists {
                return Err(format!(
                    "inspect Git graft path {}: {error}",
                    path.display()
                ));
            }
            return Ok(());
        }
    };
    let file_type = metadata.file_type();
    let special = file_type.is_block_device()
        || file_type.is_char_device()
        || file_type.is_dir()
        || file_type.is_fifo()
        || file_type.is_socket()
        || file_type.is_symlink();
    if special {
        return Err(format!(
            "Git graft path {} must not be a special file",
            path.display()
        ));
    }
    if metadata.len() != 0 {
        return Err(format!(
            "Git graft path {} must be empty before history scanning",
            path.display()
        ));
    }
    return Ok(());
}
