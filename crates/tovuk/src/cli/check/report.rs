use crate::cli::config::TovukConfig;
use serde::{Serialize, ser::SerializeStruct};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct QualityCheck {
    pub(crate) name: String,
    pub(crate) ok: bool,
    pub(crate) message: String,
    pub(crate) agent_instruction: Option<String>,
}

impl Serialize for QualityCheck {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("QualityCheck", 5)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("ok", &self.ok)?;
        state.serialize_field("status", quality_check_status(self.ok))?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("agent_instruction", &self.agent_instruction)?;
        state.end()
    }
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

fn quality_check_status(ok: bool) -> &'static str {
    if ok { "passed" } else { "failed" }
}

#[cfg(test)]
mod tests {
    use super::QualityCheck;

    #[test]
    fn quality_check_json_includes_status() -> Result<(), Box<dyn std::error::Error>> {
        let value = serde_json::to_value(QualityCheck {
            name: "npm run typecheck".to_owned(),
            ok: false,
            message: "missing types".to_owned(),
            agent_instruction: Some("Run npm install, then retry.".to_owned()),
        })?;

        if value["ok"] != false {
            return Err(format!("unexpected ok value: {}", value["ok"]).into());
        }
        if value["status"] != "failed" {
            return Err(format!("unexpected status: {}", value["status"]).into());
        }
        if value["name"] != "npm run typecheck" {
            return Err(format!("unexpected name: {}", value["name"]).into());
        }
        Ok(())
    }
}
