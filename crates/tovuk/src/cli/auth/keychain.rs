use super::super::constants::{SESSION_ACCOUNT, SESSION_LABEL, SESSION_SERVICE};
use std::{
    env,
    io::Write,
    process::{Command, Stdio},
};

pub(super) fn read_keychain_token() -> String {
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

pub(super) fn write_keychain_token(token: &str) -> bool {
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

fn has_command(command: &str) -> bool {
    env::var_os("PATH").is_some_and(|paths| {
        env::split_paths(&paths).any(|directory| {
            let candidate = directory.join(command);
            if cfg!(windows) {
                candidate.is_file() || directory.join(format!("{command}.exe")).is_file()
            } else {
                candidate.is_file()
            }
        })
    })
}
