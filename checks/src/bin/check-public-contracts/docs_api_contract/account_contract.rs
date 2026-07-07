use crate::helpers::{CheckResult, reject_contains, require_contains_all};

use super::openapi::{
    OpenApi, openapi_schema, reject_operation_field, reject_schema_property,
    reject_schema_property_enum, require_operation_id, require_operation_response_ref,
    require_schema_properties, require_schema_property_enum,
};

pub(super) fn require_openapi_account_profile_contract(openapi: &OpenApi) -> CheckResult {
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

pub(super) fn require_openapi_account_usage_contract(openapi: &OpenApi) -> CheckResult {
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

pub(super) fn require_openapi_api_key_contract(openapi: &OpenApi) -> CheckResult {
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
    require_operation_response_ref(
        openapi,
        "/v1/account/api-keys",
        "post",
        "402",
        "#/components/responses/PaymentRequired",
        "OpenAPI API key create plan-limit response",
    )?;
    require_operation_id(
        openapi,
        "/v1/account/api-keys/{key_id}",
        "delete",
        "revokeAccountApiKey",
        "OpenAPI API key revoke operation",
    )
}

pub(super) fn require_openapi_api_key_prefix_contract(openapi: &str) -> CheckResult {
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
