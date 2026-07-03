use serde_json::{Map, Value};

use crate::helpers::CheckResult;

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

pub(super) fn reject_operation_field(
    openapi: &OpenApi,
    path_name: &str,
    method: &str,
    field: &str,
    label: &str,
) -> CheckResult {
    if openapi_operation(openapi, path_name, method)?
        .get(field)
        .is_some()
    {
        Err(format!("{label} is present"))
    } else {
        Ok(())
    }
}

pub(super) fn reject_schema(openapi: &OpenApi, schema_name: &str, label: &str) -> CheckResult {
    if openapi_schemas(openapi)?.contains_key(schema_name) {
        Err(format!("OpenAPI {label} is present"))
    } else {
        Ok(())
    }
}

pub(super) fn reject_schema_property(schema: &Value, field: &str, label: &str) -> CheckResult {
    if schema_properties(schema, label)?.contains_key(field) {
        Err(format!("{label} is present"))
    } else {
        Ok(())
    }
}

pub(super) fn reject_schema_property_enum(
    schema: &Value,
    field: &str,
    rejected_value: &str,
    label: &str,
) -> CheckResult {
    let Some(enum_values) = schema_properties(schema, label)?
        .get(field)
        .and_then(|property| property.get("enum"))
        .and_then(Value::as_array)
    else {
        return Ok(());
    };
    if enum_values
        .iter()
        .any(|value| value.as_str() == Some(rejected_value))
    {
        Err(format!("{label} is present"))
    } else {
        Ok(())
    }
}

pub(super) fn reject_numeric_property_anywhere(
    openapi: &OpenApi,
    field: &str,
    rejected_value: u64,
    label: &str,
) -> CheckResult {
    let mut matches = Vec::new();
    collect_numeric_property_matches(openapi, field, rejected_value, "$", &mut matches);
    if matches.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{label} found {field}={rejected_value} at {}",
            matches.join(", ")
        ))
    }
}

pub(in crate::docs_api_contract) fn require_schema_property_example_u64(
    schema: &Value,
    field: &str,
    expected: u64,
    label: &str,
) -> CheckResult {
    let actual = schema_properties(schema, label)?
        .get(field)
        .and_then(|property| property.get("example"))
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label} field {field:?} numeric example is missing"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{label} field {field:?} example must be {expected}, got {actual}"
        ))
    }
}

pub(super) fn require_example_string(
    schema: &Value,
    path: &[&str],
    expected: &str,
    label: &str,
) -> CheckResult {
    let mut current = schema
        .get("example")
        .ok_or_else(|| format!("{label} example is missing"))?;
    for field in path {
        current = current
            .get(*field)
            .ok_or_else(|| format!("{label} example field {field:?} is missing"))?;
    }
    let actual = current
        .as_str()
        .ok_or_else(|| format!("{label} example value is not a string"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} must be {expected:?}, got {actual:?}"))
    }
}

pub(super) fn require_json_response_example_check_name(
    openapi: &OpenApi,
    response_name: &str,
    name: &str,
    label: &str,
) -> CheckResult {
    if json_response_example_has_check_name(openapi, response_name, name, label)? {
        Ok(())
    } else {
        Err(format!("{label} is missing"))
    }
}

pub(super) fn reject_json_response_example_check_name(
    openapi: &OpenApi,
    response_name: &str,
    name: &str,
    label: &str,
) -> CheckResult {
    if json_response_example_has_check_name(openapi, response_name, name, label)? {
        Err(format!("{label} is present"))
    } else {
        Ok(())
    }
}

pub(super) fn require_json_response_example_string(
    openapi: &OpenApi,
    response_name: &str,
    path: &[&str],
    expected: &str,
    label: &str,
) -> CheckResult {
    let response = openapi_response(openapi, response_name)?;
    let mut current = response
        .get("content")
        .and_then(|content| content.get("application/json"))
        .and_then(|json| json.get("examples"))
        .and_then(|examples| examples.get("example"))
        .and_then(|example| example.get("value"))
        .ok_or_else(|| format!("{label} JSON example is missing"))?;
    for field in path {
        current = current
            .get(*field)
            .ok_or_else(|| format!("{label} example field {field:?} is missing"))?;
    }
    let actual = current
        .as_str()
        .ok_or_else(|| format!("{label} example value is not a string"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} must be {expected:?}, got {actual:?}"))
    }
}

