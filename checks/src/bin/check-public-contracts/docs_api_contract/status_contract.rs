use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_json_response_example_check_name,
    require_json_response_example_check_name, require_json_response_example_check_nested_u64,
    require_json_response_example_check_u64, require_json_response_example_string,
    require_schema_properties,
};

pub(super) fn require_openapi_status_checks(openapi: &OpenApi) -> CheckResult {
    require_status_response_schemas(openapi)?;
    require_status_check_names(openapi)?;
    require_status_latency_examples(openapi)?;
    require_status_top_level_examples(openapi)?;
    reject_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "database",
        "OpenAPI status must not expose generic database product wording",
    )
}

fn require_status_response_schemas(openapi: &OpenApi) -> CheckResult {
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
        &[
            "name",
            "ok",
            "message",
            "latency_ms",
            "latency_us",
            "details",
        ],
        "OpenAPI status check schema",
    )
}

fn require_status_check_names(openapi: &OpenApi) -> CheckResult {
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
    )
}

fn require_status_latency_examples(openapi: &OpenApi) -> CheckResult {
    for response_name in ["StatusLoaded", "StatusUnavailable"] {
        require_dependency_latency_examples(openapi, response_name, "control_plane_postgres")?;
        require_postgres_pool_details(openapi, response_name)?;
        require_dependency_latency_examples(openapi, response_name, "redis")?;
    }
    Ok(())
}

fn require_dependency_latency_examples(
    openapi: &OpenApi,
    response_name: &str,
    check_name: &str,
) -> CheckResult {
    require_json_response_example_check_u64(
        openapi,
        response_name,
        check_name,
        "latency_ms",
        "OpenAPI status millisecond latency",
    )?;
    require_json_response_example_check_u64(
        openapi,
        response_name,
        check_name,
        "latency_us",
        "OpenAPI status microsecond latency",
    )
}

fn require_status_top_level_examples(openapi: &OpenApi) -> CheckResult {
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
