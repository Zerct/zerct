use super::super::constants::{SESSION_ACCOUNT, SESSION_LABEL, SESSION_SERVICE};
use std::{
    env,
    io::Write as _,
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug)]
/// Linux secret-service credential store.
struct LinuxKeychain;

impl TokenKeychain for LinuxKeychain {
    fn read(self) -> Option<String> {
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
        let Some(output) = result.ok().filter(|output| return output.status.success()) else {
            return None;
        };
        return non_empty_stdout_token(&output.stdout);
    }

    fn write(self, token: &str) -> bool {
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
        if let Some(mut stdin) = child.stdin.take()
            && stdin.write_all(token.as_bytes()).is_err()
        {
            return false;
        }
        return child.wait().is_ok_and(|status| return status.success());
    }
}

#[derive(Clone, Copy, Debug)]
/// macOS Keychain credential store.
struct MacKeychain;

impl TokenKeychain for MacKeychain {
    fn read(self) -> Option<String> {
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
        let Some(output) = result.ok().filter(|output| return output.status.success()) else {
            return None;
        };
        return non_empty_stdout_token(&output.stdout);
    }

    fn write(self, _: &str) -> bool {
        // `security add-generic-password -w` exposes the token through argv.
        return false;
    }
}

#[derive(Clone, Copy, Debug)]
/// Platform-selected credential store.
pub(super) struct SystemKeychain;

impl TokenKeychain for SystemKeychain {
    fn read(self) -> Option<String> {
        if cfg!(target_os = "macos") {
            return TokenKeychain::read(MacKeychain);
        }
        if cfg!(target_os = "linux") && has_command("secret-tool") {
            return TokenKeychain::read(LinuxKeychain);
        }
        return None;
    }

    fn write(self, token: &str) -> bool {
        if cfg!(target_os = "linux") && has_command("secret-tool") {
            return TokenKeychain::write(LinuxKeychain, token);
        }
        return TokenKeychain::write(MacKeychain, token);
    }
}

/// Reads and writes session tokens through an operating-system credential store.
pub(super) trait TokenKeychain {
    /// Reads a non-empty stored session token.
    fn read(self) -> Option<String>;

    /// Writes a session token and indicates whether secure storage succeeded.
    fn write(self, token: &str) -> bool;
}

/// Reports whether an executable is available on the current search path.
fn has_command(command: &str) -> bool {
    return env::var_os("PATH").is_some_and(|paths| {
        return env::split_paths(&paths).any(|directory| {
            let candidate = directory.join(command);
            return candidate.is_file()
                || (cfg!(windows) && directory.join(format!("{command}.exe")).is_file());
        });
    });
}

/// Decodes a non-empty token from command standard output.
fn non_empty_stdout_token(stdout: &[u8]) -> Option<String> {
    let token = String::from_utf8_lossy(stdout).trim().to_owned();
    if token.is_empty() {
        return None;
    }
    return Some(token);
}
