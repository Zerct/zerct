mod archive;
mod artifact;
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
    config::TovukConfig,
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
    match cli.args.first().map(String::as_str) {
        Some("list") => return list_deploys(cli),
        Some("show") => return show_deploy(cli),
        Some("cancel") => return cancel_deploy(cli),
        _ => {}
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

fn list_deploys(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let route = deploy_list_route(cli);
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    print_json(&response)
}

fn show_deploy(cli: &CliOptions) -> Result<()> {
    let deploy_id = deploy_id_arg(cli, "show")?;
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::GET,
        &deploy_show_route(&deploy_id),
        Some(&token),
        None,
    )?;
    print_json(&response)
}

fn cancel_deploy(cli: &CliOptions) -> Result<()> {
    let deploy_id = deploy_id_arg(cli, "cancel")?;
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

fn deploy_id_arg(cli: &CliOptions, command: &str) -> Result<String> {
    cli.args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            agent_error(
                "deploy_id_required",
                "Deploy id is required.",
                format!(
                    "Pass a deploy id to `tovuk deploy {command} <deploy_id> --json` from `tovuk deploy list --json`, `tovuk deploy --wait --json`, `tovuk service show <service> --json`, or build logs."
                ),
                cli.output.json,
            )
        })
}

fn deploy_list_route(cli: &CliOptions) -> String {
    let suffix = deploy_page_query(cli);
    if cli.service.is_empty() {
        return format!("/v1/deploys{suffix}");
    }
    format!(
        "/v1/services/{}/deploys{suffix}",
        encode_component(&cli.service)
    )
}

fn deploy_show_route(deploy_id: &str) -> String {
    format!("/v1/deploys/{}", encode_component(deploy_id))
}

fn deploy_cancel_route(deploy_id: &str) -> String {
    format!("/v1/deploys/{}/cancel", encode_component(deploy_id))
}

fn deploy_page_query(cli: &CliOptions) -> String {
    let mut params = Vec::new();
    if !cli.limit.is_empty() {
        params.push(format!("limit={}", encode_component(&cli.limit)));
    }
    if !cli.cursor.is_empty() {
        params.push(format!("cursor={}", encode_component(&cli.cursor)));
    }
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
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
    let config = report
        .config
        .as_ref()
        .map(|config| deploy_config_value(config, cli.output.json))
        .transpose()?;
    let body = json!({
        "config": config,
        "commit_sha": git_commit_sha(project_dir),
        "source_archive_base64": create_archive_base64(project_dir, cli.output.json)?,
    });
    api_request(cli, Method::POST, "/v1/deploy", Some(token), Some(body))
}

fn deploy_config_value(config: &TovukConfig, json_output: bool) -> Result<Value> {
    let mut value = serde_json::to_value(config).map_err(|error| {
        agent_error(
            "config_serialization_failed",
            "Could not serialize tovuk.toml.",
            format!("Fix tovuk.toml, then retry deploy: {error}"),
            json_output,
        )
    })?;
    if let Value::Object(fields) = &mut value {
        fields.remove("dev");
    }
    Ok(value)
}

fn git_commit_sha(project_dir: &Path) -> Option<String> {
    if !git_worktree_is_clean(project_dir) {
        return None;
    }

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

fn git_worktree_is_clean(project_dir: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_dir)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| git_porcelain_is_clean(&output.stdout))
}

fn git_porcelain_is_clean(stdout: &[u8]) -> bool {
    String::from_utf8_lossy(stdout).trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{
        deploy_cancel_route, deploy_config_value, deploy_id_arg, deploy_list_route,
        deploy_show_route, git_porcelain_is_clean,
    };
    use crate::cli::{
        args::CliOptions,
        config::{parse_tovuk_toml, validate_config},
    };
    use std::path::Path;

    #[test]
    fn deploy_show_and_cancel_use_target_routes() {
        assert_eq!(
            deploy_show_route("deploy_0123456789abcdef0123"),
            "/v1/deploys/deploy_0123456789abcdef0123"
        );
        assert_eq!(deploy_show_route("deploy/id"), "/v1/deploys/deploy%2Fid");
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
    fn deploy_list_uses_account_or_service_route() {
        let account_cli = CliOptions {
            command: "deploy".to_owned(),
            args: vec!["list".to_owned()],
            limit: "10".to_owned(),
            cursor: "next/page".to_owned(),
            ..CliOptions::default()
        };
        assert_eq!(
            deploy_list_route(&account_cli),
            "/v1/deploys?limit=10&cursor=next%2Fpage"
        );

        let service_cli = CliOptions {
            command: "deploy".to_owned(),
            args: vec!["list".to_owned()],
            service: "api/service".to_owned(),
            ..CliOptions::default()
        };
        assert_eq!(
            deploy_list_route(&service_cli),
            "/v1/services/api%2Fservice/deploys"
        );
    }

    #[test]
    fn deploy_lifecycle_commands_require_deploy_id() {
        let cli = CliOptions {
            command: "deploy".to_owned(),
            args: vec!["cancel".to_owned()],
            ..CliOptions::default()
        };

        let error_message = deploy_id_arg(&cli, "cancel")
            .err()
            .map(|error| error.to_string());
        assert_eq!(error_message.as_deref(), Some("Deploy id is required."));
    }

    #[test]
    fn git_porcelain_cleanliness_detects_dirty_worktree() {
        assert!(git_porcelain_is_clean(b""));
        assert!(git_porcelain_is_clean(b"\n"));
        assert!(!git_porcelain_is_clean(
            b" M crates/tovuk/src/cli/deploy.rs\n"
        ));
        assert!(!git_porcelain_is_clean(
            b"?? examples/shape-store/web/dist/\n"
        ));
    }

    #[test]
    fn deploy_config_value_strips_local_dev_config() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"
name = "demo"

[capabilities]
static_frontend = false
worker = true
sqlite = false
object_storage = false
kv = false
state = false
queue = false
cron = false
service_bindings = false
secrets = false
custom_domains = false
logs = true
builds = true
usage_caps = true
billing = true
support = true
abuse = true

[run]
command = "./target/release/demo"

[dev]
worker_port = 3001
"#;
        let config = parse_tovuk_toml(source, Path::new("."))?;
        validate_config(&config)?;

        let value = deploy_config_value(&config, true)?;

        if value.get("dev").is_some() {
            return Err(format!("deploy config should not include dev: {value}").into());
        }
        if value["name"] != "demo" {
            return Err(format!("unexpected deploy config: {value}").into());
        }
        Ok(())
    }
}
