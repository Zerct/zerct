use core::error::Error;
use std::{
    env,
    fs::{self as file_system},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{OutputFormat, TokenFileWrite, WrittenTokenFile, read_token_file};

/// Result returned by session token storage tests.
type TestResult<Value = ()> = Result<Value, Box<dyn Error>>;

#[test]
/// Verifies an absent session file is treated as an absent token.
///
/// # Errors
///
/// Returns an error when path creation fails or the read contract changes.
fn missing_session_file_is_not_a_token() -> TestResult {
    let path = result_or_return!(unique_test_path("missing-session-token"));

    let actual = match read_token_file(path.as_path(), OutputFormat::Text) {
        Ok(token) => token,
        Err(error) => return Err(error.message().to_owned().into()),
    };
    if actual.is_some() {
        return Err(format!("missing session file returned token {actual:?}").into());
    }
    return Ok(());
}

#[test]
/// Verifies session tokens are trimmed when read from disk.
///
/// # Errors
///
/// Returns an error when file I/O fails or token normalization changes.
fn session_file_token_is_trimmed() -> TestResult {
    let path = result_or_return!(unique_test_path("session-token"));
    result_or_return!(
        file_system::write(path.as_path(), "  tovuk_session_test\n").map_err(|error| return Box::<
            dyn Error,
        >::from(
            error
        ))
    );

    let actual = match read_token_file(path.as_path(), OutputFormat::Text) {
        Ok(token) => token,
        Err(error) => return Err(error.message().to_owned().into()),
    };
    if actual.as_deref() != Some("tovuk_session_test") {
        return Err(format!("session token was not trimmed: {actual:?}").into());
    }

    result_or_return!(
        file_system::remove_file(path).map_err(|error| return Box::<dyn Error>::from(error))
    );
    return Ok(());
}

#[test]
/// Verifies session-file writes are atomic and owner-private where supported.
///
/// # Errors
///
/// Returns an error when persistence, permission checks, or cleanup fails.
fn session_file_write_is_private_and_atomic() -> TestResult {
    let path = result_or_return!(unique_test_path("session-token-write"));

    if let Err(error) = WrittenTokenFile::try_from(TokenFileWrite {
        path: path.as_path(),
        token: "tovuk_session_test",
    }) {
        return Err(error.message().to_owned().into());
    }

    let actual = result_or_return!(
        file_system::read_to_string(path.as_path())
            .map_err(|error| return Box::<dyn Error>::from(error))
    );
    if actual.as_str() != "tovuk_session_test\n" {
        return Err(format!("unexpected session file contents: {actual:?}").into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        let metadata = result_or_return!(
            file_system::metadata(path.as_path())
                .map_err(|error| return Box::<dyn Error>::from(error))
        );
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Err(format!("session file mode was {mode:o}, expected 600").into());
        }
    }

    result_or_return!(
        file_system::remove_file(path).map_err(|error| return Box::<dyn Error>::from(error))
    );
    return Ok(());
}

/// Creates a collision-resistant temporary path for a storage test.
///
/// # Errors
///
/// Returns an error when the system clock predates the Unix epoch.
fn unique_test_path(label: &str) -> TestResult<PathBuf> {
    let nanos = result_or_return!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| return Box::<dyn Error>::from(error))
    )
    .as_nanos();
    return Ok(env::temp_dir().join(format!("tovuk-{label}-{nanos}")));
}
