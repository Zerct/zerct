use crate::helpers::CheckResult;

use serde_json::Value;

use super::{OpenApi, openapi_schema, openapi_schemas, schema_properties};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&reject_schema)];

/// Contract implementation for `collect_numeric_property_matches`.
fn collect_numeric_property_matches(
    value: &Value,
    criterion: (&str, u64),
    path: &str,
    matches: &mut Vec<String>,
) {
    if let Some(object) = value.as_object() {
        for (key, child) in object {
            let child_path = format!("{path}.{key}");
            matches.extend(
                (key == criterion.0 && child.as_u64() == Some(criterion.1))
                    .then(|| return child_path.clone()),
            );
            collect_numeric_property_matches(child, criterion, &child_path, matches);
        }
        return;
    }
    if let Some(items) = value.as_array() {
        for (index, child) in items.iter().enumerate() {
            collect_numeric_property_matches(
                child,
                criterion,
                format!("{path}[{index}]").as_str(),
                matches,
            );
        }
    }
}

/// Contract implementation for `reject_numeric_property_anywhere`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn reject_numeric_property_anywhere(
    openapi: &OpenApi,
    field: &str,
    rejected_value: u64,
    label: &str,
) -> CheckResult {
    let mut matches = Vec::new();
    collect_numeric_property_matches(openapi, (field, rejected_value), "$", &mut matches);
    if matches.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "{label} found {field}={rejected_value} at {}",
        matches.join(", ")
    ));
}

/// Contract implementation for `reject_schema`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn reject_schema(
    openapi: &OpenApi,
    schema_name: &str,
    label: &str,
) -> CheckResult {
    if check_try!(openapi_schemas(openapi)).contains_key(schema_name) {
        return Err(format!("OpenAPI {label} is present"));
    }
    return Ok(());
}

/// Contract implementation for `reject_schema_property`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn reject_schema_property(
    schema: &Value,
    field: &str,
    label: &str,
) -> CheckResult {
    if check_try!(schema_properties(schema, label)).contains_key(field) {
        return Err(format!("{label} is present"));
    }
    return Ok(());
}

/// Contract implementation for `reject_schema_property_enum`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn reject_schema_property_enum(
    schema: &Value,
    field: &str,
    rejected_value: &str,
    label: &str,
) -> CheckResult {
    let Some(enum_values) = check_try!(schema_properties(schema, label))
        .get(field)
        .and_then(|property| return property.get("enum"))
        .and_then(Value::as_array)
    else {
        return Ok(());
    };
    if enum_values
        .iter()
        .any(|value| return value.as_str() == Some(rejected_value))
    {
        return Err(format!("{label} is present"));
    }
    return Ok(());
}

/// Require named component schema properties.
///
/// # Errors
///
/// Returns an error when the schema or a required property is missing.
pub(in crate::docs_api_contract) fn require_named_schema_properties(
    openapi: &OpenApi,
    schema_name: &str,
    fields: &[&str],
    label: &str,
) -> CheckResult {
    return require_schema_properties(
        check_try!(openapi_schema(openapi, schema_name)),
        fields,
        label,
    );
}

/// Contract implementation for `require_schema_properties`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_schema_properties(
    schema: &Value,
    fields: &[&str],
    label: &str,
) -> CheckResult {
    let properties = check_try!(schema_properties(schema, label));
    for field in fields {
        if !properties.contains_key(*field) {
            return Err(format!("{label} field {field:?} is missing"));
        }
    }
    return Ok(());
}

/// Contract implementation for `require_schema_property_enum`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_schema_property_enum(
    schema: &Value,
    field: &str,
    expected_values: &[&str],
    label: &str,
) -> CheckResult {
    let enum_values = check_try!(
        check_try!(schema_properties(schema, label))
            .get(field)
            .and_then(|property| return property.get("enum"))
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{label} enum is missing"))
    );
    for expected in expected_values {
        if !enum_values
            .iter()
            .any(|value| return value.as_str() == Some(*expected))
        {
            return Err(format!("{label} value {expected:?} is missing"));
        }
    }
    return Ok(());
}

/// Contract implementation for `require_schema_property_example_u64`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_schema_property_example_u64(
    schema: &Value,
    field: &str,
    expected: u64,
    label: &str,
) -> CheckResult {
    let actual = check_try!(
        check_try!(schema_properties(schema, label))
            .get(field)
            .and_then(|property| return property.get("example"))
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{label} field {field:?} numeric example is missing"))
    );
    if actual == expected {
        return Ok(());
    }
    return Err(format!(
        "{label} field {field:?} example must be {expected}, got {actual}"
    ));
}
