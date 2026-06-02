mod cargo_lints;
mod checks;
mod config_file;
mod report;

use super::{
    deploy::discover_deploy_projects,
    errors::{AgentErrorPayload, CliError, CliFailure, Result, internal_error, print_json},
    project_kind::ProjectKind,
};
pub(crate) use checks::first_output_line;
use checks::{fullstack_checks, required_file_checks, rust_quality_checks};
use config_file::read_config;
use report::{ProjectQualityReport, WorkspaceQualityReport, print_quality_report, quality_report};
pub(crate) use report::{QualityCheck, QualityReport, QualityReportKind, quality_check};
use std::path::Path;

const CHECK_DOCS_URL: &str = "https://docs.tovuk.com/agents";

pub(crate) fn check_project(project_dir: &Path, json_output: bool) -> Result<()> {
    let report = run_check_workspace(project_dir);
    if json_output {
        let value =
            serde_json::to_value(&report).map_err(|error| internal_error(error.to_string()))?;
        print_json(&value)?;
        if report.ok() {
            return Ok(());
        }
        return Err(check_failed_error(&report, true));
    }

    print_quality_report(&report);
    if !report.ok() {
        return Err(check_failed_error(&report, false));
    }
    Ok(())
}

fn check_failed_error(report: &QualityReportKind, json_output: bool) -> CliError {
    let instruction = first_failed_instruction(report)
        .unwrap_or_else(|| "Fix the failed checks and retry `tovuk check`.".to_owned());
    CliError::new(CliFailure {
        payload: AgentErrorPayload {
            code: "check_failed".to_owned(),
            message: "Tovuk check failed.".to_owned(),
            agent_instruction: Some(instruction),
            docs_url: Some(CHECK_DOCS_URL.to_owned()),
            checkout_url: None,
        },
        json: json_output,
        exit_code: 1,
    })
}

pub(crate) fn run_check_workspace(project_dir: &Path) -> QualityReportKind {
    if project_dir.join("tovuk.toml").exists() {
        return QualityReportKind::Project(Box::new(run_check(project_dir)));
    }
    let projects = discover_deploy_projects(project_dir).unwrap_or_default();
    if projects.is_empty() {
        return QualityReportKind::Project(Box::new(run_check(project_dir)));
    }
    let reports = projects
        .iter()
        .map(|project| {
            let report = run_check(&project.dir);
            ProjectQualityReport {
                relative: project.relative.clone(),
                ok: report.ok,
                project: report.project,
                config: report.config,
                checks: report.checks,
            }
        })
        .collect::<Vec<_>>();
    QualityReportKind::Workspace(WorkspaceQualityReport {
        ok: reports.iter().all(|report| report.ok),
        workspace: project_dir.display().to_string(),
        projects: reports,
    })
}

pub(crate) fn run_check(project_dir: &Path) -> QualityReport {
    let config_result = read_config(project_dir);
    let mut checks = vec![config_result.check];
    let kind = config_result
        .config
        .as_ref()
        .map_or(ProjectKind::RustWorker, |config| config.kind);

    if kind.is_fullstack() {
        if let Some(config) = config_result.config.as_ref() {
            checks.extend(fullstack_checks(project_dir, config, config_result.valid));
        }
        return quality_report(project_dir, config_result.config, checks);
    }

    checks.extend(required_file_checks(project_dir, kind));
    checks.extend(rust_quality_checks(project_dir, kind, config_result.valid));
    quality_report(project_dir, config_result.config, checks)
}

fn first_failed_instruction(report: &QualityReportKind) -> Option<String> {
    report
        .checks()
        .iter()
        .find(|check| !check.ok)
        .and_then(|check| check.agent_instruction.clone())
}

#[cfg(test)]
mod tests {
    use super::{QualityCheck, QualityReport, QualityReportKind, check_failed_error};

    #[test]
    fn check_failure_points_agents_to_repair_docs() -> Result<(), Box<dyn std::error::Error>> {
        let report = QualityReportKind::Project(Box::new(QualityReport {
            ok: false,
            project: ".".to_owned(),
            config: None,
            checks: vec![QualityCheck {
                name: "tovuk.toml".to_owned(),
                ok: false,
                message: "missing".to_owned(),
                agent_instruction: Some("Create and commit tovuk.toml, then retry.".to_owned()),
            }],
        }));
        let error = check_failed_error(&report, true);
        let payload = error.payload();
        if payload.code != "check_failed" {
            return Err(format!("unexpected error code: {}", payload.code).into());
        }
        if payload.docs_url.as_deref() != Some("https://docs.tovuk.com/agents") {
            return Err(format!("unexpected docs URL: {:?}", payload.docs_url).into());
        }
        if !payload
            .agent_instruction
            .as_deref()
            .is_some_and(|instruction| instruction.contains("tovuk.toml"))
        {
            return Err(format!(
                "unexpected agent instruction: {:?}",
                payload.agent_instruction
            )
            .into());
        }
        Ok(())
    }
}
