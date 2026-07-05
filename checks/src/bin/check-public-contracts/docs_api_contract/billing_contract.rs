use crate::helpers::CheckResult;

use super::openapi::{
    OpenApi, openapi_schema, reject_numeric_property_anywhere, reject_schema,
    reject_schema_property, require_example_string, require_json_response_example_string,
    require_schema_properties, require_schema_property_example_u64,
};

pub(super) fn require_openapi_billing_contract(openapi: &OpenApi) -> CheckResult {
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
