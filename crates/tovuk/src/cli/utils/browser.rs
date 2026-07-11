use std::process::{Command, Stdio};

/// Opens a public URL with the platform's default browser when possible.
pub(in crate::cli) fn open_url(url: &str) {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        let _command = command.arg(url);
        command
    } else if cfg!(windows) {
        let mut command = Command::new("cmd");
        let _command = command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        let _command = command.arg(url);
        command
    };
    let _ignore = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}
