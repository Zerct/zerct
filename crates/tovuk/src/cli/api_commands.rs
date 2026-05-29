use super::{
    args::CliOptions,
    auth::read_or_login_token,
    constants::{BILLING_CHECKOUT_ROUTE, VERSION},
    errors::{
        AgentErrorPayload, CliError, CliFailure, Result, agent_error, internal_error, print_json,
    },
    project::{encode_component, open_url, string_field},
};
use reqwest::{Method, blocking::Client};
use serde_json::{Map, Value, json};

pub(crate) fn capabilities(cli: &CliOptions) -> Result<()> {
    let response = api_request(cli, Method::GET, "/v1/capabilities", None, None)?;
    print_json(&response)
}

pub(crate) fn print_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, Method::GET, route, Some(&token), None)?;
    print_json(&response)
}

pub(crate) fn print_paged_authenticated(cli: &CliOptions, route: &str) -> Result<()> {
    print_authenticated(cli, &format!("{route}{}", page_query(cli)))
}

pub(crate) fn app_get_command(cli: &CliOptions, suffix: &str) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::GET,
        &app_route(cli, suffix)?,
        Some(&token),
        None,
    )?;
    print_json(&response)
}

pub(crate) fn deploys_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/deploys{}", page_query(cli))
    } else {
        format!(
            "/v1/apps/{}/deploys{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}

pub(crate) fn builds_command(cli: &CliOptions) -> Result<()> {
    let route = if cli.app.is_empty() {
        format!("/v1/builds{}", page_query(cli))
    } else {
        format!(
            "/v1/apps/{}/builds{}",
            encode_component(&cli.app),
            page_query(cli)
        )
    };
    print_authenticated(cli, &route)
}

pub(crate) fn logs_command(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let (route, target) = if !cli.build.is_empty() {
        (
            format!(
                "/v1/builds/{}/logs{}",
                encode_component(&cli.build),
                page_query(cli)
            ),
            format!("--build {}", cli.build),
        )
    } else if !cli.deploy.is_empty() {
        (
            format!(
                "/v1/deploys/{}/logs{}",
                encode_component(&cli.deploy),
                page_query(cli)
            ),
            format!("--deploy {}", cli.deploy),
        )
    } else {
        let app = require_app(cli)?;
        (
            format!(
                "/v1/apps/{}/logs{}",
                encode_component(&app),
                page_query(cli)
            ),
            format!("--app {app}"),
        )
    };
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

pub(crate) fn env_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => app_get_command(cli, "env"),
        "set" => {
            let assignment = cli.args.get(1).cloned().unwrap_or_default();
            let separator = assignment.find('=').unwrap_or(0);
            if separator == 0 {
                return Err(agent_error(
                    "invalid_env",
                    "Environment assignment must be KEY=value.",
                    "Pass one uppercase shell-safe environment assignment, for example `API_KEY=value`.",
                    cli.output.json,
                ));
            }
            let name = &assignment[..separator];
            let value = &assignment[separator + 1..];
            print_authenticated_mutation(
                cli,
                Method::PUT,
                &app_route(cli, "env")?,
                Some(json!({ "name": name, "value": value })),
            )
        }
        "delete" => {
            let name = command_arg(
                cli,
                "invalid_env",
                "Environment variable name is required.",
                "Use `tovuk env delete --app <app> KEY`.",
            )?;
            print_authenticated_mutation(
                cli,
                Method::DELETE,
                &app_route(cli, &format!("env/{}", encode_component(&name)))?,
                None,
            )
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown env command.",
            "Use `tovuk env list`, `env set`, or `env delete`.",
            cli.output.json,
        )),
    }
}

pub(crate) fn domains_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => app_get_command(cli, "domains"),
        "add" => {
            let domain = command_arg(
                cli,
                "missing_domain",
                "Domain is required.",
                "Use `tovuk domains add --app <app> api.example.com`.",
            )?;
            print_authenticated_mutation(
                cli,
                Method::POST,
                &app_route(cli, "domains")?,
                Some(json!({ "domain": domain })),
            )
        }
        "verify" => {
            let domain = command_arg(
                cli,
                "missing_domain",
                "Domain is required.",
                "Use `tovuk domains verify --app <app> api.example.com`.",
            )?;
            let app = require_app(cli)?;
            print_authenticated_mutation(
                cli,
                Method::POST,
                &format!(
                    "/v1/apps/{}/domains/{}/verify",
                    encode_component(&app),
                    encode_component(&domain)
                ),
                None,
            )
        }
        "delete" => {
            let domain = command_arg(
                cli,
                "missing_domain",
                "Domain is required.",
                "Use `tovuk domains delete --app <app> api.example.com`.",
            )?;
            let app = require_app(cli)?;
            print_authenticated_mutation(
                cli,
                Method::DELETE,
                &format!(
                    "/v1/apps/{}/domains/{}",
                    encode_component(&app),
                    encode_component(&domain)
                ),
                None,
            )
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown domains command.",
            "Use `domains list`, `domains add`, `domains verify`, or `domains delete`.",
            cli.output.json,
        )),
    }
}

