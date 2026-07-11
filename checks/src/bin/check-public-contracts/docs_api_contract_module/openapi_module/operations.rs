use crate::helpers::CheckResult;

use serde_json::Value;

use super::{OpenApi, openapi_operation};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0003] = [
    size_of_val(&reject_operation_field),
    size_of_val(&require_operation_response_ref),
    size_of_val(&require_parameter_bounds),
];

/// Contract implementation for `reject_operation_field`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn reject_operation_field(
    openapi: &OpenApi,
    operation: (&str, &str),
    field: &str,
    label: &str,
) -> CheckResult {
    if check_try!(openapi_operation(openapi, operation.0, operation.1))
        .get(field)
        .is_some()
    {
        return Err(format!("{label} is present"));
    }
    return Ok(());
}

/// Contract implementation for `require_operation_id`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_operation_id(
    openapi: &OpenApi,
    operation: (&str, &str),
    operation_id: &str,
    label: &str,
) -> CheckResult {
    let actual = check_try!(
        check_try!(openapi_operation(openapi, operation.0, operation.1))
            .get("operationId")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{label} operationId is missing"))
    );
    if actual == operation_id {
        return Ok(());
    }
    return Err(format!("{label} must be {operation_id:?}, got {actual:?}"));
}

/// Contract implementation for `require_operation_response_ref`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_operation_response_ref(
    openapi: &OpenApi,
    operation: (&str, &str),
    expected_response: (&str, &str),
    label: &str,
) -> CheckResult {
    let (status, expected_ref) = expected_response;
    let actual = check_try!(
        check_try!(openapi_operation(openapi, operation.0, operation.1))
            .get("responses")
            .and_then(|responses| return responses.get(status))
            .and_then(|response_value| return response_value.get("$ref"))
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{label} response {status} ref is missing"))
    );
    if actual == expected_ref {
        return Ok(());
    }
    return Err(format!(
        "{label} response {status} ref must be {expected_ref:?}, got {actual:?}"
    ));
}

/// Contract implementation for `require_parameter_bounds`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(in crate::docs_api_contract) fn require_parameter_bounds(
    openapi: &OpenApi,
    parameter_path: (&str, &str, &str),
    expected_bounds: (u64, u64),
    label: &str,
) -> CheckResult {
    let (path_name, method, parameter_name) = parameter_path;
    let parameters = check_try!(
        check_try!(openapi_operation(openapi, path_name, method))
            .get("parameters")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{label} parameters are missing"))
    );
    let Some(matched_parameter) = parameters.iter().find(|candidate| {
        return candidate.get("name").and_then(Value::as_str) == Some(parameter_name);
    }) else {
        return Err(format!("{label} parameter {parameter_name:?} is missing"));
    };
    let schema = check_try!(
        matched_parameter
            .get("schema")
            .ok_or_else(|| format!("{label} schema is missing"))
    );
    let default = check_try!(
        schema
            .get("default")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{label} default is missing"))
    );
    let maximum = check_try!(
        schema
            .get("maximum")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{label} maximum is missing"))
    );
    for (actual, expected, field) in [
        (default, expected_bounds.0, "default"),
        (maximum, expected_bounds.1, "maximum"),
    ] {
        if actual != expected {
            return Err(format!("{label} {field} must be {expected}, got {actual}"));
        }
    }
    return Ok(());
}
