use serde_json::Value;

use crate::helpers::CheckResult;

use super::{OpenApi, openapi_operation};

pub(in crate::docs_api_contract) fn reject_operation_field(
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

pub(in crate::docs_api_contract) fn require_parameter_bounds(
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
