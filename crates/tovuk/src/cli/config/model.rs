use super::super::{project_kind::ProjectKind, resource_config::ResourceConfig};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TovukConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    pub(crate) kind: ProjectKind,
    pub(crate) build: BuildConfig,
    pub(crate) run: RunConfig,
    pub(crate) frontend: FrontendConfig,
    pub(crate) backend: BackendConfig,
    pub(crate) resources: ResourceConfig,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct BuildConfig {
    pub(crate) command: String,
    pub(crate) check: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RunConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<String>,
    pub(crate) port: u16,
    pub(crate) health: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct FrontendConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub(crate) struct BackendConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) health: Option<String>,
}
