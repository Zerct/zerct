use super::{
    super::{
        args::CliOptions,
        constants::{SESSION_DIR, SESSION_FILE},
        errors::{Result, agent_error, internal_error},
    },
    keychain::{read_keychain_token, write_keychain_token},
};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub(super) fn read_stored_token(cli: &CliOptions) -> String {
    if !cli.token.trim().is_empty() {
        return cli.token.trim().to_owned();
    }
    if let Ok(token) = env::var("TOVUK_TOKEN") {
        if !token.trim().is_empty() {
            return token.trim().to_owned();
        }
    }
    let keychain = read_keychain_token();
    if !keychain.is_empty() {
        return keychain;
    }
    let user_token = read_token_file(&user_session_path());
    if !user_token.is_empty() {
        return user_token;
    }
    read_token_file(&home_dir().join(SESSION_DIR).join(SESSION_FILE))
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

fn read_token_file(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|source| source.trim().to_owned())
        .unwrap_or_default()
}

fn write_token_file(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| internal_error(error.to_string()))?;
        set_private_dir(parent);
    }
    fs::write(path, format!("{token}\n")).map_err(|error| internal_error(error.to_string()))?;
    set_private_file(path);
    Ok(())
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