pub(crate) fn billing_command(cli: &CliOptions) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let action = cli.args.first().map_or("checkout", String::as_str);
    let route = match action {
        "" | "checkout" => BILLING_CHECKOUT_ROUTE,
        "portal" => "/v1/billing/portal",
        _ => {
            return Err(agent_error(
                "unknown_billing_command",
                "Unknown billing command.",
                "Use `tovuk billing checkout --json` or `tovuk billing portal`.",
                cli.output.json,
            ));
        }
    };
    let reason = cli
        .args
        .iter()
        .skip(1)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let body = if route == BILLING_CHECKOUT_ROUTE {
        Some(json!({
            "target_plan": "pro",
            "reason": if reason.trim().is_empty() { "Upgrade to Tovuk Pro." } else { reason.trim() },
        }))
    } else {
        None
    };
    let response = api_request(cli, Method::POST, route, Some(&token), body)?;
    if cli.output.json {
        return print_json(&response);
    }
    let url = response
        .get("checkout")
        .and_then(|checkout| checkout.get("url"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    println!("{url}");
    if !url.is_empty() {
        open_url(url);
    }
    Ok(())
}

pub(crate) fn support_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => print_paged_authenticated(cli, "/v1/support/tickets"),
        "create" => support_create(cli),
        "resolve" => {
            let ticket_id = command_arg(
                cli,
                "invalid_support_ticket",
                "Support ticket id is required.",
                "Use `tovuk support resolve <ticket_id> --json` with an id from support list.",
            )?;
            print_authenticated_mutation(
                cli,
                Method::POST,
                &format!(
                    "/v1/support/tickets/{}/resolve",
                    encode_component(&ticket_id)
                ),
                None,
            )
        }
        _ => Err(agent_error(
            "unknown_command",
            "Unknown support command.",
            "Use `tovuk support list --json` or `support create` with subject and details.",
            cli.output.json,
        )),
    }
}

pub(crate) fn support_create(cli: &CliOptions) -> Result<()> {
    let subject = cli.args.get(1).cloned().unwrap_or_default();
    let details = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if subject.is_empty() || details.trim().is_empty() {
        return Err(agent_error(
            "invalid_support_ticket",
            "Support ticket subject and details are required.",
            "Use `tovuk support create \"Short subject\" \"Command, app id, build id, deploy id, and first actionable log line\" --json`.",
            cli.output.json,
        ));
    }

    let token = read_or_login_token(cli)?;
    let mut body = Map::new();
    body.insert("subject".to_owned(), Value::String(subject));
    body.insert(
        "details".to_owned(),
        Value::String(details.trim().to_owned()),
    );
    body.insert(
        "severity".to_owned(),
        Value::String(if cli.severity.is_empty() {
            "normal".to_owned()
        } else {
            cli.severity.clone()
        }),
    );
    insert_optional(&mut body, "app_id", &cli.app);
    insert_optional(&mut body, "failing_command", &cli.failing_command);
    insert_optional(&mut body, "build_id", &cli.build);
    insert_optional(&mut body, "deploy_id", &cli.deploy);
    insert_optional(&mut body, "first_log_line", &cli.first_log_line);
    let response = api_request(
        cli,
        Method::POST,
        "/v1/support/tickets",
        Some(&token),
        Some(Value::Object(body)),
    )?;
    print_json(&response)
}

pub(crate) fn print_authenticated_mutation(
    cli: &CliOptions,
    method: Method,
    route: &str,
    body: Option<Value>,
) -> Result<()> {
    let token = read_or_login_token(cli)?;
    let response = api_request(cli, method, route, Some(&token), body)?;
    print_json(&response)
}

pub(crate) fn insert_optional(body: &mut Map<String, Value>, key: &str, value: &str) {
    if !value.is_empty() {
        body.insert(key.to_owned(), Value::String(value.to_owned()));
    }
}

pub(crate) fn command_arg(
    cli: &CliOptions,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    cli.args
        .get(1)
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| agent_error(code, message, instruction, cli.output.json))
}

pub(crate) fn require_app(cli: &CliOptions) -> Result<String> {
    if cli.app.is_empty() {
        return Err(agent_error(
            "missing_app",
            "App is required.",
            "Pass `--app <app>` using either the app name from tovuk.toml or the app id printed by deploy.",
            cli.output.json,
        ));
    }
    Ok(cli.app.clone())
}

pub(crate) fn app_route(cli: &CliOptions, suffix: &str) -> Result<String> {
    Ok(format!(
        "/v1/apps/{}/{}",
        encode_component(&require_app(cli)?),
        suffix
    ))
}

pub(crate) fn page_query(cli: &CliOptions) -> String {
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
        agent_error(
            "api_unreachable",
            format!("Could not reach Tovuk API: {error}"),
            "Retry the command. If it keeps failing, check Tovuk status before changing your project.",
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

pub(crate) fn parse_json_text(text: &str) -> Value {
    if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(text).unwrap_or(Value::Null)
    }
}

pub(crate) fn agent_payload_from_json(value: &Value) -> Option<AgentErrorPayload> {
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

pub(crate) fn enrich_agent_error_payload(
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

pub(crate) fn payment_required_agent_error(
    cli: &CliOptions,
    token: &str,
    message: impl Into<String>,
    instruction: impl Into<String>,
) -> CliError {
    let mut payload = AgentErrorPayload {
        code: "payment_required".to_owned(),
        message: message.into(),
        agent_instruction: Some(instruction.into()),
        docs_url: None,
        checkout_url: None,
    };
    enrich_agent_error_payload(cli, "local:preflight", Some(token), &mut payload);
    CliError::new(CliFailure {
        payload,
        json: cli.output.json,
        exit_code: 1,
    })
}

pub(crate) fn create_checkout_url(
    cli: &CliOptions,
    token: Option<&str>,
    reason: &str,
) -> Option<String> {
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
