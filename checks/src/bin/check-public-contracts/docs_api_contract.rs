use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, reject_contains, require_contains_all},
};

mod login_contract;
mod openapi;
mod pricing_contract;

use login_contract::require_openapi_login_contract;
use openapi::{
    OpenApi, openapi_document, openapi_path, openapi_schema,
    reject_json_response_example_check_name, reject_numeric_property_anywhere,
    reject_operation_field, reject_schema, reject_schema_property, reject_schema_property_enum,
    require_example_string, require_json_response_example_check_name,
    require_json_response_example_string, require_operation_id, require_parameter_bounds,
    require_schema_properties, require_schema_property_enum, require_schema_property_example_u64,
};
use pricing_contract::require_pricing_contract;

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

fn require_openapi_account_profile_contract(openapi: &OpenApi) -> CheckResult {
    let profile_schema = openapi_schema(openapi, "AccountProfileResponse")?;
    require_schema_properties(
        profile_schema,
        &["accountId", "email", "provider", "plan", "displayName"],
        "OpenAPI account profile",
    )?;
    require_schema_property_enum(
        profile_schema,
        "plan",
        &["unpaid", "plus", "pro", "max"],
        "OpenAPI account plan enum",
    )?;
    reject_schema_property(
        profile_schema,
        "handle",
        "retired account profile handle field",
    )?;
    reject_schema_property(
        profile_schema,
        "billingActive",
        "retired account profile billingActive field",
    )?;
    reject_schema_property_enum(profile_schema, "plan", "free", "retired free account plan")?;
    Ok(())
}

fn require_openapi_account_usage_contract(openapi: &OpenApi) -> CheckResult {
    require_operation_id(
        openapi,
        "/v1/account/activity",
        "get",
        "getAccountActivity",
        "OpenAPI account activity summary operation",
    )?;
    for schema_name in [
        "AccountOverviewResponse",
        "AccountOverviewUsage",
        "AccountUsageMeters",
        "PlanCatalogEntry",
        "PlanPublicLimits",
        "PlanPricing",
        "AccountUsageResponse",
        "AccountActivityResponse",
        "UsageCostEstimate",
    ] {
        openapi_schema(openapi, schema_name)?;
    }
    require_schema_properties(
        openapi_schema(openapi, "AccountOverviewResponse")?,
        &[
            "profile",
            "usage",
            "pricing",
            "planCatalog",
            "billingEstimate",
            "apiKeys",
        ],
        "AccountOverviewResponse",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "PlanCatalogEntry")?,
        &["plan", "limits", "pricing"],
        "PlanCatalogEntry",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "PlanPublicLimits")?,
        &["apiTokens", "supportTicketsPerDay"],
        "PlanPublicLimits",
    )?;
    for schema_name in ["AccountUsageResponse", "AccountActivityResponse"] {
        let schema = openapi_schema(openapi, schema_name)?;
        require_schema_properties(
            schema,
            &["profile", "usage", "pricing", "billingEstimate"],
            schema_name,
        )?;
        for retired_property in ["ok", "balanceUsdMicros", "activity", "nextCursor"] {
            reject_schema_property(
                schema,
                retired_property,
                format!("OpenAPI {schema_name} retired {retired_property} field").as_str(),
            )?;
        }
    }
    require_schema_properties(
        openapi_schema(openapi, "UsageCostEstimate")?,
        &[
            "topUpBalanceUsdMicros",
            "currentMonthTopUpBalanceUsedUsdMicros",
            "estimatedMonthlyTotalUsdMicros",
        ],
        "OpenAPI usage estimate",
    )?;
    reject_operation_field(
        openapi,
        "/v1/account/activity",
        "get",
        "parameters",
        "OpenAPI account activity route must not document ignored pagination parameters",
    )?;
    Ok(())
}

fn require_openapi_paths(openapi: &OpenApi) -> CheckResult {
    for path in [
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
    ] {
        openapi_path(openapi, path)?;
    }
    Ok(())
}

fn require_openapi_api_key_contract(openapi: &OpenApi) -> CheckResult {
    require_schema_properties(
        openapi_schema(openapi, "AccountApiKeySummary")?,
        &[
            "tokenPrefix",
            "currentDayRequestCount",
            "currentMonthRequestCount",
        ],
        "OpenAPI API key summary",
    )?;
    for schema_name in [
        "AccountApiKeysResponse",
        "AccountApiKeyCreateRequest",
        "AccountApiKeyCreateResponse",
        "AccountApiKeyRevokeResponse",
    ] {
        openapi_schema(openapi, schema_name)?;
    }
    require_operation_id(
        openapi,
        "/v1/account/api-keys",
        "get",
        "listAccountApiKeys",
        "OpenAPI API key list operation",
    )?;
    require_operation_id(
        openapi,
        "/v1/account/api-keys",
        "post",
        "createAccountApiKey",
        "OpenAPI API key create operation",
    )?;
    require_operation_id(
        openapi,
        "/v1/account/api-keys/{key_id}",
        "delete",
        "revokeAccountApiKey",
        "OpenAPI API key revoke operation",
    )
}

