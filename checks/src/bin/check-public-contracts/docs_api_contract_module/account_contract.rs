use crate::helpers::{CheckResult, reject_contains, require_contains_all};

use serde_json::Value;

use super::openapi::{
    OpenApi, openapi_schema, reject_operation_field, reject_schema_property,
    reject_schema_property_enum, require_operation_id, require_operation_response_ref,
    require_schema_properties, require_schema_property_enum,
};

/// Required account usage schema shapes.
const ACCOUNT_SCHEMA_REQUIREMENTS: &[SchemaRequirement] = &[
    SchemaRequirement {
        fields: &[
            "profile",
            "usage",
            "pricing",
            "planCatalog",
            "billingEstimate",
            "apiKeys",
        ],
        name: "AccountOverviewResponse",
    },
    SchemaRequirement {
        fields: &["plan", "limits", "pricing"],
        name: "PlanCatalogEntry",
    },
    SchemaRequirement {
        fields: &["apiTokens", "supportTicketsPerDay"],
        name: "PlanPublicLimits",
    },
    SchemaRequirement {
        fields: &["profile", "usage", "pricing", "billingEstimate"],
        name: "AccountUsageResponse",
    },
    SchemaRequirement {
        fields: &["profile", "usage", "pricing", "billingEstimate"],
        name: "AccountActivityResponse",
    },
    SchemaRequirement {
        fields: &[
            "topUpBalanceUsdMicros",
            "currentMonthTopUpBalanceUsedUsdMicros",
            "estimatedMonthlyTotalUsdMicros",
        ],
        name: "UsageCostEstimate",
    },
];

/// Schemas required by the account usage surface.
const ACCOUNT_USAGE_SCHEMAS: &[&str] = &[
    "AccountOverviewResponse",
    "AccountOverviewUsage",
    "AccountUsageMeters",
    "PlanCatalogEntry",
    "PlanPublicLimits",
    "PlanPricing",
    "AccountUsageResponse",
    "AccountActivityResponse",
    "UsageCostEstimate",
];

/// Schemas required by the API-key surface.
const API_KEY_SCHEMAS: &[&str] = &[
    "AccountApiKeysResponse",
    "AccountApiKeyCreateRequest",
    "AccountApiKeyCreateResponse",
    "AccountApiKeyRevokeResponse",
];

/// Fields rejected from canonical usage responses.
const RETIRED_USAGE_FIELDS: &[&str] = &["ok", "balanceUsdMicros", "activity", "nextCursor"];

/// One required account schema shape.
struct SchemaRequirement {
    /// Required property names.
    fields: &'static [&'static str],
    /// Component schema name and diagnostic label.
    name: &'static str,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0004] = [
    size_of_val(&require_openapi_account_profile_contract),
    size_of_val(&require_openapi_account_usage_contract),
    size_of_val(&require_openapi_api_key_contract),
    size_of_val(&require_openapi_api_key_prefix_contract),
];

/// Contract implementation for `require_openapi_account_profile_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_account_profile_contract(openapi: &OpenApi) -> CheckResult {
    let profile_schema = check_try!(openapi_schema(openapi, "AccountProfileResponse"));
    check_try!(require_schema_properties(
        profile_schema,
        &["accountId", "email", "provider", "plan", "displayName"],
        "OpenAPI account profile",
    ));
    check_try!(require_schema_property_enum(
        profile_schema,
        "plan",
        &["unpaid", "plus", "pro", "max"],
        "OpenAPI account plan enum",
    ));
    check_try!(reject_schema_property(
        profile_schema,
        "handle",
        "retired account profile handle field",
    ));
    check_try!(reject_schema_property(
        profile_schema,
        "billingActive",
        "retired account profile billingActive field",
    ));
    check_try!(reject_schema_property_enum(
        profile_schema,
        "plan",
        "free",
        "retired free account plan"
    ));
    return Ok(());
}

/// Contract implementation for `require_openapi_account_usage_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_account_usage_contract(openapi: &OpenApi) -> CheckResult {
    check_try!(require_operation_id(
        openapi,
        ("/v1/account/activity", "get"),
        "getAccountActivity",
        "OpenAPI account activity summary operation",
    ));
    for schema_name in ACCOUNT_USAGE_SCHEMAS {
        let _: &Value = check_try!(openapi_schema(openapi, schema_name));
    }
    for requirement in ACCOUNT_SCHEMA_REQUIREMENTS {
        check_try!(require_schema_properties(
            check_try!(openapi_schema(openapi, requirement.name)),
            requirement.fields,
            requirement.name,
        ));
    }
    let retired_fields = ["AccountUsageResponse", "AccountActivityResponse"]
        .into_iter()
        .flat_map(|schema_name| {
            return RETIRED_USAGE_FIELDS
                .iter()
                .map(move |field| return (schema_name, *field));
        });
    for (schema_name, retired_property) in retired_fields {
        check_try!(reject_schema_property(
            check_try!(openapi_schema(openapi, schema_name)),
            retired_property,
            format!("OpenAPI {schema_name} retired {retired_property} field").as_str(),
        ));
    }
    check_try!(reject_operation_field(
        openapi,
        ("/v1/account/activity", "get"),
        "parameters",
        "OpenAPI account activity route must not document ignored pagination parameters",
    ));
    return Ok(());
}

/// Contract implementation for `require_openapi_api_key_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_api_key_contract(openapi: &OpenApi) -> CheckResult {
    check_try!(require_schema_properties(
        check_try!(openapi_schema(openapi, "AccountApiKeySummary")),
        &[
            "tokenPrefix",
            "currentDayRequestCount",
            "currentMonthRequestCount",
        ],
        "OpenAPI API key summary",
    ));
    for schema_name in API_KEY_SCHEMAS {
        let _: &Value = check_try!(openapi_schema(openapi, schema_name));
    }
    check_try!(require_operation_id(
        openapi,
        ("/v1/account/api-keys", "get"),
        "listAccountApiKeys",
        "OpenAPI API key list operation",
    ));
    check_try!(require_operation_id(
        openapi,
        ("/v1/account/api-keys", "post"),
        "createAccountApiKey",
        "OpenAPI API key create operation",
    ));
    check_try!(require_operation_response_ref(
        openapi,
        ("/v1/account/api-keys", "post"),
        ("402", "#/components/responses/PaymentRequired"),
        "OpenAPI API key create plan-limit response",
    ));
    return require_operation_id(
        openapi,
        ("/v1/account/api-keys/{key_id}", "delete"),
        "revokeAccountApiKey",
        "OpenAPI API key revoke operation",
    );
}

/// Contract implementation for `require_openapi_api_key_prefix_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_api_key_prefix_contract(openapi: &str) -> CheckResult {
    check_try!(require_contains_all(
        openapi,
        &[
            ("tovuk_key_8Qm2", "OpenAPI API key prefix example"),
            (
                "tovuk_key_example_token_shown_once",
                "OpenAPI one-time API key token example",
            ),
            (
                "20 most recently revoked API keys",
                "OpenAPI API key list revoked-history bound",
            ),
            (
                "all active API keys",
                "OpenAPI API key list active-key visibility",
            ),
        ],
    ));
    check_try!(reject_contains(
        openapi,
        "tvk_live",
        "retired tvk_live API key prefix"
    ));
    return reject_contains(openapi, "tovuk_live", "retired tovuk_live API key prefix");
}
