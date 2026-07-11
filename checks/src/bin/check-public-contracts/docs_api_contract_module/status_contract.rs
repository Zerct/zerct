use crate::helpers::CheckResult;

use serde_json::Value;

use super::openapi::{
    OpenApi, openapi_response, openapi_schema, require_json_response_example_check_name,
    require_json_response_example_check_u64, require_json_response_example_string,
    require_schema_properties,
};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0008] = [
    size_of_val(&reject_private_status_names),
    size_of_val(&require_dependency_latency_examples),
    size_of_val(&require_bounded_diagnostic_schema),
    size_of_val(&require_openapi_status_checks),
    size_of_val(&require_status_check_names),
    size_of_val(&require_status_latency_examples),
    size_of_val(&require_status_top_level_examples),
    size_of_val(&status_example_checks),
];

/// Reject concrete infrastructure names from public status examples.
///
/// # Errors
///
/// Returns an error when a private implementation name is present.
fn reject_private_status_names(openapi: &OpenApi) -> CheckResult {
    for response_name in ["StatusLoaded", "StatusUnavailable"] {
        let invalid_name = check_try!(status_example_checks(openapi, response_name))
            .iter()
            .filter_map(|check| return check.get("name").and_then(Value::as_str))
            .find(|name| return !matches!(*name, "api" | "data_sources"));
        if let Some(name) = invalid_name {
            return Err(format!(
                "OpenAPI public status check name {name:?} is not an allowed public category"
            ));
        }
    }
    return Ok(());
}

/// Require bounded, topology-free public diagnostic metadata.
///
/// # Errors
///
/// Returns an error when the status diagnostic schema is absent or unbounded.
fn require_bounded_diagnostic_schema(openapi: &OpenApi) -> CheckResult {
    let status_check = check_try!(openapi_schema(openapi, "StatusCheck"));
    let details = check_try!(
        status_check
            .get("properties")
            .and_then(|properties| return properties.get("details"))
            .ok_or_else(|| return "OpenAPI status diagnostic schema is missing".to_owned())
    );
    let is_object = details.get("type").and_then(Value::as_str) == Some("object");
    let rejects_unknown_fields =
        details.get("additionalProperties").and_then(Value::as_bool) == Some(false);
    let bounded_properties = details
        .get("maxProperties")
        .and_then(Value::as_u64)
        .is_some_and(|maximum| return maximum <= 0x0008);
    if is_object && rejects_unknown_fields && bounded_properties {
        return Ok(());
    }
    return Err(
        "OpenAPI status diagnostics must be a bounded object without arbitrary fields".to_owned(),
    );
}

/// Require both public latency units for one status check example.
///
/// # Errors
///
/// Returns an error when either latency value is missing.
fn require_dependency_latency_examples(
    openapi: &OpenApi,
    response_name: &str,
    check_name: &str,
) -> CheckResult {
    check_try!(require_json_response_example_check_u64(
        openapi,
        (response_name, check_name, "latency_ms"),
        "OpenAPI status millisecond latency",
    ));
    return require_json_response_example_check_u64(
        openapi,
        (response_name, check_name, "latency_us"),
        "OpenAPI status microsecond latency",
    );
}

/// Check the public `OpenAPI` status contract.
///
/// # Errors
///
/// Returns an error when a status schema or example violates the contract.
pub(super) fn require_openapi_status_checks(openapi: &OpenApi) -> CheckResult {
    check_try!(require_schema_properties(
        check_try!(openapi_schema(openapi, "StatusResponse")),
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
    ));
    check_try!(require_schema_properties(
        check_try!(openapi_schema(openapi, "StatusCheck")),
        &[
            "name",
            "ok",
            "message",
            "latency_ms",
            "latency_us",
            "details",
        ],
        "OpenAPI status check schema",
    ));
    check_try!(require_bounded_diagnostic_schema(openapi));
    check_try!(require_status_check_names(openapi));
    check_try!(require_status_latency_examples(openapi));
    check_try!(require_status_top_level_examples(openapi));
    return reject_private_status_names(openapi);
}

/// Require stable, implementation-neutral public check names.
///
/// # Errors
///
/// Returns an error when a generic public check example is missing.
pub(super) fn require_status_check_names(openapi: &OpenApi) -> CheckResult {
    check_try!(require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "api",
        "OpenAPI status API check",
    ));
    return require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "data_sources",
        "OpenAPI status data-source check",
    );
}

/// Require latency examples for every stable public status check.
///
/// # Errors
///
/// Returns an error when a loaded or unavailable example lacks latency data.
pub(super) fn require_status_latency_examples(openapi: &OpenApi) -> CheckResult {
    for response_name in ["StatusLoaded", "StatusUnavailable"] {
        check_try!(require_dependency_latency_examples(
            openapi,
            response_name,
            "api"
        ));
        check_try!(require_dependency_latency_examples(
            openapi,
            response_name,
            "data_sources"
        ));
    }
    return Ok(());
}

/// Require stable top-level status example values.
///
/// # Errors
///
/// Returns an error when a service or agent-instruction example drifts.
pub(super) fn require_status_top_level_examples(openapi: &OpenApi) -> CheckResult {
    check_try!(require_json_response_example_string(
        openapi,
        ("StatusLoaded", &["service"]),
        "tovuk-api",
        "OpenAPI loaded status service",
    ));
    check_try!(require_json_response_example_string(
        openapi,
        ("StatusUnavailable", &["service"]),
        "tovuk-api",
        "OpenAPI unavailable status service",
    ));
    check_try!(require_json_response_example_string(
        openapi,
        ("StatusLoaded", &["agent_instruction"]),
        "Continue with GET /v1/capabilities, then run the requested Tovuk command.",
        "OpenAPI loaded status agent instruction",
    ));
    return require_json_response_example_string(
        openapi,
        ("StatusUnavailable", &["agent_instruction"]),
        "Retry the command. If it keeps failing, check Tovuk status before changing your request.",
        "OpenAPI unavailable status agent instruction",
    );
}
/// Return status checks from one named response example.
///
/// # Errors
///
/// Returns an error when the public status example has no checks array.
fn status_example_checks<'document>(
    openapi: &'document OpenApi,
    response_name: &str,
) -> CheckResult<&'document [Value]> {
    return check_try!(openapi_response(openapi, response_name))
        .get("content")
        .and_then(|content| return content.get("application/json"))
        .and_then(|json| return json.get("examples"))
        .and_then(|examples| return examples.get("example"))
        .and_then(|example| return example.get("value"))
        .and_then(|value| return value.get("checks"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("OpenAPI {response_name} status checks are missing"));
}
