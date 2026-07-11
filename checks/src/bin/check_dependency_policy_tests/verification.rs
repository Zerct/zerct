use std::{
    env,
    fs::{create_dir_all, read as read_file, remove_dir_all, write as write_file},
    process::id as process_id,
};

use super::deny::TemporaryFile;

use tovuk_public_checks::{check_support::CheckResult, check_try};

/// Verify exclusive creation cannot replace an occupied candidate.
///
/// # Panics
///
/// Panics when fixture I/O fails or collision handling replaces trusted bytes.
#[test]
fn occupied_candidate_is_preserved() {
    let result = (|| -> CheckResult<Vec<u8>> {
        let directory =
            env::temp_dir().join(format!("tovuk-dependency-policy-test-{}", process_id()));
        check_try!(
            create_dir_all(directory.as_path())
                .map_err(|error| return format!("create {}: {error}", directory.display()))
        );
        let candidate = directory.join("occupied.toml");
        check_try!(
            write_file(candidate.as_path(), b"trusted")
                .map_err(|error| return format!("write {}: {error}", candidate.display()))
        );
        let created = check_try!(TemporaryFile::create_new(candidate.clone(), b"untrusted"));
        if created.is_some() {
            return Err("an occupied path was replaced".to_owned());
        }
        let preserved = check_try!(
            read_file(candidate.as_path())
                .map_err(|error| return format!("read {}: {error}", candidate.display()))
        );
        check_try!(
            remove_dir_all(directory.as_path())
                .map_err(|error| return format!("remove {}: {error}", directory.display()))
        );
        return Ok(preserved);
    })();
    assert_eq!(
        result,
        Ok(b"trusted".to_vec()),
        "collision handling must preserve existing bytes"
    );
}
