use super::super::{
    args::CliOptions,
    auth::read_or_login_token,
    errors::{Result, agent_error, print_json},
    project::encode_component,
};
use super::{
    common::service_route,
    generic::{print_authenticated_mutation, service_get_command},
    http::api_request,
};
use reqwest::Method;
use serde_json::{Map, Value, json};

pub(crate) fn platform_command(cli: &CliOptions) -> Result<()> {
    service_get_command(cli, "platform")
}

pub(crate) fn sqlite_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "sqlite_binding_required",
            "SQLite binding name is required.",
            "Use `tovuk database create --service <service> DB --json`.",
            "sqlite/databases",
            "name",
        ),
        "query" | "execute" => sqlite_query(cli),
        "delete" | "del" | "rm" => delete_app_resource(
            cli,
            1,
            "sqlite_database_required",
            "SQLite database is required.",
            "Use `tovuk database delete --service <service> DB --json`.",
            "sqlite/databases",
        ),
        _ => unknown_platform_command(cli, "sqlite"),
    }
}

pub(crate) fn kv_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "kv_binding_required",
            "KV binding name is required.",
            "Use `tovuk kv create --service <service> CACHE --json`.",
            "kv/namespaces",
            "name",
        ),
        "keys" => kv_keys(cli),
        "get" => kv_get(cli),
        "put" => kv_put(cli),
        "bulk" => kv_bulk_command(cli),
        "bulk-get" => kv_bulk_get(cli, 1),
        "bulk-put" => kv_bulk_put(cli, 1),
        "bulk-delete" | "bulk-del" | "bulk-rm" => kv_bulk_delete(cli, 1),
        "delete" | "del" | "rm" => kv_delete(cli),
        "namespace" | "namespaces" => kv_namespace_command(cli),
        "delete-namespace" | "remove-namespace" => delete_app_resource(
            cli,
            1,
            "kv_namespace_required",
            "KV namespace is required.",
            "Use `tovuk kv namespace delete --service <service> CACHE --json`.",
            "kv/namespaces",
        ),
        _ => unknown_platform_command(cli, "kv"),
    }
}

fn kv_bulk_command(cli: &CliOptions) -> Result<()> {
    match cli.args.get(1).map_or("", String::as_str) {
        "get" => kv_bulk_get(cli, 2),
        "put" => kv_bulk_put(cli, 2),
        "delete" | "del" | "rm" => kv_bulk_delete(cli, 2),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown KV bulk command.",
            "Use `tovuk kv bulk put --service <service> CACHE '[{\"key\":\"a\",\"value\":\"1\"}]' --json`, `tovuk kv bulk get --service <service> CACHE a b --json`, or `tovuk kv bulk delete --service <service> CACHE a b --json`.",
            cli.output.json,
        )),
    }
}

pub(crate) fn queue_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_queue(cli),
        "update" | "set" => update_queue(cli),
        "messages" => queue_messages(cli),
        "send" => queue_send(cli),
        "delete" | "del" | "rm" => delete_app_resource(
            cli,
            1,
            "queue_name_required",
            "Queue name is required.",
            "Use `tovuk queue delete --service <service> jobs --json`.",
            "queues",
        ),
        _ => unknown_platform_command(cli, "queue"),
    }
}

fn create_queue(cli: &CliOptions) -> Result<()> {
    let name = required_arg(
        cli,
        1,
        "queue_name_required",
        "Queue name is required.",
        "Use `tovuk queue create --service <service> jobs --json`.",
    )?;
    let mut body = Map::new();
    body.insert("name".to_owned(), Value::String(name));
    apply_queue_policy_options(cli, &mut body)?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &service_route(cli, "queues")?,
        Some(Value::Object(body)),
    )
}

