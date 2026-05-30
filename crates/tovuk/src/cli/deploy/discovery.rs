use super::{
    super::{
        config::parse_tovuk_toml,
        errors::Result,
        project::path_relative,
        project_layout::{discover_project_dirs, kind_order},
    },
    types::DeployProjectInfo,
};
use std::{fs, path::Path};

pub(crate) fn discover_deploy_projects(root_dir: &Path) -> Result<Vec<DeployProjectInfo>> {
    super::super::project::ensure_directory(root_dir)?;
    if root_dir.join("tovuk.toml").exists() {
        return Ok(vec![deploy_project_info(root_dir, root_dir)]);
    }
    let mut project_dirs = Vec::new();
    discover_project_dirs(root_dir, &mut project_dirs);
    let mut projects = project_dirs
        .iter()
        .map(|dir| deploy_project_info(dir, root_dir))
        .collect::<Vec<_>>();
    projects.sort_by(|left, right| {
        kind_order(left.kind)
            .cmp(&kind_order(right.kind))
            .then_with(|| left.relative.cmp(&right.relative))
    });
    Ok(projects)
}

fn deploy_project_info(dir: &Path, root_dir: &Path) -> DeployProjectInfo {
    let relative = path_relative(dir, root_dir);
    let source = fs::read_to_string(dir.join("tovuk.toml")).unwrap_or_default();
    if let Ok(config) = parse_tovuk_toml(&source, dir) {
        return DeployProjectInfo {
            dir: dir.to_path_buf(),
            relative,
            name: config.name.unwrap_or_default(),
            kind: Some(config.kind),
        };
    }
    DeployProjectInfo {
        dir: dir.to_path_buf(),
        relative,
        name: String::new(),
        kind: None,
    }
}
