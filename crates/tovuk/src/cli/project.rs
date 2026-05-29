use super::config::ProjectKind;
use super::{
    args::CliOptions,
    constants::WALK_EXCLUDED_DIRS,
    errors::{Result, agent_error},
};
use serde_json::Value;
use std::{
    env,
    fmt::Write as _,
    fs,
    path::Path,
    process::{Command, Stdio},
};
use walkdir::{DirEntry, WalkDir};

pub(crate) fn ensure_directory(dir: &Path) -> Result<()> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(agent_error(
            "missing_project",
            "Project directory does not exist.",
            "Run Tovuk from the root of a Rust project or pass the project path.",
            false,
        ))
    }
}

pub(crate) fn walk_project_files(project_dir: &Path, mut visit: impl FnMut(&Path, &str)) {
    for entry in WalkDir::new(project_dir)
        .into_iter()
        .filter_entry(|entry| !is_walk_excluded_entry(entry))
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(relative) = entry.path().strip_prefix(project_dir) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            visit(entry.path(), &relative);
        }
    }
}

pub(crate) fn is_walk_excluded_entry(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| entry.depth() > 0 && WALK_EXCLUDED_DIRS.contains(&name))
}

pub(crate) fn has_command(command: &str) -> bool {
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

pub(crate) fn is_safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && !Path::new(value).is_absolute()
        && !value.contains('\\')
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

pub(crate) fn is_safe_relative_directory(value: &str) -> bool {
    value == "." || is_safe_relative_path(value)
}

pub(crate) fn read_package_json(project_dir: &Path) -> Option<Value> {
    let source = fs::read_to_string(project_dir.join("package.json")).ok()?;
    serde_json::from_str(&source).ok()
}

pub(crate) fn service_name_from_dir(project_dir: &Path) -> String {
    service_name_from_value(
        project_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
    )
    .unwrap_or_else(|| "api".to_owned())
}

pub(crate) fn service_name_from_cargo(project_dir: &Path) -> Option<String> {
    let source = fs::read_to_string(project_dir.join("Cargo.toml")).ok()?;
    let name = source.lines().find_map(|line| {
        let line = line.trim();
        if !line.starts_with("name") {
            return None;
        }
        line.split('"').nth(1).map(str::to_owned)
    })?;
    service_name_from_value(&name)
}

pub(crate) fn service_name_from_package(project_dir: &Path) -> Option<String> {
    let manifest = read_package_json(project_dir)?;
    let name = manifest.get("name")?.as_str()?;
    service_name_from_value(name)
}

pub(crate) fn service_name_from_value(value: &str) -> Option<String> {
    let mut result = String::new();
    let mut last_dash = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
        if result.len() >= 48 {
            break;
        }
    }
    let result = result.trim_matches('-').to_owned();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

pub(crate) fn infer_project_kind(project_dir: &Path) -> ProjectKind {
    if detect_fullstack_roots(project_dir).is_some() {
        ProjectKind::Fullstack
    } else if project_dir.join("Cargo.toml").exists() {
        ProjectKind::RustBackend
    } else if project_dir.join("package.json").exists() || project_dir.join("index.html").exists() {
        ProjectKind::StaticFrontend
    } else {
        ProjectKind::RustBackend
    }
}

pub(crate) fn detect_fullstack_roots(project_dir: &Path) -> Option<(String, String)> {
    let backend = ["api", "backend", "server"]
        .iter()
        .find(|root| project_dir.join(root).join("Cargo.toml").exists())?;
    let frontend = ["web", "frontend", "app", "site"].iter().find(|root| {
        project_dir.join(root).join("package.json").exists()
            || project_dir.join(root).join("index.html").exists()
    })?;
    Some(((*backend).to_owned(), (*frontend).to_owned()))
}

pub(crate) fn is_dns_safe_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 48 {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn path_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map_or_else(
            || ".".to_owned(),
            |relative| relative.to_string_lossy().replace('\\', "/"),
        )
}

pub(crate) fn open_url(url: &str) {
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };
    let _ignore = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

pub(crate) fn progress(cli: &CliOptions, message: &str) {
    if cli.output.json {
        eprintln!("{message}");
    } else {
        println!("{message}");
    }
}

pub(crate) fn encode_component(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            let _ = write!(output, "%{byte:02X}");
        }
    }
    output
}

pub(crate) fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn number_field(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_else(|| {
        value
            .get(key)
            .and_then(Value::as_i64)
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(0)
    })
}

pub(crate) fn string_alias(value: &Value, aliases: &[&str]) -> String {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn number_alias(value: &Value, aliases: &[&str]) -> Option<u64> {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_u64))
}

pub(crate) fn nested_string(value: &Value, path: &[&str]) -> String {
    let mut cursor = value;
    for part in path {
        cursor = cursor.get(part).unwrap_or(&Value::Null);
    }
    cursor.as_str().unwrap_or_default().to_owned()
}
