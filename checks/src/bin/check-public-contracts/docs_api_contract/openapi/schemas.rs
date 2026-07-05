use serde_json::Value;

use crate::helpers::CheckResult;

use super::{OpenApi, openapi_schemas, schema_properties};

pub(in crate::docs_api_contract) fn reject_schema(
    openapi: &OpenApi,
    schema_name: &str,
    label: &str,
) -> CheckResult {
    if openapi_schemas(openapi)?.contains_key(schema_name) {
        Err(format!("OpenAPI {label} is present"))
    } else {
        Ok(())
    }
}

pub(in crate::docs_api_contract) fn reject_schema_property(
    schema: &Value,
    field: &str,
    label: &str,
) -> CheckResult {
    if schema_properties(schema, label)?.contains_key(field) {
        Err(format!("{label} is present"))
    } else {
        Ok(())
    }
}

pub(in crate::docs_api_contract) fn reject_schema_property_enum(
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

pub(in crate::docs_api_contract) fn reject_numeric_property_anywhere(
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