fn update_queue(cli: &CliOptions) -> Result<()> {
    let queue = required_arg(
        cli,
        1,
        "queue_name_required",
        "Queue name is required.",
        "Use `tovuk queue update --service <service> jobs --max-batch-size 25 --json`.",
    )?;
    let mut body = Map::new();
    apply_queue_policy_options(cli, &mut body)?;
    if body.is_empty() {
        return Err(agent_error(
            "queue_update_empty",
            "Queue update has no changes.",
            "Pass at least one of `--max-retries`, `--retention-seconds`, `--max-batch-size`, `--max-batch-timeout-seconds`, or `--dead-letter-queue`.",
            cli.output.json,
        ));
    }
    let route = format!(
        "{}/queues/{}",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(&queue)
    );
    print_authenticated_mutation(cli, Method::PUT, &route, Some(Value::Object(body)))
}

fn apply_queue_policy_options(cli: &CliOptions, body: &mut Map<String, Value>) -> Result<()> {
    if let Some(max_retries) = optional_u16(&cli.queue.max_retries, "--max-retries", cli)? {
        body.insert("maxRetries".to_owned(), json!(max_retries));
    }
    if let Some(retention_seconds) =
        optional_u32(&cli.queue.retention_seconds, "--retention-seconds", cli)?
    {
        body.insert("retentionSeconds".to_owned(), json!(retention_seconds));
    }
    if let Some(max_batch_size) = optional_u16(&cli.queue.max_batch_size, "--max-batch-size", cli)?
    {
        body.insert("maxBatchSize".to_owned(), json!(max_batch_size));
    }
    if let Some(max_batch_timeout_seconds) = optional_u16(
        &cli.queue.max_batch_timeout_seconds,
        "--max-batch-timeout-seconds",
        cli,
    )? {
        body.insert(
            "maxBatchTimeoutSeconds".to_owned(),
            json!(max_batch_timeout_seconds),
        );
    }
    if cli.queue.clear_dead_letter_queue && !cli.queue.dead_letter_queue.is_empty() {
        return Err(agent_error(
            "queue_dead_letter_conflict",
            "Queue dead-letter options conflict.",
            "Use either `--dead-letter-queue <queue>` or `--clear-dead-letter-queue`, not both.",
            cli.output.json,
        ));
    }
    if cli.queue.clear_dead_letter_queue {
        body.insert("deadLetterQueue".to_owned(), Value::Null);
    } else if !cli.queue.dead_letter_queue.is_empty() {
        body.insert(
            "deadLetterQueue".to_owned(),
            Value::String(cli.queue.dead_letter_queue.clone()),
        );
    }
    Ok(())
}

pub(crate) fn cron_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_cron(cli),
        "delete" | "del" | "rm" => delete_app_resource(
            cli,
            1,
            "cron_name_required",
            "Cron trigger name is required.",
            "Use `tovuk cron delete --service <service> nightly --json`.",
            "cron",
        ),
        _ => unknown_platform_command(cli, "cron"),
    }
}

pub(crate) fn durable_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_app_resource(
            cli,
            "durable_class_required",
            "Durable Object class name is required.",
            "Use `tovuk durable-object create --service <service> Room --json`.",
            "durable-objects/namespaces",
            "className",
        ),
        "delete" | "del" | "rm" => delete_app_resource(
            cli,
            1,
            "durable_class_required",
            "Durable Object class name is required.",
            "Use `tovuk durable-object delete --service <service> Room --json`.",
            "durable-objects/namespaces",
        ),
        _ => unknown_platform_command(cli, "durable"),
    }
}

pub(crate) fn binding_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("list", String::as_str) {
        "list" => platform_command(cli),
        "create" => create_service_binding(cli),
        "delete" | "del" | "rm" => delete_app_resource(
            cli,
            1,
            "binding_name_required",
            "Service binding name is required.",
            "Use `tovuk binding delete --service <service> AUTH_SERVICE --json`.",
            "service-bindings",
        ),
        _ => unknown_platform_command(cli, "binding"),
    }
}

pub(crate) fn caps_command(cli: &CliOptions) -> Result<()> {
    match cli.args.first().map_or("", String::as_str) {
        "set" => set_usage_cap(cli),
        "delete" | "del" | "rm" => delete_usage_cap(cli),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown usage cap command.",
            "Use `tovuk caps set worker_requests --period day --value 100000 --json` or `tovuk caps delete worker_requests --period day --json`.",
            cli.output.json,
        )),
    }
}

