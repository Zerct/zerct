use super::super::{
    args::CliOptions,
    constants::{BILLING_CHECKOUT_ROUTE, VERSION},
    errors::{
        AgentErrorPayload, CliError, CliFailure, Result, agent_error_with_docs, internal_error,
    },
};
use reqwest::{Method, blocking::Client};
use serde_json::{Value, json};

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
            "Retry the command. If it keeps failing, check Tovuk status before changing your project.",
            "https://docs.tovuk.com/status",
            cli.output.json,
        )
    })?;
    let status = response.status();
    let text = response.text().unwrap_or_default();
    let data = parse_json_text(&text);
    if status.is_success() {
        return Ok(data);
    }

    let mut payload = agent_payload_from_json(&data).unwrap_or_else(|| AgentErrorPayload {
        code: "api_error".to_owned(),
        message: format!("Tovuk API returned HTTP {}.", status.as_u16()),
        agent_instruction: Some(
            "Retry the command. If it keeps failing, check Tovuk status before changing your project."
                .to_owned(),
        ),
        docs_url: None,
        checkout_url: None,
    });
    enrich_agent_error_payload(cli, route, token, &mut payload);
    Err(CliError::new(CliFailure {
        payload,
        json: cli.output.json,
        exit_code: if status.is_server_error() { 2 } else { 1 },
    }))
}

fn parse_json_text(text: &str) -> Value {
    if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(text).unwrap_or(Value::Null)
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

fn enrich_agent_error_payload(
    cli: &CliOptions,
    route: &str,
    token: Option<&str>,
    payload: &mut AgentErrorPayload,
) {
    if payload.code != "payment_required"
        || payload
            .checkout_url
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        || token.is_none()
        || route == BILLING_CHECKOUT_ROUTE
    {
        return;
    }
    if let Some(url) = create_checkout_url(cli, token, &payload.message) {
        payload.checkout_url = Some(url);
    }
}

fn create_checkout_url(cli: &CliOptions, token: Option<&str>, reason: &str) -> Option<String> {
    let token = token?;
    let response = api_request(
        cli,
        Method::POST,
        BILLING_CHECKOUT_ROUTE,
        Some(token),
        Some(json!({
            "reason": if reason.is_empty() { "Plan limit reached." } else { reason },
            "target_plan": "pro",
        })),
    )
    .ok()?;
    response
        .get("checkout")
        .and_then(|checkout| checkout.get("url"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::api_request;
    use crate::cli::args::CliOptions;
    use reqwest::Method;
    use std::{error::Error, net::TcpListener};

    #[test]
    fn api_unreachable_points_agents_to_status_docs() -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let api_url = format!("http://{}", listener.local_addr()?);
        drop(listener);
        let cli = CliOptions {
            api_url,
            ..CliOptions::default()
        };

        let error = match api_request(&cli, Method::GET, "/v1/status", None, None) {
            Ok(_response) => return Err("request unexpectedly succeeded".into()),
            Err(error) => error,
        };
        let payload = error.payload();

        if payload.code != "api_unreachable" {
            return Err(format!("unexpected code: {}", payload.code).into());
        }
        if payload.docs_url.as_deref() != Some("https://docs.tovuk.com/status") {
            return Err(format!("unexpected docs url: {:?}", payload.docs_url).into());
        }
        Ok(())
    }
}
