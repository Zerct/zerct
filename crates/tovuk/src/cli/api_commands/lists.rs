use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    config::CapabilitiesConfig,
    errors::{Result, agent_error},
    project::{encode_component, nested_string, number_field, string_field},
};
use super::{common::service_route, generic::print_authenticated_mutation, http::api_request};
use reqwest::Method;
use serde_json::{Value, json};

pub(crate) fn service_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => service_list(cli),
        "show" => service_show(cli),
        "status" => service_status(cli),
        "delete" => {
            let service = cli.args.get(1).cloned().filter(|value| !value.is_empty());
            let route = if let Some(service) = service {
                format!("/v1/services/{}", encode_component(&service))
            } else {
                service_route(cli, "")?
            };
            print_authenticated_mutation(cli, Method::DELETE, &route, None)
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown service command.",
            "Use `tovuk service list --json`, `tovuk service status <service> --json`, `tovuk service show <service> --json`, or `tovuk service delete <service> --json`.",
            cli.output.json,
        )),
    }
}

fn service_list(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, "/v1/services", Some(&token), None)?;
    if cli.output.json {
        return super::super::errors::print_json(&response);
    }
    for line in service_list_lines(&response) {
        println!("{line}");
    }
    Ok(())
}

fn service_show(cli: &CliOptions) -> Result<()> {
    let route = service_route_from_arg(cli, 1, "overview")?;
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    if cli.output.json {
        return super::super::errors::print_json(&response);
    }
    for line in service_show_lines(&response) {
        println!("{line}");
    }
    Ok(())
}

fn service_status(cli: &CliOptions) -> Result<()> {
    let route = service_route_from_arg(cli, 1, "overview")?;
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    let status = service_status_summary(&response);
    if cli.output.json {
        return super::super::errors::print_json(&status);
    }
    for line in service_status_lines(&status) {
        println!("{line}");
    }
    Ok(())
}

fn service_route_from_arg(cli: &CliOptions, arg_index: usize, suffix: &str) -> Result<String> {
    let service = cli
        .args
        .get(arg_index)
        .cloned()
        .filter(|value| !value.is_empty());
    if let Some(service) = service {
        let suffix = suffix.trim_matches('/');
        if suffix.is_empty() {
            return Ok(format!("/v1/services/{}", encode_component(&service)));
        }
        return Ok(format!(
            "/v1/services/{}/{}",
            encode_component(&service),
            suffix
        ));
    }
    service_route(cli, suffix)
}

fn service_status_summary(response: &Value) -> Value {
    let status = response.get("status").unwrap_or(&Value::Null);
    let service = status.get("service").unwrap_or(&Value::Null);
    let latest_deploy = status.get("latest_deploy").unwrap_or(&Value::Null);
    let latest_build = status.get("latest_build_job").unwrap_or(&Value::Null);
    let runtime_status = string_field(service, "runtime_status");
    let latest_build_status = string_field(latest_build, "status");
    let live = runtime_status == "running" && latest_build_status == "succeeded";

    json!({
        "service": {
            "id": string_field(service, "id"),
            "name": string_field(service, "name"),
            "kind": string_field(service, "kind"),
            "runtimeStatus": runtime_status,
            "url": string_field(service, "url")
        },
        "latestDeploy": {
            "id": string_field(latest_deploy, "id"),
            "buildJobId": string_field(latest_deploy, "build_job_id"),
            "url": string_field(latest_deploy, "url")
        },
        "latestBuildJob": {
            "id": string_field(latest_build, "id"),
            "status": latest_build_status,
            "message": string_field(latest_build, "message")
        },
        "live": live,
        "nextActions": [
            "Use `tovuk service show <service> --json` for the full resource, usage, billing, logs, env, domain, and next-action snapshot.",
            "Use `tovuk logs --service <service> --limit 100 --json` when the latest build is not succeeded or the runtime is not running.",
            "Use `tovuk deploy list --service <service> --json` to compare recent deploy history."
        ]
    })
}