fn create_app_resource(
    cli: &CliOptions,
    code: &str,
    message: &str,
    instruction: &str,
    suffix: &str,
    body_key: &str,
) -> Result<()> {
    let name = required_arg(cli, 1, code, message, instruction)?;
    let mut body = Map::new();
    body.insert(body_key.to_owned(), Value::String(name));
    print_authenticated_mutation(
        cli,
        Method::POST,
        &service_route(cli, suffix)?,
        Some(Value::Object(body)),
    )
}

fn delete_app_resource(
    cli: &CliOptions,
    arg_index: usize,
    code: &str,
    message: &str,
    instruction: &str,
    suffix: &str,
) -> Result<()> {
    let resource = required_arg(cli, arg_index, code, message, instruction)?;
    print_authenticated_mutation(
        cli,
        Method::DELETE,
        &format!(
            "{}/{}",
            service_route(cli, suffix)?.trim_end_matches('/'),
            encode_component(&resource)
        ),
        None,
    )
}

fn sqlite_query(cli: &CliOptions) -> Result<()> {
    let database = required_arg(
        cli,
        1,
        "sqlite_database_required",
        "SQLite database is required.",
        "Use `tovuk database query --service <service> DB \"select 1\" --json`.",
    )?;
    let sql = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if sql.trim().is_empty() {
        return Err(agent_error(
            "sqlite_sql_required",
            "SQLite SQL is required.",
            "Pass one SQL statement after the database name.",
            cli.output.json,
        ));
    }
    let params = sqlite_params(cli)?;
    let route = format!(
        "{}/sqlite/{}/query",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(&database)
    );
    print_authenticated_mutation(
        cli,
        Method::POST,
        &route,
        Some(json!({
            "sql": sql,
            "params": params,
        })),
    )
}

fn sqlite_params(cli: &CliOptions) -> Result<Vec<Value>> {
    if cli.params.trim().is_empty() {
        return Ok(Vec::new());
    }
    let value = serde_json::from_str::<Value>(&cli.params).map_err(|_error| {
        agent_error(
            "invalid_sqlite_params",
            "SQLite params must be a JSON array.",
            "Pass params as JSON such as `--params '[1,\"Ada\"]'`.",
            cli.output.json,
        )
    })?;
    match value {
        Value::Array(values) => Ok(values),
        _other => Err(agent_error(
            "invalid_sqlite_params",
            "SQLite params must be a JSON array.",
            "Pass params as JSON such as `--params '[1,\"Ada\"]'`.",
            cli.output.json,
        )),
    }
}

fn kv_namespace_command(cli: &CliOptions) -> Result<()> {
    match cli.args.get(1).map_or("", String::as_str) {
        "delete" | "del" | "rm" => delete_app_resource(
            cli,
            2,
            "kv_namespace_required",
            "KV namespace is required.",
            "Use `tovuk kv namespace delete --service <service> CACHE --json`.",
            "kv/namespaces",
        ),
        _ => Err(agent_error(
            "unknown_command",
            "Unknown KV namespace command.",
            "Use `tovuk kv namespace delete --service <service> CACHE --json`.",
            cli.output.json,
        )),
    }
}

fn kv_keys(cli: &CliOptions) -> Result<()> {
    let namespace = required_arg(
        cli,
        1,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv keys --service <service> CACHE --json`.",
    )?;
    let token = read_or_login_token(cli)?;
    let route = format!(
        "{}/kv/{}/keys",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(&namespace)
    );
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    print_json(&response)
}

fn kv_get(cli: &CliOptions) -> Result<()> {
    let namespace = required_arg(
        cli,
        1,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv get --service <service> CACHE user:1 --json`.",
    )?;
    let key = required_arg(
        cli,
        2,
        "kv_key_required",
        "KV key is required.",
        "Use `tovuk kv get --service <service> CACHE user:1 --json`.",
    )?;
    print_authenticated_mutation(
        cli,
        Method::GET,
        &kv_value_route(cli, &namespace, &key)?,
        None,
    )
}

