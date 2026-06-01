use super::{
    errors::{Result, internal_error},
    project::{ensure_directory, path_relative},
    project_layout::infer_project_kind,
};
use std::{env, fs, path::Path};

mod config;
mod scaffold;

use config::init_config;
use scaffold::create_template;

pub(crate) fn new_project(project_dir: &Path, template: &str) -> Result<()> {
    if !template.is_empty() {
        fs::create_dir_all(project_dir).map_err(|error| internal_error(error.to_string()))?;
        create_template(project_dir, template)?;
        return Ok(());
    }
    ensure_directory(project_dir)?;
    let config_path = project_dir.join("tovuk.toml");
    if config_path.exists() {
        println!("tovuk.toml already exists");
        return Ok(());
    }
    let kind = infer_project_kind(project_dir);
    let source = init_config(project_dir, kind)?;
    fs::write(&config_path, source).map_err(|error| internal_error(error.to_string()))?;
    println!(
        "created {}",
        path_relative(&config_path, &env::current_dir().unwrap_or_default())
    );
    println!("detected {}", kind.as_str());
    Ok(())
}
