use crate::helpers::CheckResult;

use std::env;

/// Compile-time references preserve the public docs cache identity boundaries.
const _: [usize; 0x0006] = [
    size_of_val(&Identity::new),
    size_of_val(&read_identity),
    size_of_val(&read_optional_env),
    size_of_val(&render_cache_path),
    size_of_val(&validate_check_id),
    size_of_val(&validate_revision),
];

/// Validated immutable revision and unique workflow-run identity for CDN requests.
#[derive(Debug)]
pub(crate) struct Identity {
    /// Unique `GitHub` workflow run and attempt identifier.
    check_id: String,
    /// Full Git object identifier for the deployment under test.
    revision: String,
}

impl Identity {
    /// Validate and construct one public docs cache identity.
    ///
    /// # Errors
    ///
    /// Returns an error when either value is unsafe for a public query parameter.
    pub(super) fn new(revision: String, check_id: String) -> CheckResult<Self> {
        check_try!(validate_revision(revision.as_str()));
        check_try!(validate_check_id(check_id.as_str()));
        return Ok(Self { check_id, revision });
    }
}

impl AsRef<str> for Identity {
    #[inline]
    fn as_ref(&self) -> &str {
        return self.revision.as_str();
    }
}

/// Read and validate the optional immutable docs deployment cache identity.
///
/// # Errors
///
/// Returns an error when only one value is configured or either value is unsafe.
#[inline]
pub(crate) fn read_identity() -> CheckResult<Option<Identity>> {
    let configured_revision = check_try!(read_optional_env("TOVUK_DOCS_REVISION"));
    let configured_check_id = check_try!(read_optional_env("TOVUK_DOCS_CHECK_ID"));
    return match (configured_revision, configured_check_id) {
        (None, None) => Ok(None),
        (Some(revision), Some(check_id)) => Ok(Some(check_try!(Identity::new(revision, check_id)))),
        _ => Err(
            "TOVUK_DOCS_REVISION and TOVUK_DOCS_CHECK_ID must be configured together.".to_owned(),
        ),
    };
}

/// Read one optional UTF-8 environment value.
///
/// # Errors
///
/// Returns an error when the configured value is not valid UTF-8.
fn read_optional_env(name: &str) -> CheckResult<Option<String>> {
    return match env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("read {name}: {error}")),
    };
}

/// Append a unique deployment check key without accepting arbitrary query input.
pub(super) fn render_cache_path(
    path: &str,
    cache_identity: Option<&Identity>,
    attempt: i64,
) -> String {
    return cache_identity.map_or_else(
        || return path.to_owned(),
        |identity| {
            return format!(
                "{path}?revision={}&check={}&attempt={attempt}",
                identity.as_ref(),
                identity.check_id
            );
        },
    );
}

/// Require a numeric `GitHub` workflow run ID and attempt pair.
///
/// # Errors
///
/// Returns an error when `check_id` is not safe for a public query parameter.
pub(super) fn validate_check_id(check_id: &str) -> CheckResult {
    let Some((run_id, run_attempt)) = check_id.split_once('-') else {
        return Err("TOVUK_DOCS_CHECK_ID must be <run-id>-<run-attempt>.".to_owned());
    };
    let valid = check_id.len() <= 0x40
        && !run_id.is_empty()
        && !run_attempt.is_empty()
        && run_id.bytes().all(|byte| return byte.is_ascii_digit())
        && run_attempt.bytes().all(|byte| return byte.is_ascii_digit());
    if valid {
        return Ok(());
    }
    return Err("TOVUK_DOCS_CHECK_ID must be <run-id>-<run-attempt>.".to_owned());
}

/// Require a full lowercase SHA-1 or SHA-256 Git object identifier.
///
/// # Errors
///
/// Returns an error when `revision` is not safe for a public query parameter.
pub(super) fn validate_revision(revision: &str) -> CheckResult {
    let valid_length = revision.len() == 0x28 || revision.len() == 0x40;
    let valid_characters = revision
        .bytes()
        .all(|byte| return byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid_length && valid_characters {
        return Ok(());
    }
    return Err(
        "TOVUK_DOCS_REVISION must be a full lowercase SHA-1 or SHA-256 object identifier."
            .to_owned(),
    );
}