fn kv_put(cli: &CliOptions) -> Result<()> {
    let namespace = required_arg(
        cli,
        1,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv put --service <service> CACHE user:1 '{\"name\":\"Ada\"}' --json`.",
    )?;
    let key = required_arg(
        cli,
        2,
        "kv_key_required",
        "KV key is required.",
        "Use `tovuk kv put --service <service> CACHE user:1 '{\"name\":\"Ada\"}' --json`.",
    )?;
    let value = if cli.value.is_empty() {
        cli.args
            .iter()
            .skip(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        cli.value.clone()
    };
    if value.is_empty() {
        return Err(agent_error(
            "kv_value_required",
            "KV value is required.",
            "Pass the value as the final argument or with `--value <value>`.",
            cli.output.json,
        ));
    }
    let mut payload = Map::new();
    payload.insert("value".to_owned(), Value::String(value));
    payload.insert("encoding".to_owned(), Value::String("text".to_owned()));
    if !cli.kv.metadata.trim().is_empty() {
        payload.insert("metadata".to_owned(), kv_metadata(cli)?);
    }
    if let Some(ttl) = optional_u32(&cli.kv.expiration_ttl_seconds, "--ttl", cli)? {
        payload.insert("expirationTtlSeconds".to_owned(), json!(ttl));
    }
    print_authenticated_mutation(
        cli,
        Method::PUT,
        &kv_value_route(cli, &namespace, &key)?,
        Some(Value::Object(payload)),
    )
}

fn kv_metadata(cli: &CliOptions) -> Result<Value> {
    let value = serde_json::from_str::<Value>(&cli.kv.metadata).map_err(|_error| {
        agent_error(
            "invalid_kv_metadata",
            "KV metadata must be valid JSON.",
            "Pass metadata as JSON such as `--metadata '{\"cache\":\"user\"}'`.",
            cli.output.json,
        )
    })?;
    if !value.is_object() {
        return Err(agent_error(
            "invalid_kv_metadata",
            "KV metadata must be a JSON object.",
            "Pass metadata as JSON such as `--metadata '{\"cache\":\"user\"}'`.",
            cli.output.json,
        ));
    }
    Ok(value)
}

fn kv_delete(cli: &CliOptions) -> Result<()> {
    let namespace = required_arg(
        cli,
        1,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv delete --service <service> CACHE user:1 --json`.",
    )?;
    let key = required_arg(
        cli,
        2,
        "kv_key_required",
        "KV key is required.",
        "Use `tovuk kv delete --service <service> CACHE user:1 --json`.",
    )?;
    print_authenticated_mutation(
        cli,
        Method::DELETE,
        &kv_value_route(cli, &namespace, &key)?,
        None,
    )
}

fn kv_bulk_get(cli: &CliOptions, namespace_index: usize) -> Result<()> {
    let namespace = required_arg(
        cli,
        namespace_index,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv bulk get --service <service> CACHE key-a key-b --json`.",
    )?;
    let keys = kv_bulk_keys(cli, namespace_index + 1)?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &kv_bulk_route(cli, &namespace, "")?,
        Some(json!({ "keys": keys })),
    )
}

fn kv_bulk_put(cli: &CliOptions, namespace_index: usize) -> Result<()> {
    let namespace = required_arg(
        cli,
        namespace_index,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv bulk put --service <service> CACHE '[{\"key\":\"a\",\"value\":\"1\"}]' --json`.",
    )?;
    let raw = raw_bulk_json(
        cli,
        namespace_index + 1,
        "KV bulk entries are required.",
        "Pass JSON entries as `--value '[{\"key\":\"a\",\"value\":\"1\"}]'` or as the final positional argument.",
    )?;
    let payload = kv_bulk_put_payload(&raw, cli)?;
    print_authenticated_mutation(
        cli,
        Method::PUT,
        &kv_bulk_route(cli, &namespace, "")?,
        Some(payload),
    )
}

