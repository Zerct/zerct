use super::super::{
    args::CliOptions,
    constants::VERSION,
    errors::{
        AgentErrorPayload, CliError, CliFailure, Result, agent_error_with_docs, internal_error,
    },
};
use reqwest::{Method, blocking::Client};
use serde_json::Value;

pub(crate) fn api_request(
    cli: &CliOptions,
    method: Method,
    route: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> Result<Value> {
    let client = Client::builder()
        .user_agent(format!("tovuk-cli/{VERSION}"))
        .build()
        .map_err(|error| internal_error(error.to_string()))?;
    let url = format!("{}{}", cli.api_url, route);
    let mut request = client
        .request(method, url)
        .header("accept", "application/json");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().map_err(|error| {
        agent_error_with_docs(
            "api_unreachable",
            format!("Could not reach Tovuk API: {error}"),
            "Retry the command. If it keeps failing, check Tovuk status before changing your request.",
            "https://docs.tovuk.com/status",
            cli.output.json,
        )
    })?;
    let status = response.status();
    if status.is_success() {
        let text = response
            .text()
            .map_err(|error| internal_error(error.to_string()))?;
        return parse_success_json_text(&text);
    }

    let text = response
        .text()
        .map_err(|error| internal_error(error.to_string()))?;
    let data = parse_error_json_text(&text);
    let payload = agent_payload_from_json(&data).unwrap_or_else(|| AgentErrorPayload {
        code: "api_error".to_owned(),
        message: format!("Tovuk API returned HTTP {}.", status.as_u16()),
        agent_instruction: Some(
            "Retry the command. If it keeps failing, check Tovuk status before changing your request."
                .to_owned(),
        ),
        docs_url: None,
        checkout_url: None,
    });
    Err(CliError::new(CliFailure {
        payload,
        json: cli.output.json,
        exit_code: if status.is_server_error() { 2 } else { 1 },
    }))
}

fn parse_success_json_text(text: &str) -> Result<Value> {
    if text.trim().is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_str(text)
            .map_err(|error| internal_error(format!("Tovuk API returned invalid JSON: {error}")))
    }
}

fn parse_error_json_text(text: &str) -> Value {
    if text.trim().is_empty() {
        return Value::Null;
    }
    match serde_json::from_str(text) {
        Ok(value) => value,
        Err(_) => Value::Null,
    }
}

fn agent_payload_from_json(value: &Value) -> Option<AgentErrorPayload> {
    let object = value.as_object()?;
    Some(AgentErrorPayload {
        code: object.get("code")?.as_str()?.to_owned(),
        message: object.get("message")?.as_str()?.to_owned(),
        agent_instruction: object
            .get("agent_instruction")
            .and_then(Value::as_str)
            .map(str::to_owned),
        docs_url: object
            .get("docs_url")
            .and_then(Value::as_str)
            .map(str::to_owned),
        checkout_url: object
            .get("checkout_url")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
