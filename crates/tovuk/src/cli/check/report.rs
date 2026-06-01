use crate::cli::config::TovukConfig;
use serde::Serialize;
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct QualityCheck {
    pub(crate) name: String,
    pub(crate) ok: bool,
    pub(crate) message: String,
    pub(crate) agent_instruction: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct QualityReport {
    pub(crate) ok: bool,
    pub(crate) project: String,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) checks: Vec<QualityCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ProjectQualityReport {
    pub(crate) relative: String,
    pub(crate) ok: bool,
    pub(crate) project: String,
    pub(crate) config: Option<TovukConfig>,
    pub(crate) checks: Vec<QualityCheck>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WorkspaceQualityReport {
    pub(crate) ok: bool,
    pub(crate) workspace: String,
    pub(crate) projects: Vec<ProjectQualityReport>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum QualityReportKind {
    Project(Box<QualityReport>),
    Workspace(WorkspaceQualityReport),
}

impl QualityReportKind {
    pub(crate) fn ok(&self) -> bool {
        match self {
            Self::Project(report) => report.ok,
            Self::Workspace(report) => report.ok,
        }
    }

    pub(crate) fn checks(&self) -> Vec<QualityCheck> {
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

pub(super) fn quality_report(
    project_dir: &Path,
    config: Option<TovukConfig>,
    checks: Vec<QualityCheck>,
) -> QualityReport {
    QualityReport {
        ok: checks.iter().all(|check| check.ok),
        project: project_dir.display().to_string(),
        config,
        checks,
    }
}

pub(super) fn print_quality_report(report: &QualityReportKind) {
    match report {
        QualityReportKind::Project(report) => print_checks(&report.checks),
        QualityReportKind::Workspace(report) => {
            for project in &report.projects {
                println!("project {}", project.relative);
                print_checks(&project.checks);
            }
        }
    }
}

pub(super) fn print_checks(checks: &[QualityCheck]) {
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

pub(crate) fn quality_check(
    name: &str,
    ok: bool,
    success: &str,
    failure: &str,
    instruction: &str,
) -> QualityCheck {
    QualityCheck {
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