fn service_status_lines(status: &Value) -> Vec<String> {
    let service = status.get("service").unwrap_or(&Value::Null);
    let latest_deploy = status.get("latestDeploy").unwrap_or(&Value::Null);
    let latest_build = status.get("latestBuildJob").unwrap_or(&Value::Null);
    let service_id = string_field(service, "id");
    let service_name = string_field(service, "name");
    let runtime_status = string_field(service, "runtimeStatus");
    let latest_deploy_id = string_field(latest_deploy, "id");
    let latest_build_id = string_field(latest_build, "id");
    let latest_build_status = string_field(latest_build, "status");
    let live = status
        .get("live")
        .and_then(Value::as_bool)
        .unwrap_or_default();

    let mut lines = vec![
        format!("service {service_name}"),
        format!("id {service_id}"),
        format!("status {runtime_status}"),
        format!("live {live}"),
        format!("url {}", string_field(service, "url")),
    ];
    if !latest_deploy_id.is_empty() {
        lines.push(format!("latest_deploy {latest_deploy_id}"));
    }
    if !latest_build_id.is_empty() {
        lines.push(format!(
            "latest_build {latest_build_id} {latest_build_status}"
        ));
    }
    lines.push(format!("json: tovuk service status {service_id} --json"));
    lines.push(format!("full: tovuk service show {service_id} --json"));
    lines
}

fn service_list_lines(response: &Value) -> Vec<String> {
    let services = response
        .get("services")
        .and_then(Value::as_array)
        .map_or(EMPTY_VALUES, Vec::as_slice);
    if services.is_empty() {
        return vec![
            "no services".to_owned(),
            "next: tovuk deploy --dry-run --json".to_owned(),
        ];
    }

    let mut lines = Vec::with_capacity(services.len() + 2);
    lines.push("name\tservice\tkind\tstatus\tcapabilities\tresources\turl".to_owned());
    for service in services {
        let name = string_field(service, "name");
        let service_id = string_field(service, "serviceId");
        let kind = string_field(service, "kind");
        let status = string_field(service, "runtimeStatus");
        let url = string_field(service, "url");
        let capabilities = capability_summary(service.get("capabilities").unwrap_or(&Value::Null));
        let resources = resource_count_summary(
            service.get("resources").unwrap_or(&Value::Null),
            ACCOUNT_SERVICE_RESOURCE_COUNT_FIELDS,
        );
        lines.push(format!(
            "{name}\t{service_id}\t{kind}\t{status}\t{capabilities}\t{resources}\t{url}"
        ));
    }
    lines.push("next: tovuk service show <service> --json".to_owned());
    lines
}

