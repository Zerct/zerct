use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, print_json},
    project::{encode_component, string_field},
};
use super::{
    common::{page_query, require_service},
    http::api_request,
};
use reqwest::Method;
use serde_json::Value;

pub(crate) fn logs_command(cli: &CliOptions) -> Result<()> {
    let (route, target) = log_route(cli)?;
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    if cli.output.json {
        return print_json(&response);
    }

    for line in response
        .get("lines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let timestamp = string_field(line, "timestamp");
        let stream = string_field(line, "stream");
        let message = string_field(line, "message");
        println!("[{timestamp}] {stream}: {message}");
    }
    if response
        .get("has_more")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let cursor = string_field(&response, "next_cursor");
        if !cursor.is_empty() {
            println!("next tovuk logs {target} --cursor {cursor}");
        }
    }
    Ok(())
}

fn log_route(cli: &CliOptions) -> Result<(String, String)> {
    if !cli.build.is_empty() {
        return Ok((
            format!(
                "/v1/builds/{}/logs{}",
                encode_component(&cli.build),
                page_query(cli)
            ),
            format!("--build {}", cli.build),
        ));
    }
    if !cli.deploy.is_empty() {
        return Ok((
            format!(
                "/v1/deploys/{}/logs{}",
                encode_component(&cli.deploy),
                page_query(cli)
            ),
            format!("--deploy {}", cli.deploy),
        ));
    }

    let service = require_service(cli)?;
    Ok((
        format!(
            "/v1/services/{}/logs{}",
            encode_component(&service),
            page_query(cli)
        ),
        format!("--service {service}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::logs_command;
    use crate::cli::args::CliOptions;

    #[test]
    fn logs_require_service_before_login() {
        let cli = CliOptions {
            command: "logs".to_owned(),
            ..CliOptions::default()
        };

        let error_message = logs_command(&cli).err().map(|error| error.to_string());
        assert_eq!(error_message.as_deref(), Some("Service is required."));
    }
}
