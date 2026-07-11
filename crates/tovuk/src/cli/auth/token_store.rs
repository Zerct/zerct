#[cfg(test)]
#[path = "token_store_tests.rs"]
/// Session token storage tests.
mod tests;

use super::{
    super::{
        args::CliOptions,
        constants::{SESSION_DIR, SESSION_FILE},
        errors::{CliError, OutputFormat, Result, agent_error, internal_error},
    },
    keychain::{SystemKeychain, TokenKeychain},
};
use std::{
    env,
    fs::{self as file_system, OpenOptions},
    io::{self as standard_io, Write as _},
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

/// Applies owner-only permissions to session paths.
trait ApplyPrivatePermissions {
    /// Applies this permission profile to `path` where supported.
    fn apply(self, path: &Path);
}

impl From<StoredToken> for Option<String> {
    #[inline]
    fn from(value: StoredToken) -> Self {
        return value.0;
    }
}

impl From<PrivateTempPath> for PathBuf {
    #[inline]
    fn from(value: PrivateTempPath) -> Self {
        return value.0;
    }
}

#[derive(Clone, Copy, Debug)]
/// Path permission update awaiting application.
struct PrivatePath(PrivatePathKind);

impl ApplyPrivatePermissions for PrivatePath {
    fn apply(self, path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = match self.0 {
                PrivatePathKind::Directory => 0o700,
                PrivatePathKind::File => 0o600,
            };
            let _permission_result =
                file_system::set_permissions(path, file_system::Permissions::from_mode(mode));
        }
    }
}

#[derive(Clone, Copy, Debug)]
/// Permission profile for a private session path.
enum PrivatePathKind {
    /// Owner-only directory access.
    Directory,
    /// Owner-only file access.
    File,
}

#[derive(Debug)]
/// Marker confirming a private temporary file was written and synchronized.
struct PrivateTempFile;

impl<'input> TryFrom<PrivateTempFileInput<'input>> for PrivateTempFile {
    type Error = standard_io::Error;

    fn try_from(value: PrivateTempFileInput<'input>) -> standard_io::Result<Self> {
        let mut options = OpenOptions::new();
        let base_options = options.write(true).create_new(true);
        #[cfg(unix)]
        let file_result = {
            use std::os::unix::fs::OpenOptionsExt as _;
            base_options.mode(0o600).open(value.path)
        };
        #[cfg(not(unix))]
        let file_result = base_options.open(value.path);
        let mut file = result_or_return!(file_result);
        result_or_return!(file.write_all(value.contents));
        result_or_return!(file.sync_all());
        return Ok(Self);
    }
}

#[derive(Clone, Copy, Debug)]
/// Contents and destination for a private temporary file.
struct PrivateTempFileInput<'input> {
    /// Bytes to write.
    contents: &'input [u8],
    /// Exclusive temporary file path.
    path: &'input Path,
}

#[derive(Debug)]
/// Collision-resistant temporary path adjacent to the session file.
struct PrivateTempPath(PathBuf);

impl From<&Path> for PrivateTempPath {
    fn from(value: &Path) -> Self {
        let file_name = value
            .file_name()
            .and_then(|name| return name.to_str())
            .unwrap_or(SESSION_FILE);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| return duration.as_nanos());
        return Self(value.with_file_name(format!(".{file_name}.{}.{nanos}.tmp", process::id())));
    }
}

#[derive(Debug)]
/// Optional session token discovered from configured storage sources.
pub(super) struct StoredToken(Option<String>);

impl TryFrom<&CliOptions> for StoredToken {
    type Error = CliError;

    fn try_from(value: &CliOptions) -> Result<Self> {
        if let Some(token) = trimmed_token(value.token()) {
            return Ok(Self(Some(token)));
        }
        if let Ok(environment_token) = env::var("TOVUK_TOKEN")
            && let Some(clean_token) = trimmed_token(environment_token.as_str())
        {
            return Ok(Self(Some(clean_token)));
        }
        if let Some(token) = TokenKeychain::read(SystemKeychain) {
            return Ok(Self(Some(token)));
        }
        if let Some(token) =
            result_or_return!(read_token_file(&user_session_path(), value.output_format(),))
        {
            return Ok(Self(Some(token)));
        }
        return read_token_file(
            &home_dir().join(SESSION_DIR).join(SESSION_FILE),
            value.output_format(),
        )
        .map(Self);
    }
}

