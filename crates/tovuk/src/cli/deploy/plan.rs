use super::{
    super::{
        api_commands::{api_request, payment_required_agent_error},
        args::CliOptions,
        errors::{Result, agent_error},
        project::number_field,
    },
    types::{DeployPlanProject, DeployProjectInfo},
};
use reqwest::Method;
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn create_deploy_plan(
    projects: &[DeployProjectInfo],
    cli: &CliOptions,
    token: &str,
) -> Result<Vec<DeployPlanProject>> {
    reject_legacy_database_flag(cli)?;
    let plan = projects
        .iter()
        .map(|project| DeployPlanProject {
            project: project.clone(),
        })
        .collect::<Vec<_>>();
    preflight_deploy_limits(&plan, cli, token)?;
    Ok(plan)
}

fn reject_legacy_database_flag(cli: &CliOptions) -> Result<()> {
    if cli.deployment.database {
        return Err(agent_error(
            "deploy_database_flag_removed",
            "The deploy-time database flag is no longer supported.",
            "Create SQLite databases with `tovuk database create --service <service> DB --json` and bind them through worker service resources.",
            cli.output.json,
        ));
    }
    Ok(())
}

fn preflight_deploy_limits(
    plan: &[DeployPlanProject],
    cli: &CliOptions,
    token: &str,
) -> Result<()> {
    let usage_response = api_request(cli, Method::GET, "/v1/usage", Some(token), None)?;
    let services_response = api_request(cli, Method::GET, "/v1/services", Some(token), None)?;
    let existing_services = service_name_set(&services_response);
    let requested = requested_new_resources(plan, &existing_services);
    let usage = usage_response.get("usage").unwrap_or(&Value::Null);
    let limits = usage_response.get("limits").unwrap_or(&Value::Null);
    let used_projects = number_field(usage, "serviceCount");
    let project_limit = number_field(limits, "projects");

    if requested.projects > 0 && used_projects + requested.projects > project_limit {
        return Err(payment_required_agent_error(
            cli,
            token,
            format!(
                "Project limit reached: {used_projects}/{project_limit} projects are already used."
            ),
            "Redeploy an existing service by reusing its `name` in tovuk.toml, or open the returned Stripe Checkout URL before creating another project.",
        ));
    }
    Ok(())
}

struct RequestedResources {
    projects: u64,
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

fn requested_new_resources(
    plan: &[DeployPlanProject],
    existing_apps: &BTreeSet<String>,
) -> RequestedResources {
    let mut projects = 0u64;
    for target in plan {
        if target.project.name.is_empty() || target.project.kind.is_none() {
            continue;
        }
        if !existing_apps.contains(&target.project.name) {
            projects += 1;
        }
    }
    RequestedResources { projects }
}