pub(in crate::docs_api_contract) fn require_operation_id(
    openapi: &OpenApi,
    path_name: &str,
    method: &str,
    operation_id: &str,
    label: &str,
) -> CheckResult {
    let actual = openapi_operation(openapi, path_name, method)?
        .get("operationId")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label} operationId is missing"))?;
    if actual == operation_id {
        Ok(())
    } else {
        Err(format!("{label} must be {operation_id:?}, got {actual:?}"))
    }
}

pub(super) fn require_parameter_bounds(
    openapi: &OpenApi,
    path_name: &str,
    method: &str,
    parameter_name: &str,
    expected_default: u64,
    expected_maximum: u64,
    label: &str,
) -> CheckResult {
    let parameters = openapi_operation(openapi, path_name, method)?
        .get("parameters")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} parameters are missing"))?;
    let Some(parameter) = parameters
        .iter()
        .find(|parameter| parameter.get("name").and_then(Value::as_str) == Some(parameter_name))
    else {
        return Err(format!("{label} parameter {parameter_name:?} is missing"));
    };
    let schema = parameter
        .get("schema")
        .ok_or_else(|| format!("{label} schema is missing"))?;
    let default = schema
        .get("default")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label} default is missing"))?;
    let maximum = schema
        .get("maximum")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{label} maximum is missing"))?;
    if default != expected_default {
        return Err(format!(
            "{label} default must be {expected_default}, got {default}"
        ));
    }
    if maximum != expected_maximum {
        return Err(format!(
            "{label} maximum must be {expected_maximum}, got {maximum}"
        ));
    }
    Ok(())
}

pub(in crate::docs_api_contract) fn require_schema_properties(
    schema: &Value,
    fields: &[&str],
    label: &str,
) -> CheckResult {
    let properties = schema_properties(schema, label)?;
    for field in fields {
        if !properties.contains_key(*field) {
            return Err(format!("{label} field {field:?} is missing"));
        }
    }
    Ok(())
}

pub(in crate::docs_api_contract) fn require_schema_property_enum(
    schema: &Value,
    field: &str,
    expected_values: &[&str],
    label: &str,
) -> CheckResult {
    let enum_values = schema_properties(schema, label)?
        .get(field)
        .and_then(|property| property.get("enum"))
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} enum is missing"))?;
    for expected in expected_values {
        if !enum_values
            .iter()
            .any(|value| value.as_str() == Some(*expected))
        {
            return Err(format!("{label} value {expected:?} is missing"));
        }
    }
    Ok(())
}

fn json_response_example_has_check_name(
    openapi: &OpenApi,
    response_name: &str,
    name: &str,
    label: &str,
) -> CheckResult<bool> {
    let checks = openapi_response(openapi, response_name)?
        .get("content")
        .and_then(|content| content.get("application/json"))
        .and_then(|json| json.get("examples"))
        .and_then(|examples| examples.get("example"))
        .and_then(|example| example.get("value"))
        .and_then(|value| value.get("checks"))
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} JSON example checks are missing"))?;
    Ok(checks
        .iter()
        .any(|check| check.get("name").and_then(Value::as_str) == Some(name)))
}

fn collect_numeric_property_matches(
    value: &Value,
    field: &str,
    rejected_value: u64,
    path: &str,
    matches: &mut Vec<String>,
) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if key == field && child.as_u64() == Some(rejected_value) {
                    matches.push(child_path.clone());
                }
                collect_numeric_property_matches(
                    child,
                    field,
                    rejected_value,
                    &child_path,
                    matches,
                );
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_numeric_property_matches(
                    child,
                    field,
                    rejected_value,
                    format!("{path}[{index}]").as_str(),
                    matches,
                );
            }
        }
        _other => {}
    }
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
