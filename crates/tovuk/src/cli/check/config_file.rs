use super::report::{QualityCheck, quality_check};
use crate::cli::config::{TovukConfig, parse_tovuk_toml, validate_config};
use std::{fs, path::Path};

pub(super) struct ConfigResult {
    pub(super) check: QualityCheck,
    pub(super) config: Option<TovukConfig>,
    pub(super) valid: bool,
}

pub(super) fn read_config(project_dir: &Path) -> ConfigResult {
    let config_path = project_dir.join("tovuk.toml");
    if !config_path.exists() {
        return ConfigResult {
            check: quality_check(
                "tovuk.toml",
                false,
                "valid",
                "missing",
                "Create and commit tovuk.toml, then retry.",
            ),
            config: None,
            valid: false,
        };
    }
    let source = match fs::read_to_string(&config_path) {
        Ok(source) => source,
        Err(error) => {
            return ConfigResult {
                check: QualityCheck {
                    name: "tovuk.toml".to_owned(),
                    ok: false,
                    message: error.to_string(),
                    agent_instruction: Some(format!("Fix tovuk.toml: {error}.")),
                },
                config: None,
                valid: false,
            };
        }
    };
    match parse_tovuk_toml(&source, project_dir).and_then(|config| {
        validate_config(&config)?;
        Ok(config)
    }) {
        Ok(config) => ConfigResult {
            check: quality_check("tovuk.toml", true, "valid", "missing", ""),
            config: Some(config),
            valid: true,
        },
        Err(message) => ConfigResult {
            check: QualityCheck {
                name: "tovuk.toml".to_owned(),
                ok: false,
                message: message.clone(),
                agent_instruction: Some(format!("Fix tovuk.toml: {message}.")),
            },
            config: None,
            valid: false,
        },
    }
}