fn kv_bulk_delete(cli: &CliOptions, namespace_index: usize) -> Result<()> {
    let namespace = required_arg(
        cli,
        namespace_index,
        "kv_namespace_required",
        "KV namespace is required.",
        "Use `tovuk kv bulk delete --service <service> CACHE key-a key-b --json`.",
    )?;
    let keys = kv_bulk_keys(cli, namespace_index + 1)?;
    print_authenticated_mutation(
        cli,
        Method::POST,
        &kv_bulk_route(cli, &namespace, "removals")?,
        Some(json!({ "keys": keys })),
    )
}

fn kv_bulk_keys(cli: &CliOptions, first_key_index: usize) -> Result<Vec<String>> {
    if !cli.value.trim().is_empty() {
        return parse_kv_bulk_keys_json(&cli.value, cli);
    }
    let keys = cli
        .args
        .iter()
        .skip(first_key_index)
        .filter(|key| !key.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(agent_error(
            "kv_keys_required",
            "KV keys are required.",
            "Pass keys as positional arguments or with `--value '[\"key-a\",\"key-b\"]'`.",
            cli.output.json,
        ));
    }
    Ok(keys)
}

fn parse_kv_bulk_keys_json(source: &str, cli: &CliOptions) -> Result<Vec<String>> {
    let value = serde_json::from_str::<Value>(source).map_err(|_error| {
        agent_error(
            "invalid_kv_keys",
            "KV keys JSON is invalid.",
            "Pass keys as JSON such as `--value '[\"key-a\",\"key-b\"]'`.",
            cli.output.json,
        )
    })?;
    let keys = match value {
        Value::Array(items) => items,
        Value::Object(mut object) => match object.remove("keys") {
            Some(Value::Array(items)) => items,
            _other => {
                return Err(agent_error(
                    "invalid_kv_keys",
                    "KV keys JSON must contain a keys array.",
                    "Pass keys as JSON such as `--value '{\"keys\":[\"key-a\",\"key-b\"]}'`.",
                    cli.output.json,
                ));
            }
        },
        _other => {
            return Err(agent_error(
                "invalid_kv_keys",
                "KV keys JSON must be an array or object.",
                "Pass keys as JSON such as `--value '[\"key-a\",\"key-b\"]'`.",
                cli.output.json,
            ));
        }
    };
    keys.into_iter()
        .map(|item| match item {
            Value::String(key) if !key.trim().is_empty() => Ok(key),
            _other => Err(agent_error(
                "invalid_kv_keys",
                "KV keys must be strings.",
                "Pass keys as JSON strings.",
                cli.output.json,
            )),
        })
        .collect()
}

fn raw_bulk_json(
    cli: &CliOptions,
    first_value_index: usize,
    message: &'static str,
    instruction: &'static str,
) -> Result<String> {
    let raw = if cli.value.trim().is_empty() {
        cli.args
            .iter()
            .skip(first_value_index)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        cli.value.clone()
    };
    if raw.trim().is_empty() {
        return Err(agent_error(
            "kv_bulk_json_required",
            message,
            instruction,
            cli.output.json,
        ));
    }
    Ok(raw)
}

fn kv_bulk_put_payload(source: &str, cli: &CliOptions) -> Result<Value> {
    let value = serde_json::from_str::<Value>(source).map_err(|_error| {
        agent_error(
            "invalid_kv_bulk_entries",
            "KV bulk entries JSON is invalid.",
            "Pass an entries array such as `--value '[{\"key\":\"a\",\"value\":\"1\"}]'`.",
            cli.output.json,
        )
    })?;
    match value {
        Value::Array(entries) => Ok(json!({ "entries": entries })),
        Value::Object(object) if object.contains_key("entries") => Ok(Value::Object(object)),
        Value::Object(object) => {
            let entries = object
                .into_iter()
                .map(|(key, value)| match value {
                    Value::String(text) => json!({ "key": key, "value": text, "encoding": "text" }),
                    other => json!({ "key": key, "value": other.to_string(), "encoding": "text" }),
                })
                .collect::<Vec<_>>();
            Ok(json!({ "entries": entries }))
        }
        _other => Err(agent_error(
            "invalid_kv_bulk_entries",
            "KV bulk entries must be a JSON array or object.",
            "Pass an entries array such as `--value '[{\"key\":\"a\",\"value\":\"1\"}]'`.",
            cli.output.json,
        )),
    }
}

