use super::{
    super::{
        api_commands::api_request,
        args::CliOptions,
        check::{QualityCheck, run_check},
        config::CapabilitiesConfig,
        errors::{Result, print_json},
        project::number_field,
        project_kind::ProjectKind,
    },
    types::DeployProjectInfo,
};
use reqwest::Method;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

pub(super) fn print_deploy_dry_run(
    project_dir: &Path,
    projects: &[DeployProjectInfo],
    cli: &CliOptions,
    token: &str,
) -> Result<()> {
    let capabilities = api_request(cli, Method::GET, "/v1/capabilities", None, None)?;
    let usage = api_request(cli, Method::GET, "/v1/usage", Some(token), None)?;
    let services = api_request(cli, Method::GET, "/v1/services", Some(token), None)?;
    let existing_service_names = service_name_set(&services);
    let service_dry_runs = projects
        .iter()
        .map(|project| service_dry_run(project, &existing_service_names, &capabilities))
        .collect::<Vec<_>>();
    let warnings = workspace_warnings(&service_dry_runs, &usage);
    let ok = warnings.is_empty()
        && service_dry_runs
            .iter()
            .all(|project| project["check"]["ok"].as_bool().unwrap_or(false));

    print_json(&json!({
        "ok": ok,
        "mode": "dry_run",
        "dryRun": true,
        "sourceOfTruth": "tovuk.toml",
        "deployBehavior": "read_only_no_upload_no_build",
        "workspace": project_dir.display().to_string(),
        "services": service_dry_runs,
        "warnings": warnings,
        "capabilityCatalog": capability_catalog(&capabilities),
        "limits": usage.get("limits").cloned().unwrap_or(Value::Null),
        "usage": usage.get("usage").cloned().unwrap_or(Value::Null),
        "billingEstimate": usage.get("billingEstimate").cloned().unwrap_or(Value::Null),
        "nextActions": next_actions(ok),
    }))
}

fn service_dry_run(
    project: &DeployProjectInfo,
    existing_service_names: &BTreeSet<String>,
    capabilities: &Value,
) -> Value {
    let report = run_check(&project.dir);
    let config = report.config.as_ref();
    let kind = config
        .as_ref()
        .map_or(project.kind, |config| Some(config.kind));
    let service_name = config
        .and_then(|config| config.name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| project.name.clone());
    let required_fixes = required_fix_entries(&report.checks);
    let missing_config = missing_config_entries(&required_fixes);

    json!({
        "relative": project.relative,
        "path": project.dir.display().to_string(),
        "serviceName": service_name,
        "exists": !service_name.is_empty() && existing_service_names.contains(&service_name),
        "kind": kind.map(ProjectKind::as_str),
        "config": config,
        "capabilities": capability_dry_run(config.map(|config| &config.capabilities), capabilities),
        "check": {
            "ok": report.ok,
            "checks": report.checks,
        },
        "missingConfig": missing_config,
        "requiredFixes": required_fixes,
        "meters": meters_for_capabilities(config.map(|config| &config.capabilities), capabilities),
        "meterPlan": meter_plan_for_capabilities(config.map(|config| &config.capabilities), capabilities),
        "nextAgentActions": project_next_actions(report.ok, kind),
    })
}

fn required_fix_entries(checks: &[QualityCheck]) -> Vec<Value> {
    checks
        .iter()
        .filter(|check| !check.ok)
        .map(|check| {
            json!({
                "check": check.name,
                "message": check.message,
                "agent_instruction": check.agent_instruction,
            })
        })
        .collect()
}

fn missing_config_entries(required_fixes: &[Value]) -> Vec<Value> {
    required_fixes
        .iter()
        .filter(|entry| entry["check"].as_str() == Some("tovuk.toml"))
        .cloned()
        .collect()
}

