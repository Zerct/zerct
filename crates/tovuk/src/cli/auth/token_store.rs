use super::{
    super::{
        args::CliOptions,
        constants::{SESSION_DIR, SESSION_FILE},
        errors::{Result, agent_error, internal_error},
    },
    keychain::{read_keychain_token, write_keychain_token},
};
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) fn read_stored_token(cli: &CliOptions) -> Result<Option<String>> {
    if let Some(token) = trimmed_token(cli.token.as_str()) {
        return Ok(Some(token));
    }
    if let Ok(token) = env::var("TOVUK_TOKEN") {
        if let Some(token) = trimmed_token(token.as_str()) {
            return Ok(Some(token));
        }
    }
    if let Some(token) = read_keychain_token() {
        return Ok(Some(token));
    }
    if let Some(token) = read_token_file(&user_session_path(), cli.output.json)? {
        return Ok(Some(token));
    }
    read_token_file(
        &home_dir().join(SESSION_DIR).join(SESSION_FILE),
        cli.output.json,
    )
}

pub(super) fn write_session_token(token: &str) -> Result<()> {
    let clean_token = token.trim();
    if clean_token.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk session token is empty.",
            "Run `tovuk login` again and complete the browser login.",
            false,
        ));
    }
    if write_keychain_token(clean_token) {
        return Ok(());
    }
    write_token_file(&user_session_path(), clean_token)
}

fn read_token_file(path: &Path, json_output: bool) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(source) => Ok(trimmed_token(source.as_str())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(agent_error(
            "session_unreadable",
            format!(
                "Could not read Tovuk session file at {}: {error}.",
                path.display()
            ),
            format!(
                "Check file permissions for {} or run `tovuk login` again.",
                path.display()
            ),
            json_output,
        )),
    }
}

fn trimmed_token(value: &str) -> Option<String> {
    let token = value.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_owned())
    }
}

fn write_token_file(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| internal_error(error.to_string()))?;
        set_private_dir(parent);
    }
    let temp_path = private_temp_path(path);
    write_private_temp_file(temp_path.as_path(), format!("{token}\n").as_bytes())
        .map_err(|error| internal_error(error.to_string()))?;
    fs::rename(temp_path.as_path(), path).map_err(|error| {
        let _ignore = fs::remove_file(temp_path.as_path());
        internal_error(error.to_string())
    })?;
    set_private_file(path);
    Ok(())
}

fn write_private_temp_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

fn private_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(SESSION_FILE);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    path.with_file_name(format!(".{file_name}.{}.{nanos}.tmp", std::process::id()))
}

fn user_session_path() -> PathBuf {
    if cfg!(windows) {
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata).join("Tovuk").join(SESSION_FILE);
        }
    }
    env::var_os("XDG_CONFIG_HOME").map_or_else(
        || home_dir().join(".config").join("tovuk").join(SESSION_FILE),
        |path| PathBuf::from(path).join("tovuk").join(SESSION_FILE),
    )
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

#[cfg(unix)]
fn set_private_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ignore = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn set_private_file(_path: &Path) {}

#[cfg(unix)]
fn set_private_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ignore = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn set_private_dir(_path: &Path) {}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{read_token_file, write_token_file};

    #[test]
    fn missing_session_file_is_not_a_token() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let path = unique_test_path("missing-session-token")?;

        let actual = read_token_file(path.as_path(), false)?;
        if actual.is_some() {
            return Err(format!("missing session file returned token {actual:?}").into());
        }
        Ok(())
    }

    #[test]
    fn session_file_token_is_trimmed() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let path = unique_test_path("session-token")?;
        fs::write(path.as_path(), "  tovuk_session_test\n")?;

        let actual = read_token_file(path.as_path(), false)?;
        if actual.as_deref() != Some("tovuk_session_test") {
            return Err(format!("session token was not trimmed: {actual:?}").into());
        }

        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn session_file_write_is_private_and_atomic()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let path = unique_test_path("session-token-write")?;

        write_token_file(path.as_path(), "tovuk_session_test")?;

        let actual = fs::read_to_string(path.as_path())?;
        if actual.as_str() != "tovuk_session_test\n" {
            return Err(format!("unexpected session file contents: {actual:?}").into());
        }
        assert_private_file(path.as_path())?;

        fs::remove_file(path)?;
        Ok(())
    }

    #[cfg(unix)]
    fn assert_private_file(
        path: &std::path::Path,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;

        let mode = fs::metadata(path)?.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Err(format!("session file mode was {mode:o}, expected 600").into());
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn assert_private_file(
        _path: &std::path::Path,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn unique_test_path(
        label: &str,
    ) -> std::result::Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(std::env::temp_dir().join(format!("tovuk-{label}-{nanos}")))
    }
}
