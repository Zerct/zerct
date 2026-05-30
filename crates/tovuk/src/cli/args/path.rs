use std::{env, path::PathBuf};

use crate::cli::errors::{Result, internal_error};

pub(crate) fn project_path(value: Option<&String>) -> Result<PathBuf> {
    let path = value.map_or_else(PathBuf::new, PathBuf::from);
    let path = if path.as_os_str().is_empty() {
        env::current_dir().map_err(|error| internal_error(error.to_string()))?
    } else if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|error| internal_error(error.to_string()))?
            .join(path)
    };
    Ok(path)
}
