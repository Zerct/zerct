//! Collision-free temporary roots for isolated Git fixtures.

use core::sync::atomic::{AtomicU64, Ordering};
use std::{
    env::temp_dir,
    fs::{DirBuilder, remove_dir_all},
    path::PathBuf,
    process::id as process_id,
};

use crate::helpers::CheckResult;

/// Process-local sequence used to allocate fixture roots without deletion races.
static FIXTURE_SEQUENCE: AtomicU64 = AtomicU64::new(0x0000);

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
    check_try!(
        remove_dir_all(first.as_path())
            .map_err(|error| return format!("remove first fixture root: {error}"))
    );
    return remove_dir_all(second.as_path())
        .map_err(|error| return format!("remove second fixture root: {error}"));
}
