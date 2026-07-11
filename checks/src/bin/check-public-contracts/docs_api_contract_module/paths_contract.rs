use crate::helpers::CheckResult;

use serde_json::Value;

use super::openapi::{OpenApi, openapi_path};

/// Contract value named `REQUIRED_OPENAPI_PATHS`.
const REQUIRED_OPENAPI_PATHS: &[&str] = &[
    "/health",
    "/healthz",
    "/v1/status",
    "/v1/login/device",
    "/v1/login/device/{device_code}",
    "/v1/account",
    "/v1/account/overview",
    "/v1/account/activity",
    "/v1/account/api-keys",
    "/v1/account/api-keys/{key_id}",
    "/v1/data-sources",
    "/v1/pricing",
    "/v1/data-sources/health",
    "/v1/data-sources/{data_source}",
    "/v1/requests",
    "/v1/requests/{request_id}",
    "/v1/requests/{request_id}/cancel",
    "/v1/requests/{request_id}/results",
    "/v1/usage",
    "/v1/billing/checkout",
    "/v1/billing/portal",
    "/v1/support/tickets",
    "/v1/support/tickets/{ticket_id}/resolve",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&require_openapi_paths)];

/// Contract implementation for `require_openapi_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_paths(openapi: &OpenApi) -> CheckResult {
    for path in REQUIRED_OPENAPI_PATHS {
        let _: &Value = check_try!(openapi_path(openapi, path));
    }
    return Ok(());
}
