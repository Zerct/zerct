use super::super::{
    config::{TovukConfig, parse_tovuk_toml, validate_config},
    doctor::{DoctorReportKind, run_doctor_workspace},
    errors::{Result, agent_error, internal_error},
};
use std::{fs, path::Path};

pub(super) fn preview_config(project_dir: &Path) -> Result<TovukConfig> {
    let report = run_doctor_workspace(project_dir);
    if matches!(report, DoctorReportKind::Workspace(_)) {
        return Err(agent_error(
            "workspace_preview_unsupported",
            "Preview one project at a time.",
            "Run `tovuk preview <project-dir>` for one discovered project, or use a worker-static root tovuk.toml.",
            false,
        ));
    }
    if !report.ok() {
        let instruction = report
            .checks()
            .iter()
            .find(|check| !check.ok)
            .and_then(|check| check.agent_instruction.clone())
            .unwrap_or_else(|| "Fix the failed checks and retry `tovuk preview`.".to_owned());
        return Err(agent_error(
            "doctor_failed",
            "Tovuk doctor failed.",
            instruction,
            false,
        ));
    }
    let source = fs::read_to_string(project_dir.join("tovuk.toml"))
        .map_err(|error| internal_error(error.to_string()))?;
    let config = parse_tovuk_toml(&source, project_dir).map_err(internal_error)?;
    validate_config(&config).map_err(internal_error)?;
    Ok(config)
}
