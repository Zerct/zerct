use super::{
    super::{
        api_commands::api_request,
        args::CliOptions,
        errors::{Result, agent_error},
        project::{encode_component, progress, string_field},
    },
    types::WorkspaceDeployResult,
};
use reqwest::Method;
use serde_json::Value;
use std::{
    thread,
    time::{Duration, Instant},
};

pub(super) fn wait_for_workspace_builds(
    cli: &CliOptions,
    token: &str,
    results: &mut [WorkspaceDeployResult],
) -> Result<()> {
    for result in results {
        let build_id = super::super::project::nested_string(&result.response, &["build_job", "id"]);
        let final_build = wait_for_build(cli, token, &build_id)?;
        if let Some(object) = result.response.as_object_mut() {
            object.insert("final_build".to_owned(), final_build.clone());
        }
        result.final_build = Some(final_build);
    }
    Ok(())
}

fn wait_for_build(cli: &CliOptions, token: &str, build_id: &str) -> Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(cli.deployment.wait_timeout_seconds);
    let mut last_status = String::new();
    while Instant::now() <= deadline {
        let response = api_request(
            cli,
            Method::GET,
            &format!("/v1/builds/{}", encode_component(build_id)),
            Some(token),
            None,
        )?;
        let build = response.get("build").cloned().unwrap_or(Value::Null);
        let status = string_field(&build, "status");
        if status.is_empty() {
            return Err(agent_error(
                "build_status_unavailable",
                "Build status is unavailable.",
                format!("Retry with `tovuk logs --build {build_id}`."),
                cli.output.json,
            ));
        }
        if status != last_status {
            progress(
                cli,
                &format!("build {} {status}", string_field(&build, "id")),
            );
            last_status.clone_from(&status);
        }
        if ["succeeded", "failed", "canceled"].contains(&status.as_str()) {
            return Ok(build);
        }
        thread::sleep(Duration::from_secs(3));
    }
    Err(agent_error(
        "build_wait_timeout",
        format!("Timed out waiting for build {build_id}."),
        format!("Run `tovuk logs --build {build_id}` to continue watching."),
        cli.output.json,
    ))
}
