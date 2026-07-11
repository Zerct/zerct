use crate::helpers::CheckResult;

use serde_json::Value;

use super::{OpenApi, openapi_response};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&example_string_at_path),
    size_of_val(&example_value_at_path),
    size_of_val(&json_response_example_check),
    size_of_val(&require_example_string),
    size_of_val(&schema_example_value),
];

/// Response component name and nested example field path.
pub(in crate::docs_api_contract) type ResponseExamplePath = (&'static str, &'static [&'static str]);

/// Contract implementation for `example_string_at_path`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn example_string_at_path<'value>(
    value: &'value Value,
    path: &[&str],
    label: &str,
) -> CheckResult<&'value str> {
    return check_try!(example_value_at_path(value, path, label))
        .as_str()
        .ok_or_else(|| format!("{label} example value is not a string"));
}

/// Contract implementation for `example_value_at_path`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn example_value_at_path<'value>(
    value: &'value Value,
    path: &[&str],
    label: &str,
) -> CheckResult<&'value Value> {
    let mut current = value;
    for field in path {
        current = check_try!(
            current
                .get(*field)
                .ok_or_else(|| format!("{label} example field {field:?} is missing"))
        );
    }
    return Ok(current);
}

/// Contract implementation for `json_response_example_check`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn json_response_example_check<'value>(
    openapi: &'value OpenApi,
    response_name: &str,
    name: &str,
    label: &str,
) -> CheckResult<&'value Value> {
    let checks = check_try!(
        check_try!(json_response_example_value(openapi, response_name, label))
            .get("checks")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{label} JSON example checks are missing"))
    );
    return checks
        .iter()
        .find(|check| return check.get("name").and_then(Value::as_str) == Some(name))
        .ok_or_else(|| format!("{label} check {name:?} is missing"));
}

/// Contract implementation for `json_response_example_value`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn json_response_example_value<'value>(
    openapi: &'value OpenApi,
    response_name: &str,
    label: &str,
) -> CheckResult<&'value Value> {
    return check_try!(openapi_response(openapi, response_name))
        .get("content")
        .and_then(|content| return content.get("application/json"))
        .and_then(|json| return json.get("examples"))
        .and_then(|examples| return examples.get("example"))
        .and_then(|example| return example.get("value"))
        .ok_or_else(|| format!("{label} JSON example is missing"));
}

/// Contract implementation for `require_example_string`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_example_string(
    schema: &Value,
    path: &[&str],
    expected: &str,
    label: &str,
) -> CheckResult {
    let actual = check_try!(example_string_at_path(
        check_try!(schema_example_value(schema, label)),
        path,
        label
    ));
    if actual == expected {
        return Ok(());
    }
    return Err(format!("{label} must be {expected:?}, got {actual:?}"));
}

/// Contract implementation for `require_json_response_example_check_name`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_json_response_example_check_name(
    openapi: &OpenApi,
    response_name: &str,
    name: &str,
    label: &str,
) -> CheckResult {
    let _: &Value = check_try!(json_response_example_check(
        openapi,
        response_name,
        name,
        label
    ));
    return Ok(());
}

/// Contract implementation for `require_json_response_example_check_u64`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_json_response_example_check_u64(
    openapi: &OpenApi,
    field_path: (&str, &str, &str),
    label: &str,
) -> CheckResult {
    let (response_name, check_name, field) = field_path;
    let checks = check_try!(
        check_try!(json_response_example_value(openapi, response_name, label))
            .get("checks")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{label} JSON example checks are missing"))
    );
    let check = check_try!(
        checks
            .iter()
            .find(|value| return value.get("name").and_then(Value::as_str) == Some(check_name))
            .ok_or_else(|| format!("{label} check {check_name:?} is missing"))
    );
    if check.get(field).and_then(Value::as_u64).is_some() {
        return Ok(());
    }
    return Err(format!(
        "{label} check {check_name:?} field {field:?} is missing"
    ));
}

/// Contract implementation for `require_json_response_example_string`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_json_response_example_string(
    openapi: &OpenApi,
    field_path: ResponseExamplePath,
    expected: &str,
    label: &str,
) -> CheckResult {
    let (response_name, path) = field_path;
    let actual = check_try!(example_string_at_path(
        check_try!(json_response_example_value(openapi, response_name, label)),
        path,
        label,
    ));
    if actual == expected {
        return Ok(());
    }
    return Err(format!("{label} must be {expected:?}, got {actual:?}"));
}

/// Contract implementation for `schema_example_value`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn schema_example_value<'value>(
    schema: &'value Value,
    label: &str,
) -> CheckResult<&'value Value> {
    return schema
        .get("example")
        .ok_or_else(|| format!("{label} example is missing"));
}