fn queue_messages(cli: &CliOptions) -> Result<()> {
    let queue = required_arg(
        cli,
        1,
        "queue_name_required",
        "Queue name is required.",
        "Use `tovuk queue messages --service <service> jobs --json`.",
    )?;
    let token = read_or_login_token(cli)?;
    let route = format!(
        "{}/queues/{}/messages",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(&queue)
    );
    let response = api_request(cli, Method::GET, &route, Some(&token), None)?;
    print_json(&response)
}

fn queue_send(cli: &CliOptions) -> Result<()> {
    let queue = required_arg(
        cli,
        1,
        "queue_name_required",
        "Queue name is required.",
        "Use `tovuk queue send --service <service> jobs '{\"task\":\"sync\"}' --json`.",
    )?;
    let body = if cli.value.is_empty() {
        cli.args
            .iter()
            .skip(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        cli.value.clone()
    };
    if body.is_empty() {
        return Err(agent_error(
            "queue_body_required",
            "Queue message body is required.",
            "Pass the body as the final argument or with `--value <value>`.",
            cli.output.json,
        ));
    }
    let route = format!(
        "{}/queues/{}/messages",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(&queue)
    );
    let mut payload = Map::new();
    payload.insert("body".to_owned(), Value::String(body));
    payload.insert("encoding".to_owned(), Value::String("text".to_owned()));
    if let Some(delay_seconds) = optional_u32(&cli.queue.delay_seconds, "--delay-seconds", cli)? {
        payload.insert("delaySeconds".to_owned(), json!(delay_seconds));
    }
    print_authenticated_mutation(cli, Method::POST, &route, Some(Value::Object(payload)))
}

fn optional_u16(value: &str, flag: &str, cli: &CliOptions) -> Result<Option<u16>> {
    let Some(number) = optional_u64(value, flag, cli)? else {
        return Ok(None);
    };
    u16::try_from(number).map(Some).map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{flag} is too large."),
            format!("Pass {flag} within the documented platform limit."),
            cli.output.json,
        )
    })
}

fn optional_u32(value: &str, flag: &str, cli: &CliOptions) -> Result<Option<u32>> {
    let Some(number) = optional_u64(value, flag, cli)? else {
        return Ok(None);
    };
    u32::try_from(number).map(Some).map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{flag} is too large."),
            format!("Pass {flag} within the documented platform limit."),
            cli.output.json,
        )
    })
}

fn optional_u64(value: &str, flag: &str, cli: &CliOptions) -> Result<Option<u64>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed.parse::<u64>().map(Some).map_err(|_error| {
        agent_error(
            "invalid_argument",
            format!("{flag} must be a non-negative integer."),
            format!("Pass {flag} as seconds or a count."),
            cli.output.json,
        )
    })
}

fn kv_value_route(cli: &CliOptions, namespace: &str, key: &str) -> Result<String> {
    Ok(format!(
        "{}/kv/{}/values/{}",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(namespace),
        encode_component(key)
    ))
}

fn kv_bulk_route(cli: &CliOptions, namespace: &str, suffix: &str) -> Result<String> {
    let route = format!(
        "{}/kv/{}/bulk",
        service_route(cli, "")?.trim_end_matches('/'),
        encode_component(namespace)
    );
    if suffix.is_empty() {
        Ok(route)
    } else {
        Ok(format!("{route}/{}", encode_component(suffix)))
    }
}

fn create_cron(cli: &CliOptions) -> Result<()> {
    let name = required_arg(
        cli,
        1,
        "cron_name_required",
        "Cron trigger name is required.",
        "Use `tovuk cron create --service <service> nightly \"0 0 * * *\" --json`.",
    )?;
    let cron_expression = cli
        .args
        .iter()
        .skip(2)
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    if cron_expression.trim().is_empty() {
        return Err(agent_error(
            "cron_expression_required",
            "Cron expression is required.",
            "Use `tovuk cron create --service <service> nightly \"0 0 * * *\" --json`.",
            cli.output.json,
        ));
    }
    print_authenticated_mutation(
        cli,
        Method::POST,
        &service_route(cli, "cron")?,
        Some(json!({
            "name": name,
            "cronExpression": cron_expression,
        })),
    )
}

