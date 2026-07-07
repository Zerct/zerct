use serde_json::{Map, Value};

use crate::helpers::CheckResult;

mod examples;
mod operations;
mod schemas;

pub(in crate::docs_api_contract) use examples::{
    reject_json_response_example_check_name, require_example_string,
    require_json_response_example_check_name, require_json_response_example_check_nested_u64,
    require_json_response_example_check_u64, require_json_response_example_string,
};
pub(in crate::docs_api_contract) use operations::{
    reject_operation_field, require_operation_id, require_parameter_bounds,
};
pub(in crate::docs_api_contract) use schemas::{
    reject_numeric_property_anywhere, reject_schema, reject_schema_property,
    reject_schema_property_enum, require_schema_properties, require_schema_property_enum,
    require_schema_property_example_u64,
};

pub(in crate::docs_api_contract) type OpenApi = Value;

pub(super) fn openapi_document(openapi: &str) -> CheckResult<OpenApi> {
    serde_json::from_str(openapi)
        .map_err(|error| format!("docs/openapi.json must be valid JSON: {error}"))
}

pub(super) fn openapi_path<'a>(openapi: &'a OpenApi, path_name: &str) -> CheckResult<&'a Value> {
    openapi_paths(openapi)?
        .get(path_name)
        .ok_or_else(|| format!("OpenAPI path {path_name} was missing"))
}

pub(in crate::docs_api_contract) fn openapi_schema<'a>(
    openapi: &'a OpenApi,
    schema_name: &str,
) -> CheckResult<&'a Value> {
    openapi_schemas(openapi)?
        .get(schema_name)
        .ok_or_else(|| format!("OpenAPI schema {schema_name} was missing"))
}

fn openapi_operation<'a>(
    openapi: &'a OpenApi,
    path_name: &str,
    method: &str,
) -> CheckResult<&'a Value> {
    openapi_path(openapi, path_name)?
        .get(method)
        .ok_or_else(|| format!("OpenAPI {method} {path_name} operation is missing"))
}

fn openapi_paths(openapi: &OpenApi) -> CheckResult<&Map<String, Value>> {
    openapi
        .get("paths")
        .and_then(Value::as_object)
        .ok_or_else(|| "OpenAPI paths object is missing".to_owned())
}

fn openapi_response<'a>(openapi: &'a OpenApi, response_name: &str) -> CheckResult<&'a Value> {
    openapi_responses(openapi)?
        .get(response_name)
        .ok_or_else(|| format!("OpenAPI response {response_name} was missing"))
}

fn openapi_responses(openapi: &OpenApi) -> CheckResult<&Map<String, Value>> {
    openapi
        .get("components")
        .and_then(|components| components.get("responses"))
        .and_then(Value::as_object)
        .ok_or_else(|| "OpenAPI components.responses object is missing".to_owned())
}

fn openapi_schemas(openapi: &OpenApi) -> CheckResult<&Map<String, Value>> {
    openapi
        .get("components")
        .and_then(|components| components.get("schemas"))
        .and_then(Value::as_object)
        .ok_or_else(|| "OpenAPI components.schemas object is missing".to_owned())
}

fn schema_properties<'a>(schema: &'a Value, label: &str) -> CheckResult<&'a Map<String, Value>> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{label} properties object is missing"))
}
