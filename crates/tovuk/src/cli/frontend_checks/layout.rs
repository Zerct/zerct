use super::super::constants::{
    DEFAULT_BUN_FRONTEND_CHECK_COMMAND, DEFAULT_NPM_FRONTEND_CHECK_COMMAND,
};
use crate::cli::project::read_package_json;
use serde_json::Value;
use std::{fs, path::Path};

const NEXT_CONFIG_FILES: &[&str] = &[
    "next.config.js",
    "next.config.mjs",
    "next.config.ts",
    "next.config.mts",
];
const PACKAGE_DEPENDENCY_SECTIONS: &[&str] = &[
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
];

pub(super) fn frontend_lockfile_exists(project_dir: &Path) -> bool {
    [
        "package-lock.json",
        "npm-shrinkwrap.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lock",
        "bun.lockb",
    ]
    .iter()
    .any(|file| project_dir.join(file).exists())
}

pub(crate) fn is_plain_static_frontend(project_dir: &Path) -> bool {
    !project_dir.join("package.json").exists() && project_dir.join("index.html").exists()
}

pub(crate) fn is_next_frontend(project_dir: &Path) -> bool {
    next_config_file(project_dir).is_some()
        || read_package_json(project_dir).is_some_and(|manifest| package_uses_next(&manifest))
}

pub(super) fn next_config_file(project_dir: &Path) -> Option<&'static str> {
    NEXT_CONFIG_FILES
        .iter()
        .copied()
        .find(|file| project_dir.join(file).exists())
}

pub(super) fn next_static_export_enabled(project_dir: &Path) -> bool {
    let Some(config_file) = next_config_file(project_dir) else {
        return false;
    };
    let Ok(source) = fs::read_to_string(project_dir.join(config_file)) else {
        return false;
    };
    let compact = compact_javascript_config(&source);
    compact.contains("output:\"export\"") || compact.contains("output:'export'")
}

pub(crate) fn frontend_package_manager(project_dir: &Path) -> &'static str {
    if project_dir.join("bun.lock").exists() || project_dir.join("bun.lockb").exists() {
        "bun"
    } else {
        "npm"
    }
}

pub(crate) fn frontend_check_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        DEFAULT_BUN_FRONTEND_CHECK_COMMAND.to_owned()
    } else {
        DEFAULT_NPM_FRONTEND_CHECK_COMMAND.to_owned()
    }
}

pub(crate) fn frontend_build_command(project_dir: &Path) -> String {
    if is_plain_static_frontend(project_dir) {
        ":".to_owned()
    } else if frontend_package_manager(project_dir) == "bun" {
        "bun run build".to_owned()
    } else {
        "npm run build".to_owned()
    }
}

fn package_uses_next(manifest: &Value) -> bool {
    PACKAGE_DEPENDENCY_SECTIONS.iter().any(|section| {
        manifest
            .get(section)
            .and_then(Value::as_object)
            .is_some_and(|dependencies| dependencies.contains_key("next"))
    })
}

fn compact_javascript_config(source: &str) -> String {
    source
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{is_next_frontend, next_static_export_enabled};
    use std::{fs, path::PathBuf};

    #[test]
    fn detects_next_from_package_dependencies() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project("next-package")?;
        fs::write(
            project_dir.join("package.json"),
            r#"{"dependencies":{"next":"16.0.0"}}"#,
        )?;

        require(
            is_next_frontend(&project_dir),
            "Next dependency was not detected",
        )?;
        require(
            !next_static_export_enabled(&project_dir),
            "missing Next config unexpectedly enabled static export",
        )?;

        cleanup(project_dir)
    }

    #[test]
    fn accepts_next_static_export_config() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project("next-static-export")?;
        fs::write(project_dir.join("package.json"), r#"{"dependencies":{}}"#)?;
        fs::write(
            project_dir.join("next.config.mjs"),
            "const nextConfig = {\n  output: \"export\",\n};\nexport default nextConfig;\n",
        )?;

        require(
            is_next_frontend(&project_dir),
            "Next config was not detected",
        )?;
        require(
            next_static_export_enabled(&project_dir),
            "static export config was not accepted",
        )?;

        cleanup(project_dir)
    }

    #[test]
    fn rejects_next_config_without_static_export() -> Result<(), Box<dyn std::error::Error>> {
        let project_dir = temp_project("next-missing-export")?;
        fs::write(project_dir.join("next.config.mjs"), "export default {};\n")?;

        require(
            is_next_frontend(&project_dir),
            "Next config was not detected",
        )?;
        require(
            !next_static_export_enabled(&project_dir),
            "config without static export was accepted",
        )?;

        cleanup(project_dir)
    }

    fn temp_project(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let project_dir = std::env::temp_dir().join(format!("tovuk-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&project_dir);
        fs::create_dir_all(&project_dir)?;
        Ok(project_dir)
    }

    fn cleanup(project_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        fs::remove_dir_all(project_dir)?;
        Ok(())
    }

    fn require(condition: bool, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned().into())
        }
    }
}