fn create_service_binding(cli: &CliOptions) -> Result<()> {
    let binding_name = required_arg(
        cli,
        1,
        "binding_name_required",
        "Service binding name is required.",
        "Use `tovuk binding create --service <service> AUTH_SERVICE --target <target_service> --json`.",
    )?;
    let target_service = if cli.target.is_empty() {
        required_arg(
            cli,
            2,
            "binding_target_required",
            "Service binding target service is required.",
            "Use `tovuk binding create --service <service> AUTH_SERVICE --target <target_service> --json`.",
        )?
    } else {
        cli.target.clone()
    };
    print_authenticated_mutation(
        cli,
        Method::POST,
        &service_route(cli, "service-bindings")?,
        Some(json!({
            "bindingName": binding_name,
            "targetService": target_service,
        })),
    )
}

fn set_usage_cap(cli: &CliOptions) -> Result<()> {
    let metric = required_arg(
        cli,
        1,
        "cap_metric_required",
        "Usage cap metric is required.",
        "Use `tovuk caps set worker_requests --period day --value 100000 --json`.",
    )?;
    let period = if cli.period.is_empty() {
        cli.args.get(2).cloned().unwrap_or_default()
    } else {
        cli.period.clone()
    };
    let value = if cli.value.is_empty() {
        cli.args.get(3).cloned().unwrap_or_default()
    } else {
        cli.value.clone()
    };
    if period.is_empty() || value.is_empty() {
        return Err(agent_error(
            "cap_period_or_value_required",
            "Usage cap period and value are required.",
            "Use `tovuk caps set worker_requests --period day --value 100000 --json`.",
            cli.output.json,
        ));
    }
    let cap_value = value.parse::<u64>().map_err(|_error| {
        agent_error(
            "invalid_cap_value",
            "Usage cap value must be a positive integer.",
            "Pass an integer value such as `100000`, then retry.",
            cli.output.json,
        )
    })?;
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::PUT,
        &format!("/v1/usage/caps/{}", encode_component(&metric)),
        Some(&token),
        Some(json!({
            "period": period,
            "capValue": cap_value,
            "hardStop": true,
            "notifyAtPercent": 80,
        })),
    )?;
    print_json(&response)
}

fn delete_usage_cap(cli: &CliOptions) -> Result<()> {
    let metric = required_arg(
        cli,
        1,
        "cap_metric_required",
        "Usage cap metric is required.",
        "Use `tovuk caps delete worker_requests --period day --json`.",
    )?;
    let period = if cli.period.is_empty() {
        cli.args.get(2).cloned().unwrap_or_default()
    } else {
        cli.period.clone()
    };
    if period.is_empty() {
        return Err(agent_error(
            "cap_period_required",
            "Usage cap period is required.",
            "Use `tovuk caps delete worker_requests --period day --json`.",
            cli.output.json,
        ));
    }
    let token = read_or_login_token(cli)?;
    let response = api_request(
        cli,
        Method::DELETE,
        &format!(
            "/v1/usage/caps/{}/{}",
            encode_component(&metric),
            encode_component(&period)
        ),
        Some(&token),
        None,
    )?;
    print_json(&response)
}

fn required_arg(
    cli: &CliOptions,
    index: usize,
    code: &str,
    message: &str,
    instruction: &str,
) -> Result<String> {
    cli.args
        .get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| agent_error(code, message, instruction, cli.output.json))
}

fn unknown_platform_command(cli: &CliOptions, family: &str) -> Result<()> {
    Err(agent_error(
        "unknown_command",
        format!("Unknown {family} command."),
        "Use `list` or `create`, then retry with `--json` for agent-readable output.",
        cli.output.json,
    ))
}
