use super::{
    api_commands::api_request,
    args::CliOptions,
    constants::{
        DEFAULT_LOGIN_EXPIRES_SECONDS, DEFAULT_LOGIN_INTERVAL_SECONDS, SESSION_ACCOUNT,
        SESSION_DIR, SESSION_FILE, SESSION_LABEL, SESSION_SERVICE,
    },
    errors::{Result, agent_error, internal_error},
    project::{
        encode_component, has_command, number_alias, open_url, progress, string_alias, string_field,
    },
};
use reqwest::Method;
use serde_json::Value;
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub(crate) fn read_or_login_token(cli: &CliOptions) -> Result<String> {
    let token = read_stored_token(cli);
    if !token.is_empty() {
        return Ok(token);
    }
    login_and_store(cli)
}

pub(crate) fn login(cli: &CliOptions) -> Result<()> {
    if !cli.token.trim().is_empty() {
        write_session_token(cli.token.trim())?;
        println!("saved Tovuk session token");
        return Ok(());
    }
    login_and_store(cli)?;
    Ok(())
}

pub(crate) fn login_and_store(cli: &CliOptions) -> Result<String> {
    let start = api_request(cli, Method::POST, "/v1/login/device", None, None)?;
    let login_url = string_alias(&start, &["loginUrl", "login_url"]);
    let user_code = string_alias(&start, &["userCode", "user_code"]);
    let device_code = string_alias(&start, &["deviceCode", "device_code"]);
    if login_url.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a browser URL.",
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.output.json,
        ));
    }
    open_url(&login_url);
    progress(cli, "opened browser login");
    progress(
        cli,
        &format!(
            "waiting for browser login code {}",
            if user_code.is_empty() {
                "TOVUK"
            } else {
                &user_code
            }
        ),
    );

    let session = poll_login(cli, &device_code, &start)?;
    let token = string_field(&session, "token");
    if token.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a session token.",
            "Run `tovuk login` again and complete the browser login.",
            cli.output.json,
        ));
    }
    write_session_token(&token)?;
    let email = string_field(&session, "email");
    progress(
        cli,
        &format!(
            "logged in as {}",
            if email.is_empty() {
                "Tovuk user"
            } else {
                &email
            }
        ),
    );
    Ok(token)
}

pub(crate) fn poll_login(cli: &CliOptions, device_code: &str, start: &Value) -> Result<Value> {
    if device_code.is_empty() {
        return Err(agent_error(
            "login_failed",
            "Tovuk login did not return a device code.",
            "Retry `tovuk login`. If it keeps failing, check Tovuk status.",
            cli.output.json,
        ));
    }

    let expires_seconds = number_alias(start, &["expiresInSeconds", "expires_in_seconds"])
        .unwrap_or(DEFAULT_LOGIN_EXPIRES_SECONDS);
    let mut interval_seconds = number_alias(start, &["intervalSeconds", "interval_seconds"])
        .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS);
    let deadline = Instant::now() + Duration::from_secs(expires_seconds);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_secs(interval_seconds));
        let response = api_request(
            cli,
            Method::GET,
            &format!("/v1/login/device/{}", encode_component(device_code)),
            None,
            None,
        )?;
        let status = string_field(&response, "status");
        if status == "complete" {
            return Ok(response);
        }
        if status == "expired" {
            return login_expired(cli);
        }
        interval_seconds = number_alias(&response, &["intervalSeconds", "interval_seconds"])
            .unwrap_or(DEFAULT_LOGIN_INTERVAL_SECONDS)
            .max(DEFAULT_LOGIN_INTERVAL_SECONDS);
    }
    login_expired(cli)
}

pub(crate) fn login_expired(cli: &CliOptions) -> Result<Value> {
    Err(agent_error(
        "login_expired",
        "Tovuk login expired before it completed.",
        "Run `tovuk login` again and finish the browser login in the newly opened tab.",
        cli.output.json,
    ))
}

pub(crate) fn read_stored_token(cli: &CliOptions) -> String {
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

pub(crate) fn write_session_token(token: &str) -> Result<()> {
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

pub(crate) fn read_token_file(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|source| source.trim().to_owned())
        .unwrap_or_default()
}

pub(crate) fn write_token_file(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| internal_error(error.to_string()))?;
        set_private_dir(parent);
    }
    fs::write(path, format!("{token}\n")).map_err(|error| internal_error(error.to_string()))?;
    set_private_file(path);
    Ok(())
}

pub(crate) fn user_session_path() -> PathBuf {
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

pub(crate) fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

pub(crate) fn read_keychain_token() -> String {
    if cfg!(target_os = "macos") {
        let result = Command::new("security")
            .args([
                "find-generic-password",
                "-s",
                SESSION_SERVICE,
                "-a",
                SESSION_ACCOUNT,
                "-w",
            ])
            .stderr(Stdio::null())
            .output();
        return result
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
    }

    if cfg!(target_os = "linux") && has_command("secret-tool") {
        let result = Command::new("secret-tool")
            .args([
                "lookup",
                "service",
                SESSION_SERVICE,
                "account",
                SESSION_ACCOUNT,
            ])
            .stderr(Stdio::null())
            .output();
        return result
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .unwrap_or_default();
    }

    String::new()
}

pub(crate) fn write_keychain_token(token: &str) -> bool {
    if cfg!(target_os = "macos") {
        return Command::new("security")
            .args([
                "add-generic-password",
                "-U",
                "-s",
                SESSION_SERVICE,
                "-a",
                SESSION_ACCOUNT,
                "-l",
                SESSION_LABEL,
                "-w",
                token,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
    }

    if cfg!(target_os = "linux") && has_command("secret-tool") {
        let mut child = match Command::new("secret-tool")
            .args([
                "store",
                "--label",
                SESSION_LABEL,
                "service",
                SESSION_SERVICE,
                "account",
                SESSION_ACCOUNT,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_error) => return false,
        };
        if let Some(mut stdin) = child.stdin.take() {
            if stdin.write_all(token.as_bytes()).is_err() {
                return false;
            }
        }
        return child.wait().is_ok_and(|status| status.success());
    }

    false
}

#[cfg(unix)]
pub(crate) fn set_private_file(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ignore = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
pub(crate) fn set_private_file(_path: &Path) {}

#[cfg(unix)]
pub(crate) fn set_private_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ignore = fs::set_permissions(path, fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
pub(crate) fn set_private_dir(_path: &Path) {}
