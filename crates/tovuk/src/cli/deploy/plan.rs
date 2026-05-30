use super::{
    super::{
        api_commands::{api_request, payment_required_agent_error},
        args::CliOptions,
        errors::{Result, agent_error},
        project::number_field,
        project_kind::ProjectKind,
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
    let plan = projects
        .iter()
        .map(|project| DeployPlanProject {
            project: project.clone(),
            wants_database: cli.deployment.database
                && project.kind.is_some_and(ProjectKind::supports_database),
        })
        .collect::<Vec<_>>();
    reject_invalid_database_targets(&plan, cli)?;
    preflight_deploy_limits(&plan, cli, token)?;
    Ok(plan)
}

fn reject_invalid_database_targets(plan: &[DeployPlanProject], cli: &CliOptions) -> Result<()> {
    if cli.deployment.database
        && plan.len() == 1
        && plan.first().is_some_and(|item| {
            item.project
                .kind
                .is_some_and(ProjectKind::is_static_frontend)
        })
    {
        return Err(agent_error(
            "invalid_database_target",
            "Static frontends cannot attach managed Postgres directly.",
            "Deploy a Rust backend with managed Postgres and call it from the frontend.",
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
    let apps_response = api_request(cli, Method::GET, "/v1/apps", Some(token), None)?;
    let existing_apps = app_name_set(&apps_response);
    let requested = requested_new_resources(plan, &existing_apps);
    let usage = usage_response.get("usage").unwrap_or(&Value::Null);
    let limits = usage_response.get("limits").unwrap_or(&Value::Null);
    let used_projects = number_field(usage, "appCount");
    let project_limit = number_field(limits, "projects");
    let used_databases = number_field(usage, "databaseCount");
    let database_limit = number_field(limits, "managedDatabases");

    if requested.projects > 0 && used_projects + requested.projects > project_limit {
        return Err(payment_required_agent_error(
            cli,
            token,
            format!(
                "Project limit reached: {used_projects}/{project_limit} projects are already used."
            ),
            "Redeploy an existing app by reusing its `name` in tovuk.toml, or open the returned Stripe Checkout URL before creating another project.",
        ));
    }
    if requested.databases > 0 && used_databases + requested.databases > database_limit {
        return Err(payment_required_agent_error(
            cli,
            token,
            format!(
                "Managed Postgres limit reached: {used_databases}/{database_limit} databases are already used."
            ),
            "Redeploy an app that already has managed Postgres, deploy without `--database`, or open the returned Stripe Checkout URL.",
        ));
    }
    Ok(())
}

struct RequestedResources {
    projects: u64,
    databases: u64,
}

fn app_name_set(response: &Value) -> BTreeSet<String> {
    response
        .get("apps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|app| app.get("name").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn requested_new_resources(
    plan: &[DeployPlanProject],
    existing_apps: &BTreeSet<String>,
) -> RequestedResources {
    let mut projects = 0u64;
    let mut databases = 0u64;
    for target in plan {
        if target.project.name.is_empty() || target.project.kind.is_none() {
            continue;
        }
        if !existing_apps.contains(&target.project.name) {
            projects += 1;
            if target.wants_database {
                databases += 1;
            }
        }
    }
    RequestedResources {
        projects,
        databases,
    }
}