fn service_show_lines(response: &Value) -> Vec<String> {
    let service = response
        .get("status")
        .and_then(|status| status.get("service"))
        .unwrap_or(&Value::Null);
    let service_id = string_field(service, "id");
    let name = string_field(service, "name");
    let kind = string_field(service, "kind");
    let status = string_field(service, "runtime_status");
    let url = string_field(service, "url");
    let latest_deploy = nested_string(response, &["status", "latest_deploy", "id"]);
    let latest_build = nested_string(response, &["status", "latest_build_job", "id"]);
    let latest_build_status = nested_string(response, &["status", "latest_build_job", "status"]);

    let mut lines = vec![
        format!("service {name}"),
        format!("id {service_id}"),
        format!("kind {kind}"),
        format!("status {status}"),
        format!("url {url}"),
        format!("resources {}", service_show_resource_summary(response)),
        format!(
            "capabilities {}",
            capability_summary(service.get("capabilities").unwrap_or(&Value::Null))
        ),
        format!("usage {}", service_show_usage_summary(response)),
    ];
    if !latest_deploy.is_empty() {
        lines.push(format!("latest_deploy {latest_deploy}"));
    }
    if !latest_build.is_empty() {
        lines.push(format!("latest_build {latest_build} {latest_build_status}"));
    }
    lines.push(format!("json: tovuk service show {service_id} --json"));
    for action in response
        .get("next_actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(3)
    {
        lines.push(format!("next: {action}"));
    }
    lines
}

fn service_show_resource_summary(response: &Value) -> String {
    let resources = response.get("resources").unwrap_or(&Value::Null);
    let usage = response.get("accountUsage").unwrap_or(&Value::Null);
    let mut parts = Vec::from([
        format!("sqlite={}", array_len(resources, "sqliteDatabases")),
        format!("storage={}", number_field(usage, "storageObjectCount")),
        format!("kv={}", array_len(resources, "kvNamespaces")),
        format!("queues={}", array_len(resources, "queues")),
        format!("cron={}", array_len(resources, "cronTriggers")),
        format!("state={}", array_len(resources, "stateNamespaces")),
        format!("bindings={}", array_len(resources, "serviceBindings")),
        format!(
            "secrets={}",
            response
                .get("env")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        ),
        format!(
            "domains={}",
            response
                .get("domains")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
        ),
    ]);
    parts.push(format!("caps={}", array_len(resources, "usageCaps")));
    parts.join(" ")
}

fn service_show_usage_summary(response: &Value) -> String {
    let usage = response.get("accountUsage").unwrap_or(&Value::Null);
    format!(
        "requests_day={} cpu_ms_day={} build_minutes_month={} storage_mib={}",
        usage_meter(usage, "day", "worker_requests"),
        usage_meter(usage, "day", "worker_cpu_ms"),
        usage_meter(usage, "month", "build_minutes"),
        number_field(usage, "objectStorageMib"),
    )
}

fn resource_count_summary(value: &Value, fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(label, key)| format!("{label}={}", number_field(value, key)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn capability_summary(value: &Value) -> String {
    format!(
        "enabled={} disabled={}",
        capability_state_summary(value, true),
        capability_state_summary(value, false)
    )
}

fn capability_state_summary(value: &Value, enabled: bool) -> String {
    let items = CapabilitiesConfig::KEYS
        .iter()
        .copied()
        .filter(|key| value.get(key).and_then(Value::as_bool) == Some(enabled))
        .collect::<Vec<_>>();

    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(",")
    }
}

fn usage_meter(usage: &Value, window: &str, meter: &str) -> u64 {
    usage
        .get("meters")
        .and_then(|meters| meters.get(window))
        .map_or(0, |meters| number_field(meters, meter))
}

fn array_len(value: &Value, key: &str) -> usize {
    value.get(key).and_then(Value::as_array).map_or(0, Vec::len)
}

const ACCOUNT_SERVICE_RESOURCE_COUNT_FIELDS: &[(&str, &str)] = &[
    ("sqlite", "sqliteDatabases"),
    ("storage", "storageObjects"),
    ("kv", "kvNamespaces"),
    ("queues", "queues"),
    ("cron", "cronTriggers"),
    ("state", "stateNamespaces"),
    ("bindings", "serviceBindings"),
    ("secrets", "secrets"),
    ("domains", "customDomains"),
];

const EMPTY_VALUES: &[Value] = &[];

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        service_list_lines, service_show_lines, service_status_lines, service_status_summary,
    };

    #[test]
    fn service_list_lines_show_resource_counts() {
        assert_eq!(
            service_list_lines(&json!({
                "services": [{
                    "name": "hello-rust",
                    "serviceId": "service_1",
                    "kind": "fullstack",
                    "runtimeStatus": "running",
                    "url": "https://hello-rust.tovuk.app",
                    "capabilities": {
                        "static_frontend": true,
                        "worker": true,
                        "sqlite": true,
                        "object_storage": true,
                        "kv": true,
                        "state": true,
                        "queue": true,
                        "cron": true,
                        "service_bindings": true,
                        "secrets": true,
                        "custom_domains": true,
                        "logs": true,
                        "builds": true,
                        "usage_caps": true,
                        "billing": true,
                        "support": true,
                        "abuse": true
                    },
                    "resources": {
                        "sqliteDatabases": 1,
                        "storageObjects": 2,
                        "kvNamespaces": 3,
                        "queues": 4,
                        "cronTriggers": 5,
                        "stateNamespaces": 6,
                        "serviceBindings": 7,
                        "secrets": 8,
                        "customDomains": 9
                    }
                }]
            })),
            vec![
                "name\tservice\tkind\tstatus\tcapabilities\tresources\turl".to_owned(),
                "hello-rust\tservice_1\tfullstack\trunning\tenabled=static_frontend,worker,sqlite,object_storage,kv,state,queue,cron,service_bindings,secrets,custom_domains,logs,builds,usage_caps,billing,support,abuse disabled=none\tsqlite=1 storage=2 kv=3 queues=4 cron=5 state=6 bindings=7 secrets=8 domains=9\thttps://hello-rust.tovuk.app".to_owned(),
                "next: tovuk service show <service> --json".to_owned(),
            ]
        );
    }

    #[test]
    fn service_show_lines_include_usage_and_next_actions() {
        assert_eq!(
            service_show_lines(&json!({
                "status": {
                    "service": {
                        "id": "service_1",
                        "name": "hello-rust",
                        "kind": "fullstack",
                        "runtime_status": "running",
                        "url": "https://hello-rust.tovuk.app",
                        "capabilities": {
                            "static_frontend": true,
                            "worker": true,
                            "sqlite": true,
                            "object_storage": true,
                            "kv": true,
                            "state": false,
                            "queue": false,
                            "cron": false,
                            "service_bindings": false,
                            "secrets": true,
                            "custom_domains": true,
                            "logs": true,
                            "builds": true,
                            "usage_caps": true,
                            "billing": true,
                            "support": true,
                            "abuse": true
                        }
                    },
                    "latest_deploy": { "id": "deploy_1" },
                    "latest_build_job": { "id": "job_1", "status": "succeeded" }
                },
                "resources": {
                    "sqliteDatabases": [{ "name": "DB" }],
                    "kvNamespaces": [{ "name": "CACHE" }],
                    "queues": [],
                    "cronTriggers": [],
                    "stateNamespaces": [],
                    "serviceBindings": [],
                    "usageCaps": [{ "metric": "worker_requests" }]
                },
                "env": [{ "name": "API_KEY" }],
                "domains": [{ "domain": "www.example.com" }],
                "accountUsage": {
                    "storageObjectCount": 2,
                    "objectStorageMib": 24,
                    "meters": {
                        "day": { "worker_requests": 10, "worker_cpu_ms": 4 },
                        "month": { "build_minutes": 2 }
                    }
                },
                "next_actions": ["Inspect logs."]
            })),
            vec![
                "service hello-rust".to_owned(),
                "id service_1".to_owned(),
                "kind fullstack".to_owned(),
                "status running".to_owned(),
                "url https://hello-rust.tovuk.app".to_owned(),
                "resources sqlite=1 storage=2 kv=1 queues=0 cron=0 state=0 bindings=0 secrets=1 domains=1 caps=1".to_owned(),
                "capabilities enabled=static_frontend,worker,sqlite,object_storage,kv,secrets,custom_domains,logs,builds,usage_caps,billing,support,abuse disabled=state,queue,cron,service_bindings".to_owned(),
                "usage requests_day=10 cpu_ms_day=4 build_minutes_month=2 storage_mib=24".to_owned(),
                "latest_deploy deploy_1".to_owned(),
                "latest_build job_1 succeeded".to_owned(),
                "json: tovuk service show service_1 --json".to_owned(),
                "next: Inspect logs.".to_owned(),
            ]
        );
    }

    #[test]
    fn service_status_summary_is_compact_and_agent_readable() {
        let status = service_status_summary(&json!({
            "status": {
                "service": {
                    "id": "service_1",
                    "name": "hello-rust",
                    "kind": "fullstack",
                    "runtime_status": "running",
                    "url": "https://hello-rust.tovuk.app"
                },
                "latest_deploy": {
                    "id": "deploy_1",
                    "build_job_id": "job_1",
                    "url": "https://hello-rust.tovuk.app"
                },
                "latest_build_job": {
                    "id": "job_1",
                    "status": "succeeded",
                    "message": "Deploy is live."
                }
            },
            "accountUsage": {
                "meters": {
                    "day": { "worker_requests": 10 },
                    "month": { "build_minutes": 2 }
                }
            },
            "resources": {
                "sqliteDatabases": [{ "name": "DB" }]
            }
        }));

        assert_eq!(
            status,
            json!({
                "service": {
                    "id": "service_1",
                    "name": "hello-rust",
                    "kind": "fullstack",
                    "runtimeStatus": "running",
                    "url": "https://hello-rust.tovuk.app"
                },
                "latestDeploy": {
                    "id": "deploy_1",
                    "buildJobId": "job_1",
                    "url": "https://hello-rust.tovuk.app"
                },
                "latestBuildJob": {
                    "id": "job_1",
                    "status": "succeeded",
                    "message": "Deploy is live."
                },
                "live": true,
                "nextActions": [
                    "Use `tovuk service show <service> --json` for the full resource, usage, billing, logs, env, domain, and next-action snapshot.",
                    "Use `tovuk logs --service <service> --limit 100 --json` when the latest build is not succeeded or the runtime is not running.",
                    "Use `tovuk deploy list --service <service> --json` to compare recent deploy history."
                ]
            })
        );
    }

    #[test]
    fn service_status_lines_point_to_compact_and_full_outputs() {
        assert_eq!(
            service_status_lines(&json!({
                "service": {
                    "id": "service_1",
                    "name": "hello-rust",
                    "kind": "fullstack",
                    "runtimeStatus": "starting",
                    "url": "https://hello-rust.tovuk.app"
                },
                "latestDeploy": {
                    "id": "deploy_1",
                    "buildJobId": "job_1",
                    "url": "https://hello-rust.tovuk.app"
                },
                "latestBuildJob": {
                    "id": "job_1",
                    "status": "running",
                    "message": "Build running."
                },
                "live": false,
                "nextActions": []
            })),
            vec![
                "service hello-rust".to_owned(),
                "id service_1".to_owned(),
                "status starting".to_owned(),
                "live false".to_owned(),
                "url https://hello-rust.tovuk.app".to_owned(),
                "latest_deploy deploy_1".to_owned(),
                "latest_build job_1 running".to_owned(),
                "json: tovuk service status service_1 --json".to_owned(),
                "full: tovuk service show service_1 --json".to_owned(),
            ]
        );
    }
}
