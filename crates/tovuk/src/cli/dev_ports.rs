use serde::Serialize;
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DevPortOwner {
    pub(crate) command: String,
    pub(crate) pid: u32,
}

pub(crate) fn port_owner(port: u16) -> Option<DevPortOwner> {
    platform_port_owner(port)
}

#[cfg(not(windows))]
fn platform_port_owner(port: u16) -> Option<DevPortOwner> {
    let output = Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-Fpc"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let source = String::from_utf8_lossy(&output.stdout);
    parse_lsof_owner(&source)
}

#[cfg(windows)]
fn platform_port_owner(port: u16) -> Option<DevPortOwner> {
    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let source = String::from_utf8_lossy(&output.stdout);
    let pid = parse_netstat_owner_pid(&source, port)?;
    let command = windows_process_name(pid).unwrap_or_else(|| "unknown".to_owned());
    Some(DevPortOwner { command, pid })
}

#[cfg(not(windows))]
fn parse_lsof_owner(source: &str) -> Option<DevPortOwner> {
    let mut pid = None;
    let mut command = None;
    for line in source.lines() {
        if let Some(value) = line.strip_prefix('p') {
            pid = value.parse::<u32>().ok();
        } else if let Some(value) = line.strip_prefix('c').filter(|value| !value.is_empty()) {
            command = Some(value.to_owned());
        }
    }
    pid.map(|owner_pid| DevPortOwner {
        command: command.unwrap_or_else(|| "unknown".to_owned()),
        pid: owner_pid,
    })
}

#[cfg(windows)]
fn parse_netstat_owner_pid(source: &str, port: u16) -> Option<u32> {
    let expected_suffix = format!(":{port}");
    source.lines().find_map(|line| {
        let columns = line.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 5 || !columns[0].eq_ignore_ascii_case("TCP") {
            return None;
        }
        let local_address = columns[1];
        let state = columns[3];
        if !state.eq_ignore_ascii_case("LISTENING") || !local_address.ends_with(&expected_suffix) {
            return None;
        }
        columns[4].parse::<u32>().ok()
    })
}

#[cfg(windows)]
fn windows_process_name(pid: u32) -> Option<String> {
    let output = Command::new("tasklist")
        .args(["/fi", &format!("PID eq {pid}"), "/nh"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let source = String::from_utf8_lossy(&output.stdout);
    source
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .map(ToOwned::to_owned)
        .filter(|value| !value.is_empty() && value != "INFO:")
}

#[cfg(test)]
mod tests {
    #[cfg(not(windows))]
    #[test]
    fn lsof_owner_parser_reads_pid_and_command() -> Result<(), Box<dyn std::error::Error>> {
        let owner =
            super::parse_lsof_owner("p123\ncapi\n").ok_or("expected parser to return owner")?;

        if owner.pid != 123 {
            return Err(format!("unexpected pid: {}", owner.pid).into());
        }
        if owner.command != "api" {
            return Err(format!("unexpected command: {}", owner.command).into());
        }

        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn netstat_owner_parser_reads_listening_pid() -> Result<(), Box<dyn std::error::Error>> {
        let owner = super::parse_netstat_owner_pid(
            "TCP    127.0.0.1:3000    0.0.0.0:0    LISTENING    123",
            3000,
        )
        .ok_or("expected parser to return owner pid")?;

        if owner != 123 {
            return Err(format!("unexpected pid: {owner}").into());
        }

        Ok(())
    }
}
