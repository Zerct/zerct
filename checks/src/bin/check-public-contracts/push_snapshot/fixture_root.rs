//! Collision-free temporary roots for isolated Git fixtures.

use core::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use std::{
    env::temp_dir,
    fs::{DirBuilder, remove_dir_all},
    path::{Path, PathBuf},
    process::id as process_id,
    thread::sleep,
};

use crate::helpers::CheckResult;

/// Process-local sequence used to allocate fixture roots without deletion races.
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0x0000);

/// Bounded delays for transient directory-population races during teardown.
const REMOVE_RETRY_DELAYS: [Duration; 0x0004] = [
    Duration::from_millis(0x0001),
    Duration::from_millis(0x0004),
    Duration::from_millis(0x0010),
    Duration::from_millis(0x0040),
];

/// Atomically allocate a fixture root that cannot collide with another test.
///
/// # Errors
///
/// Returns an error when the operating system cannot create or inspect the fixture root.
pub(super) fn allocate_fixture_root(label: &str) -> CheckResult<PathBuf> {
    loop {
        let sequence = FIXTURE_SEQUENCE.fetch_add(0x0001, Ordering::Relaxed);
        let candidate = temp_dir().join(format!(
            "tovuk-push-snapshot-{}-{sequence}-{label}",
            process_id()
        ));
        let Err(error) = DirBuilder::new().create(candidate.as_path()) else {
            return Ok(candidate);
        };
        let collision = check_try!(
            candidate
                .try_exists()
                .map_err(|inspect_error| return format!(
                    "inspect fixture collision after {error}: {inspect_error}"
                ))
        );
        if !collision {
            return Err(format!("create fixture: {error}"));
        }
    }
}

/// Attempt one recursive removal and report whether the root is now absent.
///
/// # Errors
///
/// Returns an error when the root cannot be inspected after a removal error.
fn cleanup_attempt(root: &Path, label: &str) -> CheckResult<bool> {
    let Err(removal_error) = remove_dir_all(root) else {
        return Ok(true);
    };
    let exists = check_try!(root.try_exists().map_err(|inspection_error| {
        return format!("inspect {label} after cleanup error {removal_error}: {inspection_error}");
    }));
    return Ok(!exists);
}

/// Remove one fixture after all owned Git commands have completed.
///
/// A bounded retry handles the documented operating-system race where a
/// directory receives its final entry while recursive removal is traversing
/// it. Persistent or unrelated errors remain hard failures.
///
/// # Errors
///
/// Returns an error when the fixture cannot be removed completely.
pub(super) fn cleanup_fixture_root(root: &Path, label: &str) -> CheckResult {
    for delay in REMOVE_RETRY_DELAYS {
        if check_try!(cleanup_attempt(root, label)) {
            return Ok(());
        }
        sleep(delay);
    }
    if check_try!(cleanup_attempt(root, label)) {
        return Ok(());
    }
    return Err(format!("clear {label} after bounded retries: root remains"));
}

/// Verify fixture allocation never reuses a live directory.
///
/// # Errors
///
/// Returns an error when fixture allocation, comparison, or cleanup fails.
#[test]
fn fixture_roots_are_unique() -> CheckResult {
    let first = check_try!(allocate_fixture_root("allocator"));
    let second = check_try!(allocate_fixture_root("allocator"));
    if first == second {
        return Err("fixture allocator reused a live directory".to_owned());
    }
    check_try!(cleanup_fixture_root(first.as_path(), "first fixture root"));
    return cleanup_fixture_root(second.as_path(), "second fixture root");
}
