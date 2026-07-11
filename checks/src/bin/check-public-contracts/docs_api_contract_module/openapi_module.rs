/// Public contract checks for examples.
#[path = "openapi_module/examples.rs"]
pub mod examples;

/// Public contract checks for operations.
#[path = "openapi_module/operations.rs"]
pub mod operations;

/// Public contract checks for schemas.
#[path = "openapi_module/schemas.rs"]
pub mod schemas;

pub(in crate::docs_api_contract) use examples::{
    require_example_string, require_json_response_example_check_name,
    require_json_response_example_check_u64, require_json_response_example_string,
};

pub(in crate::docs_api_contract) use operations::{
    reject_operation_field, require_operation_id, require_operation_response_ref,
    require_parameter_bounds,
};

pub(in crate::docs_api_contract) use schemas::{
    reject_numeric_property_anywhere, reject_schema, reject_schema_property,
    reject_schema_property_enum, require_named_schema_properties, require_schema_properties,
    require_schema_property_enum, require_schema_property_example_u64,
};

use crate::helpers::CheckResult;

use serde_json::{Map, Value, from_str};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&openapi_document),
    size_of_val(&openapi_paths),
    size_of_val(&openapi_response),
    size_of_val(&openapi_responses),
];

/// JSON object used by `OpenAPI` component maps.
type JsonObject = Map<String, Value>;

/// Contract representation for `OpenApi`.
pub(in crate::docs_api_contract) type OpenApi = Value;

/// Contract implementation for `openapi_document`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn openapi_document(openapi: &str) -> CheckResult<OpenApi> {
    return from_str(openapi)
        .map_err(|error| format!("docs/openapi.json must be valid JSON: {error}"));
}

/// Contract implementation for `openapi_operation`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn openapi_operation<'document>(
    openapi: &'document OpenApi,
    path_name: &str,
    method: &str,
) -> CheckResult<&'document Value> {
    return check_try!(openapi_path(openapi, path_name))
        .get(method)
        .ok_or_else(|| format!("OpenAPI {method} {path_name} operation is missing"));
}

/// Contract implementation for `openapi_path`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn openapi_path<'document>(
    openapi: &'document OpenApi,
    path_name: &str,
) -> CheckResult<&'document Value> {
    return check_try!(openapi_paths(openapi))
        .get(path_name)
        .ok_or_else(|| format!("OpenAPI path {path_name} was missing"));
}

/// Contract implementation for `openapi_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn openapi_paths(openapi: &OpenApi) -> CheckResult<&JsonObject> {
    return openapi
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| return "OpenAPI paths object is missing".to_owned());
}

/// Contract implementation for `openapi_response`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn openapi_response<'document>(
    openapi: &'document OpenApi,
    response_name: &str,
) -> CheckResult<&'document Value> {
    return check_try!(openapi_responses(openapi))
        .get(response_name)
        .ok_or_else(|| format!("OpenAPI response {response_name} was missing"));
}

/// Contract implementation for `openapi_responses`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn openapi_responses(openapi: &OpenApi) -> CheckResult<&JsonObject> {
    return openapi
        .get("components")
        .and_then(|components| return components.get("responses"))
        .and_then(Value::as_object)
        .ok_or_else(|| return "OpenAPI components.responses object is missing".to_owned());
}

/// Contract implementation for `openapi_schema`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn openapi_schema<'document>(
    openapi: &'document OpenApi,
    schema_name: &str,
) -> CheckResult<&'document Value> {
    return check_try!(openapi_schemas(openapi))
        .get(schema_name)
        .ok_or_else(|| format!("OpenAPI schema {schema_name} was missing"));
}

/// Contract implementation for `openapi_schemas`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn openapi_schemas(openapi: &OpenApi) -> CheckResult<&JsonObject> {
    return openapi
        .get("components")
        .and_then(|components| return components.get("schemas"))
        .and_then(Value::as_object)
        .ok_or_else(|| return "OpenAPI components.schemas object is missing".to_owned());
}

/// Contract implementation for `schema_properties`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn schema_properties<'document>(
    schema: &'document Value,
    label: &str,
) -> CheckResult<&'document JsonObject> {
    return schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{label} properties object is missing"));
}
