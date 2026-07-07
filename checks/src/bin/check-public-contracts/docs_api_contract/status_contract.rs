use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_json_response_example_check_name,
    require_json_response_example_check_name, require_json_response_example_check_nested_u64,
    require_json_response_example_check_u64, require_json_response_example_string,
    require_schema_properties,
};

pub(super) fn require_openapi_status_checks(openapi: &OpenApi) -> CheckResult {
    require_schema_properties(
        openapi_schema(openapi, "StatusResponse")?,
        &[
            "ok",
            "service",
            "name",
            "api_version",
            "checks",
            "agent_instruction",
            "docs_url",
        ],
        "OpenAPI status response schema",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "StatusCheck")?,
        &["name", "ok", "message", "latency_ms", "details"],
        "OpenAPI status check schema",
    )?;
    require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "control_plane_postgres",
        "OpenAPI status control-plane PostgreSQL check",
    )?;
    require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "redis",
        "OpenAPI status Redis check",
    )?;
    require_json_response_example_check_u64(
        openapi,
        "StatusLoaded",
        "control_plane_postgres",
        "latency_ms",
        "OpenAPI status control-plane PostgreSQL latency",
    )?;
    require_postgres_pool_details(openapi, "StatusLoaded")?;
    require_json_response_example_check_u64(
        openapi,
        "StatusLoaded",
        "redis",
        "latency_ms",
        "OpenAPI status Redis latency",
    )?;
    require_json_response_example_check_u64(
        openapi,
        "StatusUnavailable",
        "control_plane_postgres",
        "latency_ms",
        "OpenAPI unavailable status control-plane PostgreSQL latency",
    )?;
    require_postgres_pool_details(openapi, "StatusUnavailable")?;
    require_json_response_example_check_u64(
        openapi,
        "StatusUnavailable",
        "redis",
        "latency_ms",
        "OpenAPI unavailable status Redis latency",
    )?;
    require_json_response_example_string(
        openapi,
        "StatusLoaded",
        &["service"],
        "tovuk-api",
        "OpenAPI loaded status service",
    )?;
    require_json_response_example_string(
        openapi,
        "StatusUnavailable",
        &["service"],
        "tovuk-api",
        "OpenAPI unavailable status service",
    )?;
    require_json_response_example_string(
        openapi,
        "StatusLoaded",
        &["agent_instruction"],
        "Continue with GET /v1/capabilities, then run the requested Tovuk command.",
        "OpenAPI loaded status agent instruction",
    )?;
    require_json_response_example_string(
        openapi,
        "StatusUnavailable",
        &["agent_instruction"],
        "Retry the command. If it keeps failing, check Tovuk status before changing your request.",
        "OpenAPI unavailable status agent instruction",
    )?;
    reject_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "database",
        "OpenAPI status must not expose generic database product wording",
    )
}

fn require_postgres_pool_details(openapi: &OpenApi, response_name: &str) -> CheckResult {
    for field in [
        "pool_max_connections",
        "pool_open_connections",
        "pool_idle_connections",
        "pool_busy_connections",
    ] {
        require_json_response_example_check_nested_u64(
            openapi,
            response_name,
            "control_plane_postgres",
            &["details", field],
            "OpenAPI status control-plane PostgreSQL pool details",
        )?;
    }
    Ok(())
}
