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
        if let Some(build) = terminal_build_result(cli, build_id, &status, build)? {
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

fn terminal_build_result(
    cli: &CliOptions,
    build_id: &str,
    status: &str,
    build: Value,
) -> Result<Option<Value>> {
    match status {
        "succeeded" => Ok(Some(build)),
        "failed" | "canceled" => Err(agent_error(
            format!("build_{status}"),
            format!("Build {build_id} {status}."),
            format!(
                "Run `tovuk logs --build {build_id} --limit 100 --json`, fix the first actionable error, then run `tovuk deploy --wait --json` again."
            ),
            cli.output.json,
        )),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::terminal_build_result;
    use crate::cli::args::CliOptions;
    use serde_json::json;

    #[test]
    fn terminal_build_result_returns_succeeded_build() {
        let build = json!({"id":"job_1","status":"succeeded"});
        let result =
            terminal_build_result(&CliOptions::default(), "job_1", "succeeded", build.clone())
                .ok()
                .flatten();

        assert_eq!(result, Some(build));
    }

    #[test]
    fn terminal_build_result_errors_on_failed_build() {
        let error = terminal_build_result(
            &CliOptions::default(),
            "job_1",
            "failed",
            json!({"id":"job_1","status":"failed"}),
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.payload().code.as_str()),
            Some("build_failed")
        );
        assert_eq!(
            error.as_ref().map(|error| error.payload().message.as_str()),
            Some("Build job_1 failed.")
        );
        assert_eq!(
            error
                .as_ref()
                .and_then(|error| error.payload().agent_instruction.as_deref()),
            Some(
                "Run `tovuk logs --build job_1 --limit 100 --json`, fix the first actionable error, then run `tovuk deploy --wait --json` again."
            )
        );
    }

    #[test]
    fn terminal_build_result_errors_on_canceled_build() {
        let error = terminal_build_result(
            &CliOptions::default(),
            "job_1",
            "canceled",
            json!({"id":"job_1","status":"canceled"}),
        )
        .err();

        assert_eq!(
            error.as_ref().map(|error| error.payload().code.as_str()),
            Some("build_canceled")
        );
    }
}
