use crate::helpers::CheckResult;

use super::openapi::{OpenApi, openapi_path};

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
    "/v1/scrapers",
    "/v1/pricing",
    "/v1/scrapers/health",
    "/v1/scrapers/{scraper}",
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

pub(super) fn require_openapi_paths(openapi: &OpenApi) -> CheckResult {
    for path in REQUIRED_OPENAPI_PATHS {
        openapi_path(openapi, path)?;
    }
    Ok(())
}
