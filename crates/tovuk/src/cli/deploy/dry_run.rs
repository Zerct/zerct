use super::{
    super::{
        api_commands::api_request,
        args::CliOptions,
        check::run_check,
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
    let missing_config = report
        .checks
        .iter()
        .filter(|check| !check.ok)
        .map(|check| {
            json!({
                "check": check.name,
                "message": check.message,
                "agent_instruction": check.agent_instruction,
            })
        })
        .collect::<Vec<_>>();

    json!({
        "relative": project.relative,
        "path": project.dir.display().to_string(),
        "serviceName": service_name,
        "exists": !service_name.is_empty() && existing_service_names.contains(&service_name),
        "kind": kind.map(ProjectKind::as_str),
        "config": config,
        "capabilities": capability_dry_run(kind, capabilities),
        "check": {
            "ok": report.ok,
            "checks": report.checks,
        },
        "missingConfig": missing_config,
        "meters": meters_for_kind(kind, capabilities),
        "nextAgentActions": project_next_actions(report.ok, kind),
    })
}

fn capability_dry_run(kind: Option<ProjectKind>, capabilities: &Value) -> Value {
    let enabled = enabled_capabilities(kind);
    let all = all_service_capabilities(capabilities);
    let disabled = all
        .iter()
        .filter(|capability| {
            !enabled
                .iter()
                .any(|enabled_capability| enabled_capability == &capability.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();

    json!({
        "enabled": enabled,
        "disabled": disabled,
        "source": "tovuk.toml kind and explicit platform resource commands",
        "warning": "Use platform commands to create optional service resources before code depends on them.",
    })
}

fn enabled_capabilities(kind: Option<ProjectKind>) -> Vec<&'static str> {
    let Some(kind) = kind else {
        return Vec::new();
    };
    let mut enabled = enabled_product_keys(kind);
    enabled.extend(["billing", "support", "abuse"]);
    enabled.sort_unstable();
    enabled
}

fn enabled_product_keys(kind: ProjectKind) -> Vec<&'static str> {
    let mut enabled = vec!["builds", "logs", "secrets", "custom_domains", "usage_caps"];
    if matches!(kind, ProjectKind::RustWorker | ProjectKind::WorkerStatic) {
        enabled.push("worker");
        enabled.extend([
            "sqlite",
            "object_storage",
            "kv",
            "state",
            "queue",
            "cron",
            "service_bindings",
        ]);
    }
    if matches!(
        kind,
        ProjectKind::StaticFrontend | ProjectKind::WorkerStatic
    ) {
        enabled.push("static_frontend");
    }
    enabled.sort_unstable();
    enabled
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

fn meters_for_kind(kind: Option<ProjectKind>, capabilities: &Value) -> Vec<String> {
    let Some(kind) = kind else {
        return Vec::new();
    };
    let enabled = enabled_product_keys(kind);
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
        Some(ProjectKind::WorkerStatic) => vec![
            "Review enabled worker-static capabilities and billingEstimate.lineItems.",
            "Set usage caps for worker, static transfer, SQLite, KV, queues, State, and object storage meters before load.",
            "Run `tovuk deploy --wait --json` only after this dry run is acceptable.",
        ],
        Some(ProjectKind::RustWorker) => vec![
            "Review Rust Worker meters and billingEstimate.lineItems.",
            "Set usage caps for worker and optional platform resource meters before load.",
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

    use super::{ProjectKind, capability_dry_run, meters_for_kind, workspace_warnings};

    #[test]
    fn worker_static_dry_run_exposes_one_service_capability_set() {
        let capabilities =
            capability_dry_run(Some(ProjectKind::WorkerStatic), &sample_capabilities());

        assert!(capabilities["enabled"].as_array().is_some_and(|enabled| {
            enabled.iter().any(|value| value == "worker")
                && enabled.iter().any(|value| value == "static_frontend")
                && enabled.iter().any(|value| value == "sqlite")
                && enabled.iter().any(|value| value == "object_storage")
                && enabled.iter().any(|value| value == "usage_caps")
        }));
    }

    #[test]
    fn worker_static_dry_run_lists_public_meters_before_deploy() {
        let meters = meters_for_kind(Some(ProjectKind::WorkerStatic), &sample_capabilities());

        assert!(meters.contains(&"worker_cpu_ms".to_owned()));
        assert!(meters.contains(&"static_transfer_bytes".to_owned()));
        assert!(meters.contains(&"sqlite_rows_read".to_owned()));
        assert!(meters.contains(&"kv_reads".to_owned()));
        assert!(meters.contains(&"queue_operations".to_owned()));
        assert!(meters.contains(&"state_requests".to_owned()));
        assert!(meters.contains(&"object_storage_egress_bytes".to_owned()));
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

    fn sample_capabilities() -> serde_json::Value {
        json!({
            "products": [
                { "key": "worker", "meters": ["worker_requests", "worker_cpu_ms"] },
                { "key": "static_frontend", "meters": ["static_transfer_bytes"] },
                { "key": "sqlite", "meters": ["sqlite_rows_read"] },
                { "key": "object_storage", "meters": ["object_storage_egress_bytes"] },
                { "key": "kv", "meters": ["kv_reads"] },
                { "key": "queue", "meters": ["queue_operations"] },
                { "key": "state", "meters": ["state_requests"] },
                { "key": "cron", "meters": ["worker_cpu_ms"] },
                { "key": "service_bindings", "meters": ["worker_cpu_ms"] },
                { "key": "builds", "meters": ["build_minutes"] },
                { "key": "logs", "meters": ["log_events"] },
                { "key": "secrets", "meters": [] },
                { "key": "custom_domains", "meters": [] },
                { "key": "usage_caps", "meters": [] }
            ]
        })
    }
}
