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
    #[serde(rename = "worker")]
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        super::super::{project_kind::ProjectKind, resource_config::ResourceConfig},
        BackendConfig, BuildConfig, FrontendConfig, RunConfig, TovukConfig,
    };

    #[test]
    fn serializes_worker_static_backend_as_worker() {
        let config = TovukConfig {
            name: Some("fullstack".to_owned()),
            kind: ProjectKind::WorkerStatic,
            build: BuildConfig {
                command: "cargo build --release".to_owned(),
                check: "cargo fmt --all --check".to_owned(),
                output: None,
            },
            run: RunConfig {
                command: None,
                port: 3000,
                health: "/healthz".to_owned(),
            },
            frontend: FrontendConfig {
                root: Some("web".to_owned()),
                check: Some("bun ci && bun run typecheck && bun run lint".to_owned()),
                build: Some("bun run build".to_owned()),
                output: Some("dist".to_owned()),
            },
            backend: BackendConfig {
                root: Some("api".to_owned()),
                check: Some("cargo fmt --all --check".to_owned()),
                build: Some("cargo build --release".to_owned()),
                command: Some("./target/release/api".to_owned()),
                port: Some(3000),
                health: Some("/api/healthz".to_owned()),
            },
            resources: ResourceConfig {
                memory: "128mb".to_owned(),
                cpu: "1".to_owned(),
                idle_timeout_minutes: 15,
            },
        };

        let value = serde_json::to_value(config)
            .unwrap_or_else(|error| json!({ "serialization_error": error.to_string() }));

        assert_eq!(value["worker"]["root"], json!("api"));
        assert_eq!(value.get("backend"), None);
    }
}
