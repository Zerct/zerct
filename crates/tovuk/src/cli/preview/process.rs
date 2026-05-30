use super::super::errors::{Result, agent_error};
use std::{
    path::Path,
    process::{Child, Command},
};

pub(super) fn preview_runtime(project_dir: &Path, command: &str, port: u16) -> Result<()> {
    println!("preview http://127.0.0.1:{port}");
    let status = shell_command(command)
        .current_dir(project_dir)
        .env("PORT", port.to_string())
        .status()
        .map_err(|error| {
            agent_error(
                "preview_failed",
                "Preview command failed.",
                error.to_string(),
                false,
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(agent_error(
            "preview_failed",
            "Preview command exited with an error.",
            "Fix the local runtime command and retry `tovuk preview`.",
            false,
        ))
    }
}

pub(super) fn spawn_preview_backend(command: &str, project_dir: &Path, port: u16) -> Result<Child> {
    shell_command(command)
        .current_dir(project_dir)
        .env("PORT", port.to_string())
        .spawn()
        .map_err(|error| {
            agent_error(
                "preview_failed",
                "Worker preview command failed.",
                error.to_string(),
                false,
            )
        })
}

pub(super) fn run_shell(command: &str, project_dir: &Path, failure_message: &str) -> Result<()> {
    println!("{command}");
    let status = shell_command(command)
        .current_dir(project_dir)
        .status()
        .map_err(|error| {
            agent_error("command_failed", failure_message, error.to_string(), false)
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(agent_error(
            "command_failed",
            failure_message,
            "Fix the command output above, then retry.",
            false,
        ))
    }
}

fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut process = Command::new("cmd");
        process.args(["/C", command]);
        process
    } else {
        let mut process = Command::new("sh");
        process.args(["-c", command]);
        process
    }
}
