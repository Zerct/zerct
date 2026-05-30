use crate::cli::config::TovukConfig;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DoctorCheck {
    pub(crate) name: String,
    pub(crate) ok: bool,
    pub(crate) message: String,
    pub(crate) agent_instruction: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DoctorReport {
    pub(crate) ok: bool,
    pub(crate) project: String,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProjectDoctorReport {
    pub(crate) relative: String,
    pub(crate) ok: bool,
    pub(crate) project: String,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) checks: Vec<DoctorCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WorkspaceDoctorReport {
    pub(crate) ok: bool,
    pub(crate) workspace: String,
    pub(crate) projects: Vec<ProjectDoctorReport>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum DoctorReportKind {
    Project(Box<DoctorReport>),
    Workspace(WorkspaceDoctorReport),
}

impl DoctorReportKind {
    pub(crate) fn ok(&self) -> bool {
        match self {
            Self::Project(report) => report.ok,
            Self::Workspace(report) => report.ok,
        }
    }

    pub(crate) fn checks(&self) -> Vec<DoctorCheck> {
        match self {
            Self::Project(report) => report.checks.clone(),
            Self::Workspace(report) => report
                .projects
                .iter()
                .flat_map(|project| project.checks.clone())
                .collect(),
        }
    }
}

pub(super) fn doctor_report(
    project_dir: &Path,
    config: Option<TovukConfig>,
    checks: Vec<DoctorCheck>,
) -> DoctorReport {
    DoctorReport {
        ok: checks.iter().all(|check| check.ok),
        project: project_dir.display().to_string(),
        config,
        checks,
    }
}

pub(super) fn print_doctor_report(report: &DoctorReportKind) {
    match report {
        DoctorReportKind::Project(report) => print_checks(&report.checks),
        DoctorReportKind::Workspace(report) => {
            for project in &report.projects {
                println!("project {}", project.relative);
                print_checks(&project.checks);
            }
        }
    }
}

pub(super) fn print_checks(checks: &[DoctorCheck]) {
    for check in checks {
        println!(
            "{} {}{}",
            if check.ok { "ok" } else { "fail" },
            check.name,
            if check.message.is_empty() {
                String::new()
            } else {
                format!(" - {}", check.message)
            }
        );
    }
}

pub(crate) fn doctor_check(
    name: &str,
    ok: bool,
    success: &str,
    failure: &str,
    instruction: &str,
) -> DoctorCheck {
    DoctorCheck {
        name: name.to_owned(),
        ok,
        message: if ok { success } else { failure }.to_owned(),
        agent_instruction: if ok {
            None
        } else {
            Some(instruction.to_owned())
        },
    }
}