fn capability_dry_run(config: Option<&CapabilitiesConfig>, capabilities: &Value) -> Value {
    let Some(config) = config else {
        return json!({
            "enabled": [],
            "disabled": all_service_capabilities(capabilities),
            "source": "tovuk.toml [capabilities]",
            "warning": "Fix tovuk.toml before Tovuk can identify enabled capabilities.",
        });
    };

    json!({
        "enabled": config.enabled_keys(),
        "disabled": config.disabled_keys(),
        "source": "tovuk.toml [capabilities]",
        "warning": "Enable optional capabilities in tovuk.toml before code depends on them, then create the matching resource through CLI or API.",
    })
}

fn all_service_capabilities(capabilities: &Value) -> Vec<String> {
    let mut all = product_keys(capabilities);
    all.extend([
        "billing".to_owned(),
        "support".to_owned(),
        "abuse".to_owned(),
    ]);
    all.sort_unstable();
    all.dedup();
    all
}

fn meters_for_capabilities(
    config: Option<&CapabilitiesConfig>,
    capabilities: &Value,
) -> Vec<String> {
    let Some(config) = config else {
        return Vec::new();
    };
    let enabled = enabled_meter_product_keys(config);
    let mut meters = capabilities
        .get("products")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|product| {
            product
                .get("key")
                .and_then(Value::as_str)
                .is_some_and(|key| enabled.contains(&key))
        })
        .flat_map(product_meters)
        .collect::<Vec<_>>();
    meters.sort_unstable();
    meters.dedup();
    meters
}

fn product_keys(capabilities: &Value) -> Vec<String> {
    capabilities
        .get("products")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|product| product.get("key").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn product_meters(product: &Value) -> Vec<String> {
    product
        .get("meters")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn meter_plan_for_capabilities(
    config: Option<&CapabilitiesConfig>,
    capabilities: &Value,
) -> Vec<Value> {
    let Some(config) = config else {
        return Vec::new();
    };
    let enabled = enabled_meter_product_keys(config);
    let mut meter_plan = capabilities
        .get("products")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|product| {
            product
                .get("key")
                .and_then(Value::as_str)
                .is_some_and(|key| enabled.contains(&key))
        })
        .flat_map(product_meter_plan)
        .collect::<Vec<_>>();
    meter_plan.sort_by(|left, right| {
        let left_key = left["meter"].as_str().unwrap_or_default();
        let right_key = right["meter"].as_str().unwrap_or_default();
        left_key.cmp(right_key)
    });
    meter_plan.dedup_by(|left, right| left["meter"] == right["meter"]);
    meter_plan
}

fn enabled_meter_product_keys(config: &CapabilitiesConfig) -> Vec<&'static str> {
    config
        .enabled_product_keys()
        .into_iter()
        .filter(|key| *key != "usage_caps")
        .collect()
}

fn product_meter_plan(product: &Value) -> Vec<Value> {
    let product_key = product
        .get("key")
        .and_then(Value::as_str)
        .unwrap_or_default();
    product_meters(product)
        .into_iter()
        .map(|meter| {
            let details = product_meter_details(product, &meter);
            json!({
                "meter": meter,
                "product": product_key,
                "unit": details.get("unit").cloned().unwrap_or(Value::Null),
                "description": details.get("description").cloned().unwrap_or(Value::Null),
                "limitFields": product.get("limit_fields").cloned().unwrap_or(Value::Null),
                "pricingFields": product.get("pricing_fields").cloned().unwrap_or(Value::Null),
                "capCommands": {
                    "day": format!("tovuk limits set {meter} --period day --value <value> --notify-at-percent 80 --json"),
                    "month": format!("tovuk limits set {meter} --period month --value <value> --notify-at-percent 80 --json"),
                },
            })
        })
        .collect()
}

fn product_meter_details<'a>(product: &'a Value, meter: &str) -> &'a Value {
    product
        .get("meter_details")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|details| {
            details
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name == meter)
        })
        .unwrap_or(&Value::Null)
}

