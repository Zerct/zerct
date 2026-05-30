use super::names::service_name_from_value;
use serde_json::Value;
use std::{fs, path::Path};

pub(crate) fn read_package_json(project_dir: &Path) -> Option<Value> {
    let source = fs::read_to_string(project_dir.join("package.json")).ok()?;
    serde_json::from_str(&source).ok()
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
