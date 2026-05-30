use super::super::project_kind::ProjectKind;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct DeployProjectInfo {
    pub(crate) dir: PathBuf,
    pub(crate) relative: String,
    pub(crate) name: String,
    pub(crate) kind: Option<ProjectKind>,
}

#[derive(Clone, Debug)]
pub(crate) struct DeployPlanProject {
    pub(crate) project: DeployProjectInfo,
}

pub(crate) struct WorkspaceDeployResult {
    pub(crate) project: DeployProjectInfo,
    pub(crate) response: Value,
    pub(crate) final_build: Option<Value>,
}
