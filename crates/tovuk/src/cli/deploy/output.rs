use super::{
    super::{
        args::CliOptions,
        errors::{Result, print_json},
        project::nested_string,
        project_kind::ProjectKind,
    },
    types::WorkspaceDeployResult,
};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn print_deploy_result(response: &Value, cli: &CliOptions) -> Result<()> {
    if cli.output.json {
        return print_json(response);
    }
    println!("queued {}", nested_string(response, &["build_job", "id"]));
    println!("service {}", service_field(response, "id"));
    println!("url {}", service_field(response, "url"));
    println!(
        "next tovuk logs --service {}",
        service_field(response, "id")
    );
    Ok(())
}

pub(super) fn print_workspace_deploy_results(
    project_dir: &Path,
    results: &[WorkspaceDeployResult],
    cli: &CliOptions,
) -> Result<()> {
    if cli.output.json {
        let deploys = results
            .iter()
            .map(|result| {
                json!({
                    "path": result.project.relative,
                    "kind": result.project.kind.map_or("invalid", ProjectKind::as_str),
                    "service": result.response.get("service").cloned().unwrap_or(Value::Null),
                    "build_job": result.response.get("build_job").cloned().unwrap_or(Value::Null),
                    "final_build": result.final_build.clone().unwrap_or(Value::Null),
                })
            })
            .collect::<Vec<_>>();
        return print_json(
            &json!({ "workspace": project_dir.display().to_string(), "deploys": deploys }),
        );
    }
    if let Some(first) = results.first() {
        println!(
            "next tovuk logs --service {}",
            service_field(&first.response, "id")
        );
    }
    Ok(())
}

fn service_field(response: &Value, field: &str) -> String {
    nested_string(response, &["service", field])
}
