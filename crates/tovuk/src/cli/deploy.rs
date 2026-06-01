mod archive;
mod discovery;
mod dry_run;
mod output;
mod plan;
mod types;
mod wait;

use super::{
    api_commands::api_request,
    args::CliOptions,
    auth::read_or_login_token,
    check::run_check,
    errors::{Result, agent_error, print_json},
    project::{encode_component, nested_string},
};
use archive::create_archive_base64;
pub(crate) use discovery::discover_deploy_projects;
use dry_run::print_deploy_dry_run;
use output::{print_deploy_result, print_workspace_deploy_results};
use plan::create_deploy_plan;
use reqwest::Method;
use serde_json::{Value, json};
use std::{
    path::Path,
    process::{Command, Stdio},
};
use types::{DeployPlanProject, WorkspaceDeployResult};
use wait::wait_for_workspace_builds;

pub(crate) fn deploy(project_dir: &Path, cli: &CliOptions) -> Result<()> {
    if cli.args.first().is_some_and(|arg| arg == "cancel") {
        return cancel_deploy(cli);
    }
    let projects = discover_deploy_projects(project_dir)?;
    if projects.is_empty() {
        return Err(agent_error(
            "missing_project_contract",
            "No tovuk.toml was found.",
            "Create a tovuk.toml in each service directory, run `tovuk new <path> --template <template>`, or pass a project path.",
            cli.output.json,
        ));
    }
    let token = read_or_login_token(cli)?;
    if cli.deployment.dry_run {
        return print_deploy_dry_run(project_dir, &projects, cli, &token);
    }
    let plan = create_deploy_plan(&projects, cli, &token)?;
    let mut results = deploy_projects(&plan, cli, &token)?;
    if cli.deployment.wait {
        wait_for_workspace_builds(cli, &token, &mut results)?;
    }
    if results.len() == 1 {
        print_deploy_result(&results[0].response, cli)
    } else {
        print_workspace_deploy_results(project_dir, &results, cli)
    }
}

fn cancel_deploy(cli: &CliOptions) -> Result<()> {
    let deploy_id = deploy_cancel_id(cli)?;
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::POST,
        &deploy_cancel_route(&deploy_id),
        Some(&token),
        None,
    )?;
    print_json(&response)
}

fn deploy_cancel_id(cli: &CliOptions) -> Result<String> {
    cli.args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            agent_error(
                "deploy_id_required",
                "Deploy id is required.",
                "Pass a deploy id returned by `tovuk deploy --wait --json`, `tovuk service show <service> --json`, or build logs.",
                cli.output.json,
            )
        })
}

fn deploy_cancel_route(deploy_id: &str) -> String {
    format!("/v1/deploys/{}/cancel", encode_component(deploy_id))
}

fn deploy_projects(
    plan: &[DeployPlanProject],
    cli: &CliOptions,
    token: &str,
) -> Result<Vec<WorkspaceDeployResult>> {
    let mut results = Vec::new();
    let workspace_deploy = plan.len() > 1;
    if workspace_deploy && !cli.output.json {
        println!("deploying {} projects", plan.len());
    }
    for item in plan {
        if workspace_deploy && !cli.output.json {
            println!("checking {}", item.project.relative);
        }
        let response = deploy_project(&item.project.dir, cli, token)?;
        if workspace_deploy && !cli.output.json {
            println!(
                "{} queued {}",
                item.project.relative,
                nested_string(&response, &["build_job", "id"])
            );
            println!(
                "{} url {}",
                item.project.relative,
                nested_string(&response, &["service", "url"])
            );
        }
        results.push(WorkspaceDeployResult {
            project: item.project.clone(),
            response,
            final_build: None,
        });
    }
    Ok(results)
}

fn deploy_project(project_dir: &Path, cli: &CliOptions, token: &str) -> Result<Value> {
    let report = run_check(project_dir);
    if !report.ok {
        let instruction = report
            .checks
            .iter()
            .find(|check| !check.ok)
            .and_then(|check| check.agent_instruction.clone())
            .unwrap_or_else(|| "Fix the failed checks and retry.".to_owned());
        return Err(agent_error(
            "check_failed",
            "Tovuk check failed.",
            instruction,
            cli.output.json,
        ));
    }
    let body = json!({
        "config": report.config,
        "commit_sha": git_commit_sha(project_dir),
        "source_archive_base64": create_archive_base64(project_dir, cli.output.json)?,
    });
    api_request(cli, Method::POST, "/v1/deploy", Some(token), Some(body))
}

fn git_commit_sha(project_dir: &Path) -> Option<String> {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{deploy_cancel_id, deploy_cancel_route};
    use crate::cli::args::CliOptions;

    #[test]
    fn deploy_cancel_uses_target_route() {
        assert_eq!(
            deploy_cancel_route("deploy_0123456789abcdef0123"),
            "/v1/deploys/deploy_0123456789abcdef0123/cancel"
        );
        assert_eq!(
            deploy_cancel_route("deploy/id"),
            "/v1/deploys/deploy%2Fid/cancel"
        );
    }

    #[test]
    fn deploy_cancel_requires_deploy_id() {
        let cli = CliOptions {
            command: "deploy".to_owned(),
            args: vec!["cancel".to_owned()],
            ..CliOptions::default()
        };

        let error_message = deploy_cancel_id(&cli).err().map(|error| error.to_string());
        assert_eq!(error_message.as_deref(), Some("Deploy id is required."));
    }
}
