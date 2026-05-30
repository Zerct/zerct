mod cargo_lints;
mod checks;
mod config_file;
mod report;

use super::{
    deploy::discover_deploy_projects,
    errors::{
        AgentErrorPayload, CliError, CliFailure, Result, agent_error, internal_error, print_json,
    },
    project_kind::ProjectKind,
};
pub(crate) use checks::first_output_line;
use checks::{fullstack_checks, required_file_checks, rust_doctor_checks};
use config_file::read_config;
pub(crate) use report::{DoctorCheck, DoctorReport, DoctorReportKind, doctor_check};
use report::{ProjectDoctorReport, WorkspaceDoctorReport, doctor_report, print_doctor_report};
use std::path::Path;

pub(crate) fn doctor_project(project_dir: &Path, json_output: bool) -> Result<()> {
    let report = run_doctor_workspace(project_dir);
    if json_output {
        let value =
            serde_json::to_value(&report).map_err(|error| internal_error(error.to_string()))?;
        print_json(&value)?;
        if report.ok() {
            return Ok(());
        }
        return Err(CliError::new(CliFailure {
            payload: AgentErrorPayload {
                code: "doctor_failed".to_owned(),
                message: "Tovuk doctor failed.".to_owned(),
                agent_instruction: first_failed_instruction(&report),
                docs_url: None,
                checkout_url: None,
            },
            json: true,
            exit_code: 1,
        }));
    }

    print_doctor_report(&report);
    if !report.ok() {
        let instruction = first_failed_instruction(&report)
            .unwrap_or_else(|| "Fix the failed checks and retry `tovuk doctor`.".to_owned());
        return Err(agent_error(
            "doctor_failed",
            "Tovuk doctor failed.",
            instruction,
            false,
        ));
    }
    Ok(())
}

pub(crate) fn run_doctor_workspace(project_dir: &Path) -> DoctorReportKind {
    if project_dir.join("tovuk.toml").exists() {
        return DoctorReportKind::Project(Box::new(run_doctor(project_dir)));
    }
    let projects = discover_deploy_projects(project_dir).unwrap_or_default();
    if projects.is_empty() {
        return DoctorReportKind::Project(Box::new(run_doctor(project_dir)));
    }
    let reports = projects
        .iter()
        .map(|project| {
            let report = run_doctor(&project.dir);
            ProjectDoctorReport {
                relative: project.relative.clone(),
                ok: report.ok,
                project: report.project,
                config: report.config,
                checks: report.checks,
            }
        })
        .collect::<Vec<_>>();
    DoctorReportKind::Workspace(WorkspaceDoctorReport {
        ok: reports.iter().all(|report| report.ok),
        workspace: project_dir.display().to_string(),
        projects: reports,
    })
}

pub(crate) fn run_doctor(project_dir: &Path) -> DoctorReport {
    let config_result = read_config(project_dir);
    let mut checks = vec![config_result.check];
    let kind = config_result
        .config
        .as_ref()
        .map_or(ProjectKind::RustWorker, |config| config.kind);

    if kind.is_worker_static() {
        if let Some(config) = config_result.config.as_ref() {
            checks.extend(fullstack_checks(project_dir, config, config_result.valid));
        }
        return doctor_report(project_dir, config_result.config, checks);
    }

    checks.extend(required_file_checks(project_dir, kind));
    checks.extend(rust_doctor_checks(project_dir, kind, config_result.valid));
    doctor_report(project_dir, config_result.config, checks)
}

fn first_failed_instruction(report: &DoctorReportKind) -> Option<String> {
    report
        .checks()
        .iter()
        .find(|check| !check.ok)
        .and_then(|check| check.agent_instruction.clone())
}
