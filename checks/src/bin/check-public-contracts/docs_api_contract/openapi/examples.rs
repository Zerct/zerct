use serde_json::Value;

use crate::helpers::CheckResult;

use super::{OpenApi, openapi_response};

pub(in crate::docs_api_contract) fn require_example_string(
    schema: &Value,
    path: &[&str],
    expected: &str,
    label: &str,
) -> CheckResult {
    let actual = example_string_at_path(schema_example_value(schema, label)?, path, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} must be {expected:?}, got {actual:?}"))
    }
}

pub(in crate::docs_api_contract) fn require_json_response_example_check_name(
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

pub(in crate::docs_api_contract) fn reject_json_response_example_check_name(
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

pub(in crate::docs_api_contract) fn require_json_response_example_check_u64(
    openapi: &OpenApi,
    response_name: &str,
    check_name: &str,
    field: &str,
    label: &str,
) -> CheckResult {
    let checks = json_response_example_value(openapi, response_name, label)?
        .get("checks")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} JSON example checks are missing"))?;
    let check = checks
        .iter()
        .find(|value| value.get("name").and_then(Value::as_str) == Some(check_name))
        .ok_or_else(|| format!("{label} check {check_name:?} is missing"))?;
    if check.get(field).and_then(Value::as_u64).is_some() {
        Ok(())
    } else {
        Err(format!(
            "{label} check {check_name:?} field {field:?} is missing"
        ))
    }
}

pub(in crate::docs_api_contract) fn require_json_response_example_string(
    openapi: &OpenApi,
    response_name: &str,
    path: &[&str],
    expected: &str,
    label: &str,
) -> CheckResult {
    let actual = example_string_at_path(
        json_response_example_value(openapi, response_name, label)?,
        path,
        label,
    )?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} must be {expected:?}, got {actual:?}"))
    }
}

fn json_response_example_has_check_name(
    openapi: &OpenApi,
    response_name: &str,
    name: &str,
    label: &str,
) -> CheckResult<bool> {
    let checks = json_response_example_value(openapi, response_name, label)?
        .get("checks")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} JSON example checks are missing"))?;
    Ok(checks
        .iter()
        .any(|check| check.get("name").and_then(Value::as_str) == Some(name)))
}

fn schema_example_value<'a>(schema: &'a Value, label: &str) -> CheckResult<&'a Value> {
    schema
        .get("example")
        .ok_or_else(|| format!("{label} example is missing"))
}

fn json_response_example_value<'a>(
    openapi: &'a OpenApi,
    response_name: &str,
    label: &str,
) -> CheckResult<&'a Value> {
    openapi_response(openapi, response_name)?
        .get("content")
        .and_then(|content| content.get("application/json"))
        .and_then(|json| json.get("examples"))
        .and_then(|examples| examples.get("example"))
        .and_then(|example| example.get("value"))
        .ok_or_else(|| format!("{label} JSON example is missing"))
}

fn example_string_at_path<'a>(
    value: &'a Value,
    path: &[&str],
    label: &str,
) -> CheckResult<&'a str> {
    example_value_at_path(value, path, label)?
        .as_str()
        .ok_or_else(|| format!("{label} example value is not a string"))
}

fn example_value_at_path<'a>(
    value: &'a Value,
    path: &[&str],
    label: &str,
) -> CheckResult<&'a Value> {
    let mut current = value;
    for field in path {
        current = current
            .get(*field)
            .ok_or_else(|| format!("{label} example field {field:?} is missing"))?;
    }
    Ok(current)
}
