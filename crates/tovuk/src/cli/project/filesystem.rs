use super::super::{
    constants::WALK_EXCLUDED_DIRS,
    errors::{Result, agent_error},
};
use std::path::Path;
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

fn is_walk_excluded_entry(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| entry.depth() > 0 && WALK_EXCLUDED_DIRS.contains(&name))
}
