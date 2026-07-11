/// Public contract checks for account contract.
#[path = "docs_api_contract_module/account_contract.rs"]
pub mod account_contract;

/// Public contract checks for billing contract.
#[path = "docs_api_contract_module/billing_contract.rs"]
pub mod billing_contract;

/// Public contract checks for login contract.
#[path = "docs_api_contract_module/login_contract.rs"]
pub mod login_contract;

/// Public contract checks for openapi.
#[path = "docs_api_contract_module/openapi_module.rs"]
pub mod openapi;

/// Public contract checks for paths contract.
#[path = "docs_api_contract_module/paths_contract.rs"]
pub mod paths_contract;

/// Public contract checks for pricing contract.
#[path = "docs_api_contract_module/pricing_contract.rs"]
pub mod pricing_contract;

/// Public contract checks for scraper contract.
#[path = "docs_api_contract_module/scraper_contract.rs"]
pub mod scraper_contract;

/// Public contract checks for status contract.
#[path = "docs_api_contract_module/status_contract.rs"]
pub mod status_contract;

use account_contract::{
    require_openapi_account_profile_contract, require_openapi_account_usage_contract,
    require_openapi_api_key_contract, require_openapi_api_key_prefix_contract,
};

use billing_contract::require_openapi_billing_contract;

use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, LabeledSnippet, require_contains_all},
};

use login_contract::require_openapi_login_contract;

use openapi::openapi_document;

use paths_contract::require_openapi_paths;

use pricing_contract::require_pricing_contract;

use scraper_contract::require_openapi_scraper_response_contract;

use status_contract::require_openapi_status_checks;

/// Public support snippets required in `OpenAPI`.
const OPENAPI_SUPPORT_SNIPPETS: &[LabeledSnippet] = &[
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
];

/// Public support snippets required in the support guide.
const SUPPORT_DOC_SNIPPETS: &[LabeledSnippet] = &[
    ("tovuk support create", "support create docs"),
    ("POST /v1/support/tickets", "support API create docs"),
    ("account API key", "support API key docs"),
    ("request_id", "support API request id docs"),
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&require_support_pricing_and_openapi)];

/// Contract implementation for `require_support_pricing_and_openapi`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_support_pricing_and_openapi(sources: &DocsSources) -> CheckResult {
    check_try!(require_contains_all(
        sources.status.as_str(),
        &[("tovuk scraper health --json", "status scraper health docs")],
    ));
    check_try!(require_contains_all(
        sources.support.as_str(),
        SUPPORT_DOC_SNIPPETS,
    ));
    check_try!(require_pricing_contract(
        sources.pricing.as_str(),
        sources.openapi.as_str()
    ));

    let openapi = check_try!(openapi_document(sources.openapi.as_str()));
    check_try!(require_openapi_paths(&openapi));
    check_try!(require_openapi_login_contract(&openapi));
    check_try!(require_openapi_account_profile_contract(&openapi));
    check_try!(require_openapi_account_usage_contract(&openapi));
    check_try!(require_openapi_api_key_contract(&openapi));
    check_try!(require_openapi_api_key_prefix_contract(
        sources.openapi.as_str()
    ));
    check_try!(require_openapi_billing_contract(&openapi));
    check_try!(pricing_contract::require_openapi_pricing_contract(&openapi));
    check_try!(require_openapi_scraper_response_contract(&openapi));
    check_try!(require_openapi_status_checks(&openapi));

    return require_contains_all(sources.openapi.as_str(), OPENAPI_SUPPORT_SNIPPETS);
}
