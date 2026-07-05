use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, require_contains_all},
};

mod account_contract;
mod billing_contract;
mod login_contract;
mod openapi;
mod paths_contract;
mod pricing_contract;
mod scraper_contract;
mod status_contract;

use account_contract::{
    require_openapi_account_profile_contract, require_openapi_account_usage_contract,
    require_openapi_api_key_contract, require_openapi_api_key_prefix_contract,
};
use billing_contract::require_openapi_billing_contract;
use login_contract::require_openapi_login_contract;
use openapi::openapi_document;
use paths_contract::require_openapi_paths;
use pricing_contract::require_pricing_contract;
use scraper_contract::require_openapi_scraper_response_contract;
use status_contract::require_openapi_status_checks;

pub(crate) fn require_support_pricing_and_openapi(sources: &DocsSources) -> CheckResult {
    require_contains_all(
        sources.status.as_str(),
        &[("tovuk scraper health --json", "status scraper health docs")],
    )?;
    require_contains_all(
        sources.support.as_str(),
        &[
            ("tovuk support create", "support create docs"),
            ("POST /v1/support/tickets", "support API create docs"),
            ("account API key", "support API key docs"),
            ("request_id", "support API request id docs"),
        ],
    )?;
    require_pricing_contract(sources.pricing.as_str(), sources.openapi.as_str())?;

    let openapi = openapi_document(sources.openapi.as_str())?;
    require_openapi_paths(&openapi)?;
    require_openapi_login_contract(&openapi)?;
    require_openapi_account_profile_contract(&openapi)?;
    require_openapi_account_usage_contract(&openapi)?;
    require_openapi_api_key_contract(&openapi)?;
    require_openapi_api_key_prefix_contract(sources.openapi.as_str())?;
    require_openapi_billing_contract(&openapi)?;
    pricing_contract::require_openapi_pricing_contract(&openapi)?;
    require_openapi_scraper_response_contract(&openapi)?;
    require_openapi_status_checks(&openapi)?;

    require_contains_all(
        sources.openapi.as_str(),
        &[
            (
                "users and AI/API agents can open service tickets",
                "OpenAPI support agent create description",
            ),
            (
                r#""created_by""#,
                "OpenAPI support creator attribution field",
            ),
            (r#""request_id""#, "OpenAPI support request id context"),
            (
                r#""linkedinPostSearch""#,
                "OpenAPI LinkedIn post search example",
            ),
            (
                r#""author_company_urns""#,
                "OpenAPI LinkedIn author company filter",
            ),
            (
                r#""linkedinCompanyEmployees""#,
                "OpenAPI LinkedIn company employees example",
            ),
        ],
    )
}
