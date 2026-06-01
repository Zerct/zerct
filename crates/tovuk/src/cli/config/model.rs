use super::super::{project_kind::ProjectKind, resource_config::ResourceConfig};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct TovukConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    pub(crate) kind: ProjectKind,
    pub(crate) capabilities: CapabilitiesConfig,
    pub(crate) build: BuildConfig,
    pub(crate) run: RunConfig,
    pub(crate) frontend: FrontendConfig,
    #[serde(rename = "worker")]
    pub(crate) backend: BackendConfig,
    pub(crate) resources: ResourceConfig,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CapabilitiesConfig {
    pub(crate) static_frontend: CapabilityToggle,
    pub(crate) worker: CapabilityToggle,
    pub(crate) sqlite: CapabilityToggle,
    pub(crate) object_storage: CapabilityToggle,
    pub(crate) kv: CapabilityToggle,
    pub(crate) state: CapabilityToggle,
    pub(crate) queue: CapabilityToggle,
    pub(crate) cron: CapabilityToggle,
    pub(crate) service_bindings: CapabilityToggle,
    pub(crate) secrets: CapabilityToggle,
    pub(crate) custom_domains: CapabilityToggle,
    pub(crate) logs: CapabilityToggle,
    pub(crate) builds: CapabilityToggle,
    pub(crate) usage_caps: CapabilityToggle,
    pub(crate) billing: CapabilityToggle,
    pub(crate) support: CapabilityToggle,
    pub(crate) abuse: CapabilityToggle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct CapabilityToggle(bool);

impl CapabilityToggle {
    pub(crate) const fn from_bool(value: bool) -> Self {
        Self(value)
    }

    pub(crate) const fn enabled() -> Self {
        Self(true)
    }

    pub(crate) const fn is_enabled(self) -> bool {
        self.0
    }
}

impl CapabilitiesConfig {
    pub(crate) const KEYS: [&'static str; 17] = [
        "static_frontend",
        "worker",
        "sqlite",
        "object_storage",
        "kv",
        "state",
        "queue",
        "cron",
        "service_bindings",
        "secrets",
        "custom_domains",
        "logs",
        "builds",
        "usage_caps",
        "billing",
        "support",
        "abuse",
    ];

    pub(crate) fn for_kind(kind: ProjectKind) -> Self {
        Self {
            static_frontend: CapabilityToggle::from_bool(matches!(
                kind,
                ProjectKind::StaticFrontend | ProjectKind::Fullstack
            )),
            worker: CapabilityToggle::from_bool(matches!(
                kind,
                ProjectKind::RustWorker | ProjectKind::Fullstack
            )),
            sqlite: CapabilityToggle::from_bool(false),
            object_storage: CapabilityToggle::from_bool(false),
            kv: CapabilityToggle::from_bool(false),
            state: CapabilityToggle::from_bool(false),
            queue: CapabilityToggle::from_bool(false),
            cron: CapabilityToggle::from_bool(false),
            service_bindings: CapabilityToggle::from_bool(false),
            secrets: CapabilityToggle::from_bool(false),
            custom_domains: CapabilityToggle::from_bool(false),
            logs: CapabilityToggle::enabled(),
            builds: CapabilityToggle::enabled(),
            usage_caps: CapabilityToggle::enabled(),
            billing: CapabilityToggle::enabled(),
            support: CapabilityToggle::enabled(),
            abuse: CapabilityToggle::enabled(),
        }
    }

    pub(crate) fn enabled_keys(&self) -> Vec<&'static str> {
        self.filtered_keys(true)
    }

    pub(crate) fn disabled_keys(&self) -> Vec<&'static str> {
        self.filtered_keys(false)
    }

    pub(crate) fn enabled_product_keys(&self) -> Vec<&'static str> {
        self.enabled_keys()
            .into_iter()
            .filter(|key| !matches!(*key, "billing" | "support" | "abuse"))
            .collect()
    }

    fn filtered_keys(&self, expected: bool) -> Vec<&'static str> {
        Self::KEYS
            .iter()
            .copied()
            .filter(|key| self.value_for_key(key) == expected)
            .collect()
    }

    pub(crate) fn value_for_key(&self, key: &str) -> bool {
        match key {
            "static_frontend" => self.static_frontend.is_enabled(),
            "worker" => self.worker.is_enabled(),
            "sqlite" => self.sqlite.is_enabled(),
            "object_storage" => self.object_storage.is_enabled(),
            "kv" => self.kv.is_enabled(),
            "state" => self.state.is_enabled(),
            "queue" => self.queue.is_enabled(),
            "cron" => self.cron.is_enabled(),
            "service_bindings" => self.service_bindings.is_enabled(),
            "secrets" => self.secrets.is_enabled(),
            "custom_domains" => self.custom_domains.is_enabled(),
            "logs" => self.logs.is_enabled(),
            "builds" => self.builds.is_enabled(),
            "usage_caps" => self.usage_caps.is_enabled(),
            "billing" => self.billing.is_enabled(),
            "support" => self.support.is_enabled(),
            "abuse" => self.abuse.is_enabled(),
            _ => false,
        }
    }
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
        BackendConfig, BuildConfig, CapabilitiesConfig, FrontendConfig, RunConfig, TovukConfig,
    };

    #[test]
    fn serializes_fullstack_backend_as_worker() {
        let config = TovukConfig {
            name: Some("fullstack".to_owned()),
            kind: ProjectKind::Fullstack,
            capabilities: CapabilitiesConfig::for_kind(ProjectKind::Fullstack),
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