fn service_name_set(response: &Value) -> BTreeSet<String> {
    response
        .get("services")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|service| service.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn workspace_warnings(projects: &[Value], usage: &Value) -> Vec<String> {
    let requested_new_projects = projects
        .iter()
        .filter(|project| {
            !project["exists"].as_bool().unwrap_or(false)
                && project["serviceName"]
                    .as_str()
                    .is_some_and(|name| !name.is_empty())
        })
        .count() as u64;
    let usage_totals = usage.get("usage").unwrap_or(&Value::Null);
    let limits = usage.get("limits").unwrap_or(&Value::Null);
    let used_projects = number_field(usage_totals, "serviceCount");
    let project_limit = number_field(limits, "projects");
    let mut warnings = Vec::new();
    if project_limit > 0
        && requested_new_projects > 0
        && used_projects + requested_new_projects > project_limit
    {
        warnings.push(format!(
            "Project limit would be exceeded: {used_projects}/{project_limit} used and {requested_new_projects} new requested."
        ));
    }
    warnings
}

fn capability_catalog(capabilities: &Value) -> Value {
    let products = capabilities
        .get("products")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|product| {
            json!({
                "key": product.get("key").cloned().unwrap_or(Value::Null),
                "meters": product.get("meters").cloned().unwrap_or(Value::Null),
                "meterDetails": product.get("meter_details").cloned().unwrap_or(Value::Null),
                "limitFields": product.get("limit_fields").cloned().unwrap_or(Value::Null),
                "pricingFields": product.get("pricing_fields").cloned().unwrap_or(Value::Null),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "docsUrl": capabilities.get("docs_url").cloned().unwrap_or(Value::Null),
        "openapiUrl": capabilities.get("openapi_url").cloned().unwrap_or(Value::Null),
        "products": products,
    })
}

fn project_next_actions(ok: bool, kind: Option<ProjectKind>) -> Vec<&'static str> {
    if !ok {
        return vec![
            "Fix the first failed quality check, then rerun `tovuk deploy --dry-run --json`.",
        ];
    }
    match kind {
        Some(ProjectKind::Fullstack) => vec![
            "Review enabled full-stack capabilities and billingEstimate.lineItems.",
            "Set usage caps for worker, static transfer, SQLite, KV, queues, State, and object storage meters before load.",
            "Run `tovuk deploy --wait --json` only after this dry run is acceptable.",
        ],
        Some(ProjectKind::RustWorker) => vec![
            "Review Rust Worker meters and billingEstimate.lineItems.",
            "Set usage caps for worker and optional service resource meters before load.",
            "Run `tovuk deploy --wait --json` only after this dry run is acceptable.",
        ],
        Some(ProjectKind::StaticFrontend) => vec![
            "Review static frontend checks and billingEstimate.lineItems.",
            "Set static transfer usage caps before high traffic.",
            "Run `tovuk deploy --wait --json` only after this dry run is acceptable.",
        ],
        None => vec!["Fix tovuk.toml so Tovuk can identify the service kind."],
    }
}

fn next_actions(ok: bool) -> Vec<&'static str> {
    if ok {
        return vec![
            "Review billingEstimate.lineItems and warnings.",
            "Set hard usage caps for expected load.",
            "Run `tovuk deploy --wait --json`.",
        ];
    }
    vec![
        "Fix warnings and failed quality checks.",
        "Rerun `tovuk deploy --dry-run --json` before deploy.",
        "Use `tovuk billing checkout --json` if a limit blocks the plan.",
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::cli::config::CapabilityToggle;

    use super::{
        CapabilitiesConfig, ProjectKind, QualityCheck, capability_dry_run,
        meter_plan_for_capabilities, meters_for_capabilities, missing_config_entries,
        required_fix_entries, workspace_warnings,
    };
    #[test]
    fn fullstack_dry_run_exposes_one_service_capability_set() {
        let service_capabilities = fullstack_capabilities();
        let capabilities = capability_dry_run(Some(&service_capabilities), &sample_capabilities());

        assert!(capabilities["enabled"].as_array().is_some_and(|enabled| {
            enabled.iter().any(|value| value == "worker")
                && enabled.iter().any(|value| value == "static_frontend")
                && enabled.iter().any(|value| value == "usage_caps")
        }));
        assert!(
            capabilities["disabled"]
                .as_array()
                .is_some_and(|disabled| disabled.iter().any(|value| value == "sqlite"))
        );
    }

    #[test]
    fn fullstack_dry_run_lists_public_meters_before_deploy() {
        let mut service_capabilities = fullstack_capabilities();
        service_capabilities.sqlite = CapabilityToggle::enabled();
        service_capabilities.object_storage = CapabilityToggle::enabled();
        service_capabilities.kv = CapabilityToggle::enabled();
        service_capabilities.queue = CapabilityToggle::enabled();
        service_capabilities.state = CapabilityToggle::enabled();
        let meters = meters_for_capabilities(Some(&service_capabilities), &sample_capabilities());

        assert!(meters.contains(&"worker_cpu_ms".to_owned()));
        assert!(meters.contains(&"static_transfer_bytes".to_owned()));
        assert!(meters.contains(&"sqlite_rows_read".to_owned()));
        assert!(meters.contains(&"kv_reads".to_owned()));
        assert!(meters.contains(&"queue_operations".to_owned()));
        assert!(meters.contains(&"state_requests".to_owned()));
        assert!(meters.contains(&"object_storage_bytes".to_owned()));
        assert!(meters.contains(&"object_storage_class_a_operations".to_owned()));
        assert!(meters.contains(&"object_storage_class_b_operations".to_owned()));
        assert!(meters.contains(&"object_storage_egress_bytes".to_owned()));
    }

    #[test]
    fn usage_caps_catalog_does_not_leak_disabled_resource_meters() {
        let service_capabilities = fullstack_capabilities();
        let catalog = sample_capabilities();
        let meters = meters_for_capabilities(Some(&service_capabilities), &catalog);
        let plan = meter_plan_for_capabilities(Some(&service_capabilities), &catalog);

        assert!(meters.contains(&"worker_requests".to_owned()));
        assert!(meters.contains(&"static_transfer_bytes".to_owned()));
        assert!(!meters.contains(&"sqlite_rows_read".to_owned()));
        assert!(!meters.contains(&"object_storage_class_a_operations".to_owned()));
        assert!(!meters.contains(&"object_storage_egress_bytes".to_owned()));
        assert!(
            !plan
                .iter()
                .any(|entry| entry["meter"] == "sqlite_rows_read")
        );
        assert!(
            !plan
                .iter()
                .any(|entry| entry["meter"] == "object_storage_class_a_operations")
        );
        assert!(
            !plan
                .iter()
                .any(|entry| entry["meter"] == "object_storage_egress_bytes")
        );
    }

    #[test]
    fn fullstack_dry_run_exposes_meter_plan_for_usage_caps() {
        let mut service_capabilities = fullstack_capabilities();
        service_capabilities.object_storage = CapabilityToggle::enabled();
        let plan = meter_plan_for_capabilities(Some(&service_capabilities), &sample_capabilities());

        let missing_meter = json!({});
        let object_storage_class_a = plan
            .iter()
            .find(|entry| entry["meter"] == "object_storage_class_a_operations")
            .unwrap_or(&missing_meter);
        assert_eq!(
            object_storage_class_a["capCommands"]["month"],
            "tovuk limits set object_storage_class_a_operations --period month --value <value> --notify-at-percent 80 --json",
        );
        assert!(
            object_storage_class_a["pricingFields"]
                .as_array()
                .is_some_and(|fields| {
                    fields.iter().any(|field| {
                        field == "pricing.objectStorage.classAOverageUsdMicrosPerMillion"
                    })
                })
        );
        let object_storage_egress = plan
            .iter()
            .find(|entry| entry["meter"] == "object_storage_egress_bytes")
            .unwrap_or(&missing_meter);
        assert_eq!(
            object_storage_egress["meter"],
            "object_storage_egress_bytes"
        );
        assert_eq!(object_storage_egress["product"], "object_storage");
        assert_eq!(object_storage_egress["unit"], "byte");
        assert_eq!(
            object_storage_egress["capCommands"]["month"],
            "tovuk limits set object_storage_egress_bytes --period month --value <value> --notify-at-percent 80 --json",
        );
        assert!(
            object_storage_egress["pricingFields"]
                .as_array()
                .is_some_and(|fields| fields
                    .iter()
                    .any(|field| field == "pricing.objectStorage.egressOverageUsdMicrosPerTb"))
        );
        assert!(
            object_storage_egress["limitFields"]
                .as_array()
                .is_some_and(|fields| fields
                    .iter()
                    .any(|field| field == "objectStorageEgressBytesPerMonth"))
        );
    }

    #[test]
    fn workspace_dry_run_warns_before_project_limit_overflow() {
        let projects = vec![json!({
            "exists": false,
            "serviceName": "new-service",
        })];
        let usage = json!({
            "usage": { "serviceCount": 100 },
            "limits": { "projects": 100 },
        });

        let warnings = workspace_warnings(&projects, &usage);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Project limit would be exceeded"));
    }

    #[test]
    fn dry_run_splits_missing_config_from_required_quality_fixes() {
        let required_fixes = required_fix_entries(&[
            QualityCheck {
                name: "tovuk.toml".to_owned(),
                ok: false,
                message: "missing".to_owned(),
                agent_instruction: Some("Create and commit tovuk.toml, then retry.".to_owned()),
            },
            QualityCheck {
                name: "cargo clippy".to_owned(),
                ok: false,
                message: "strict Clippy failed".to_owned(),
                agent_instruction: Some("Fix Clippy findings, then retry.".to_owned()),
            },
        ]);
        let missing_config = missing_config_entries(&required_fixes);

        assert_eq!(required_fixes.len(), 2);
        assert_eq!(missing_config.len(), 1);
        assert_eq!(missing_config[0]["check"], "tovuk.toml");
    }

    fn fullstack_capabilities() -> CapabilitiesConfig {
        CapabilitiesConfig::for_kind(ProjectKind::Fullstack)
    }

    fn sample_capabilities() -> serde_json::Value {
        json!({
            "products": [
                worker_product(),
                static_frontend_product(),
                sqlite_product(),
                object_storage_product(),
                kv_product(),
                queue_product(),
                state_product(),
                { "key": "cron", "meters": ["worker_cpu_ms"], "meter_details": [], "pricing_fields": [], "limit_fields": [] },
                { "key": "service_bindings", "meters": ["worker_cpu_ms"], "meter_details": [], "pricing_fields": [], "limit_fields": [] },
                { "key": "builds", "meters": ["build_minutes"], "meter_details": [], "pricing_fields": [], "limit_fields": [] },
                { "key": "logs", "meters": ["log_events"], "meter_details": [], "pricing_fields": [], "limit_fields": [] },
                { "key": "secrets", "meters": [], "meter_details": [], "pricing_fields": [], "limit_fields": [] },
                { "key": "custom_domains", "meters": [], "meter_details": [], "pricing_fields": [], "limit_fields": [] },
                usage_caps_product()
            ]
        })
    }

    fn worker_product() -> serde_json::Value {
        json!({
            "key": "worker",
            "meters": ["worker_requests", "worker_cpu_ms"],
            "meter_details": [
                { "name": "worker_requests", "unit": "request", "description": "Worker requests." },
                { "name": "worker_cpu_ms", "unit": "cpu_ms", "description": "Worker CPU milliseconds." }
            ],
            "pricing_fields": ["pricing.workers.requestOverageUsdMicrosPerMillion"],
            "limit_fields": ["workerRequestsPerDay"]
        })
    }

    fn static_frontend_product() -> serde_json::Value {
        json!({
            "key": "static_frontend",
            "meters": ["static_transfer_bytes"],
            "meter_details": [
                { "name": "static_transfer_bytes", "unit": "byte", "description": "Static transfer bytes." }
            ],
            "pricing_fields": ["pricing.staticAssets.storageFree"],
            "limit_fields": ["staticAssetFilesPerVersion"]
        })
    }

    fn sqlite_product() -> serde_json::Value {
        json!({
            "key": "sqlite",
            "meters": ["sqlite_rows_read"],
            "meter_details": [
                { "name": "sqlite_rows_read", "unit": "row", "description": "SQLite rows read." }
            ],
            "pricing_fields": ["pricing.sqlite.rowsReadOverageUsdMicrosPerMillion"],
            "limit_fields": ["sqliteRowsReadPerDay"]
        })
    }

    fn object_storage_product() -> serde_json::Value {
        json!({
            "key": "object_storage",
            "meters": [
                "object_storage_bytes",
                "object_storage_class_a_operations",
                "object_storage_class_b_operations",
                "object_storage_egress_bytes"
            ],
            "meter_details": [
                { "name": "object_storage_bytes", "unit": "byte", "description": "Object storage bytes." },
                { "name": "object_storage_class_a_operations", "unit": "operation", "description": "Object storage Class A operations." },
                { "name": "object_storage_class_b_operations", "unit": "operation", "description": "Object storage Class B operations." },
                { "name": "object_storage_egress_bytes", "unit": "byte", "description": "Object storage egress bytes." }
            ],
            "pricing_fields": [
                "pricing.objectStorage.storageOverageUsdMicrosPerGbMonth",
                "pricing.objectStorage.classAOverageUsdMicrosPerMillion",
                "pricing.objectStorage.classBOverageUsdMicrosPerMillion",
                "pricing.objectStorage.egressOverageUsdMicrosPerTb"
            ],
            "limit_fields": [
                "objectStorageMib",
                "objectStorageClassAOperationsPerMonth",
                "objectStorageClassBOperationsPerMonth",
                "objectStorageEgressBytesPerMonth"
            ]
        })
    }

    fn kv_product() -> serde_json::Value {
        json!({
            "key": "kv",
            "meters": ["kv_reads"],
            "meter_details": [
                { "name": "kv_reads", "unit": "read", "description": "KV reads." }
            ],
            "pricing_fields": ["pricing.kv.readOverageUsdMicrosPerMillion"],
            "limit_fields": ["kvReadsPerDay"]
        })
    }

    fn queue_product() -> serde_json::Value {
        json!({
            "key": "queue",
            "meters": ["queue_operations"],
            "meter_details": [
                { "name": "queue_operations", "unit": "operation", "description": "Queue operations." }
            ],
            "pricing_fields": ["pricing.queues.operationOverageUsdMicrosPerMillion"],
            "limit_fields": ["queueOperationsPerDay"]
        })
    }

    fn state_product() -> serde_json::Value {
        json!({
            "key": "state",
            "meters": ["state_requests"],
            "meter_details": [
                { "name": "state_requests", "unit": "request", "description": "State requests." }
            ],
            "pricing_fields": ["pricing.state.requestOverageUsdMicrosPerMillion"],
            "limit_fields": ["stateRequestsPerDay"]
        })
    }

    fn usage_caps_product() -> serde_json::Value {
        json!({
            "key": "usage_caps",
            "meters": [
                "worker_requests",
                "static_transfer_bytes",
                "sqlite_rows_read",
                "object_storage_bytes",
                "object_storage_class_a_operations",
                "object_storage_class_b_operations",
                "object_storage_egress_bytes"
            ],
            "meter_details": [
                { "name": "worker_requests", "unit": "request", "description": "Worker requests." },
                { "name": "static_transfer_bytes", "unit": "byte", "description": "Static transfer bytes." },
                { "name": "sqlite_rows_read", "unit": "row", "description": "SQLite rows read." },
                { "name": "object_storage_bytes", "unit": "byte", "description": "Object storage bytes." },
                { "name": "object_storage_class_a_operations", "unit": "operation", "description": "Object storage Class A operations." },
                { "name": "object_storage_class_b_operations", "unit": "operation", "description": "Object storage Class B operations." },
                { "name": "object_storage_egress_bytes", "unit": "byte", "description": "Object storage egress bytes." }
            ],
            "pricing_fields": [
                "pricing.subscriptionUsdCentsPerMonth",
                "pricing.paidOveragesEnabled"
            ],
            "limit_fields": ["apiTokens", "supportTicketsPerDay"]
        })
    }
}