#[derive(Clone, Copy, Debug)]
/// Atomic session-file write request.
struct TokenFileWrite<'input> {
    /// Final session file path.
    path: &'input Path,
    /// Trimmed session token.
    token: &'input str,
}

#[derive(Clone, Copy, Debug)]
/// Marker confirming an atomic session-file write completed.
struct WrittenTokenFile;

impl<'input> TryFrom<TokenFileWrite<'input>> for WrittenTokenFile {
    type Error = CliError;

    fn try_from(value: TokenFileWrite<'input>) -> Result<Self> {
        if let Some(parent) = value.path.parent() {
            result_or_return!(
                file_system::create_dir_all(parent)
                    .map_err(|error| return internal_error(error.to_string()))
            );
            ApplyPrivatePermissions::apply(PrivatePath(PrivatePathKind::Directory), parent);
        }
        let temp_path = PathBuf::from(PrivateTempPath::from(value.path));
        let contents = format!("{}\n", value.token);
        let PrivateTempFile = result_or_return!(
            PrivateTempFile::try_from(PrivateTempFileInput {
                contents: contents.as_bytes(),
                path: temp_path.as_path(),
            })
            .map_err(|error| return internal_error(error.to_string()))
        );
        result_or_return!(
            file_system::rename(temp_path.as_path(), value.path).map_err(|error| {
                let _remove_result = file_system::remove_file(temp_path.as_path());
                return internal_error(error.to_string());
            })
        );
        ApplyPrivatePermissions::apply(PrivatePath(PrivatePathKind::File), value.path);
        return Ok(Self);
    }
}

/// Returns the best available user home directory.
fn home_dir() -> PathBuf {
    return env::var_os("HOME")
        .or_else(|| return env::var_os("USERPROFILE"))
        .map_or_else(|| return PathBuf::from("."), PathBuf::from);
}

/// Reads and trims a token from a session file.
///
/// # Errors
///
/// Returns an error when an existing session file cannot be read.
fn read_token_file(path: &Path, output_format: OutputFormat) -> Result<Option<String>> {
    match file_system::read_to_string(path) {
        Ok(source) => return Ok(trimmed_token(source.as_str())),
        Err(error) if error.kind() == standard_io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(agent_error(
                "session_unreadable",
                format!(
                    "Could not read Tovuk session file at {}: {error}.",
                    path.display()
                ),
                format!(
                    "Check file permissions for {} or run `tovuk login` again.",
                    path.display()
                ),
                output_format,
            ));
        }
    }
}

/// Returns a non-empty trimmed token.
fn trimmed_token(value: &str) -> Option<String> {
    let token = value.trim();
    if token.is_empty() {
        return None;
    }
    return Some(token.to_owned());
}

/// Returns the platform-appropriate user session-file path.
fn user_session_path() -> PathBuf {
    if cfg!(windows)
        && let Ok(appdata) = env::var("APPDATA")
    {
        return PathBuf::from(appdata).join("Tovuk").join(SESSION_FILE);
    }
    return env::var_os("XDG_CONFIG_HOME").map_or_else(
        || return home_dir().join(".config").join("tovuk").join(SESSION_FILE),
        |path| return PathBuf::from(path).join("tovuk").join(SESSION_FILE),
    );
}

/// Stores a validated session token in the safest available local store.
///
/// # Errors
///
/// Returns an error when the token is empty or file persistence fails.
pub(super) fn write_session_token(token: &str) -> Result<()> {
    let clean_token = token.trim();
    if clean_token.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk session token is empty.",
            "Run `tovuk login` again and complete the browser login.",
            OutputFormat::Text,
        ));
    }
    if TokenKeychain::write(SystemKeychain, clean_token) {
        return Ok(());
    }
    let WrittenTokenFile = result_or_return!(WrittenTokenFile::try_from(TokenFileWrite {
        path: user_session_path().as_path(),
        token: clean_token,
    }));
    return Ok(());
}