fn require_openapi_api_key_prefix_contract(openapi: &str) -> CheckResult {
    require_contains_all(
        openapi,
        &[
            ("tovuk_key_8Qm2", "OpenAPI API key prefix example"),
            (
                "tovuk_key_example_token_shown_once",
                "OpenAPI one-time API key token example",
            ),
        ],
    )?;
    reject_contains(openapi, "tvk_live", "retired tvk_live API key prefix")?;
    reject_contains(openapi, "tovuk_live", "retired tovuk_live API key prefix")
}

fn require_openapi_billing_contract(openapi: &OpenApi) -> CheckResult {
    require_schema_properties(
        openapi_schema(openapi, "BillingCheckoutResponse")?,
        &["checkout"],
        "OpenAPI billing checkout response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "BillingCheckoutRequest")?,
        &["target_plan", "top_up_usd_cents"],
        "OpenAPI billing checkout request",
    )?;
    let usage_estimate_schema = openapi_schema(openapi, "UsageCostEstimate")?;
    require_schema_properties(
        usage_estimate_schema,
        &[
            "topUpMaximumUsdCents",
            "autoTopUpThresholdUsdCents",
            "autoTopUpAmountUsdCents",
            "autoTopUpMonthlyLimitUsdCents",
        ],
        "OpenAPI usage estimate",
    )?;
    require_schema_property_example_u64(
        usage_estimate_schema,
        "autoTopUpThresholdUsdCents",
        500,
        "OpenAPI usage estimate",
    )?;
    require_schema_property_example_u64(
        usage_estimate_schema,
        "autoTopUpAmountUsdCents",
        2_000,
        "OpenAPI usage estimate",
    )?;
    require_schema_property_example_u64(
        usage_estimate_schema,
        "autoTopUpMonthlyLimitUsdCents",
        100_000,
        "OpenAPI usage estimate",
    )?;
    reject_numeric_property_anywhere(
        openapi,
        "autoTopUpThresholdUsdCents",
        2_000,
        "OpenAPI stale auto top-up threshold",
    )?;
    reject_numeric_property_anywhere(
        openapi,
        "autoTopUpMonthlyLimitUsdCents",
        20_000,
        "OpenAPI stale auto top-up monthly limit",
    )?;
    reject_schema(
        openapi,
        "BillingPortalResponse",
        "retired portal response envelope",
    )?;
    reject_schema_property(
        openapi_schema(openapi, "BillingCheckoutResponse")?,
        "portal",
        "OpenAPI billing portal must use the checkout envelope returned by the API",
    )?;
    require_example_string(
        openapi_schema(openapi, "BillingCheckout")?,
        &["reason"],
        "Open Tovuk plus checkout.",
        "OpenAPI checkout response reason",
    )?;
    require_json_response_example_string(
        openapi,
        "BillingPortalCreated",
        &["checkout", "reason"],
        "Manage Tovuk billing.",
        "OpenAPI portal response reason",
    )
}

fn require_openapi_scraper_response_contract(openapi: &OpenApi) -> CheckResult {
    require_schema_properties(
        openapi_schema(openapi, "ScrapersResponse")?,
        &["scrapers", "nextActions"],
        "OpenAPI scraper catalog response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScraperSummary")?,
        &["inputSchema"],
        "OpenAPI scraper summary",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScraperRuntimeHealthResponse")?,
        &["scrapers", "nextActions"],
        "OpenAPI scraper runtime health response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScraperDetailsResponse")?,
        &["scraper", "runtimeHealth", "nextActions"],
        "OpenAPI scraper details response",
    )?;
    require_operation_id(
        openapi,
        "/v1/requests",
        "get",
        "listScrapeRequests",
        "OpenAPI request list operation",
    )?;
    require_parameter_bounds(
        openapi,
        "/v1/requests",
        "get",
        "limit",
        50,
        200,
        "OpenAPI request list page limit",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeRequest")?,
        &[
            "resultCount",
            "estimatedCostUsdMicros",
            "costUsdMicros",
            "resultsUrl",
            "agentInstruction",
        ],
        "OpenAPI scraper request",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeRequestResponse")?,
        &["request", "nextActions"],
        "OpenAPI scraper request response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeCancelResponse")?,
        &["requestId", "canceled"],
        "OpenAPI scraper cancel response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeResultsResponse")?,
        &["request", "records", "nextCursor", "nextActions"],
        "OpenAPI scraper results response",
    )?;
    require_schema_properties(
        openapi_schema(openapi, "ScrapeRecord")?,
        &["index", "sizeBytes"],
        "OpenAPI scrape record",
    )?;
    for schema_name in [
        "ScrapersResponse",
        "ScraperRuntimeHealthResponse",
        "ScraperDetailsResponse",
        "ScrapeRequestResponse",
        "ScrapeRequestsResponse",
        "ScrapeCancelResponse",
        "ScrapeResultsResponse",
    ] {
        reject_schema_property(
            openapi_schema(openapi, schema_name)?,
            "ok",
            format!("OpenAPI {schema_name} retired ok wrapper").as_str(),
        )?;
    }
    Ok(())
}

fn require_openapi_status_checks(openapi: &OpenApi) -> CheckResult {
    openapi_schema(openapi, "StatusResponse")?;
    require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "control_plane_sqlite",
        "OpenAPI status control-plane SQLite check",
    )?;
    require_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "redis",
        "OpenAPI status Redis check",
    )?;
    reject_json_response_example_check_name(
        openapi,
        "StatusLoaded",
        "database",
        "OpenAPI status must not expose generic database product wording",
    )
}
