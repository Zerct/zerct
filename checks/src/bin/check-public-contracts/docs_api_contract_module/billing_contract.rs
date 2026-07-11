use crate::helpers::{CheckResult, require_results};

use super::openapi::{
    OpenApi, openapi_schema, reject_numeric_property_anywhere, reject_schema,
    reject_schema_property, require_example_string, require_json_response_example_string,
    require_named_schema_properties, require_schema_property_example_u64,
};

/// Required billing schema shapes.
const BILLING_SCHEMA_REQUIREMENTS: &[BillingSchemaRequirement] = &[
    BillingSchemaRequirement {
        fields: &["checkout"],
        label: "OpenAPI billing checkout response",
        name: "BillingCheckoutResponse",
    },
    BillingSchemaRequirement {
        fields: &["target_plan", "top_up_usd_cents"],
        label: "OpenAPI billing checkout request",
        name: "BillingCheckoutRequest",
    },
    BillingSchemaRequirement {
        fields: &[
            "topUpMaximumUsdCents",
            "autoTopUpThresholdUsdCents",
            "autoTopUpAmountUsdCents",
            "autoTopUpMonthlyLimitUsdCents",
        ],
        label: "OpenAPI usage estimate",
        name: "UsageCostEstimate",
    },
];

/// Retired monthly-limit example rejected anywhere in the public `OpenAPI` document.
const REJECTED_MONTHLY_LIMIT: NumericField = NumericField {
    field: "autoTopUpMonthlyLimitUsdCents",
    value: 20_000,
};

/// Retired threshold example rejected anywhere in the public `OpenAPI` document.
const REJECTED_THRESHOLD: NumericField = NumericField {
    field: "autoTopUpThresholdUsdCents",
    value: 2_000,
};

/// Canonical auto-top-up examples.
const TOP_UP_EXAMPLES: &[NumericField] = &[
    NumericField {
        field: "autoTopUpThresholdUsdCents",
        value: 500,
    },
    NumericField {
        field: "autoTopUpAmountUsdCents",
        value: 2_000,
    },
    NumericField {
        field: "autoTopUpMonthlyLimitUsdCents",
        value: 100_000,
    },
];

/// Separable billing policy facets applied to the public `OpenAPI` document.
trait BillingPolicy {
    /// Require canonical and reject retired billing examples.
    ///
    /// # Errors
    ///
    /// Returns an error when an example is missing or a retired value remains.
    fn require_billing_examples(&self) -> CheckResult;

    /// Require the public billing schema shapes.
    ///
    /// # Errors
    ///
    /// Returns an error when a required schema or property is missing.
    fn require_billing_schema_shapes(&self) -> CheckResult;

    /// Require billing-specific envelopes and response reasons.
    ///
    /// # Errors
    ///
    /// Returns an error when a billing envelope or example violates the public contract.
    fn require_billing_specifics(&self) -> CheckResult;
}

/// One required billing schema shape.
struct BillingSchemaRequirement {
    /// Required property names.
    fields: &'static [&'static str],
    /// Diagnostic label.
    label: &'static str,
    /// Component schema name.
    name: &'static str,
}

/// One required numeric field example.
struct NumericField {
    /// Component field name.
    field: &'static str,
    /// Required example value.
    value: u64,
}

impl BillingPolicy for OpenApi {
    fn require_billing_examples(&self) -> CheckResult {
        let usage_schema = check_try!(openapi_schema(self, "UsageCostEstimate"));
        let top_up_results = TOP_UP_EXAMPLES.iter().map(|example| {
            return require_schema_property_example_u64(
                usage_schema,
                example.field,
                example.value,
                "OpenAPI usage estimate",
            );
        });
        let rejected_results = [
            reject_numeric_property_anywhere(
                self,
                REJECTED_MONTHLY_LIMIT.field,
                REJECTED_MONTHLY_LIMIT.value,
                "OpenAPI stale auto-top-up example",
            ),
            reject_numeric_property_anywhere(
                self,
                REJECTED_THRESHOLD.field,
                REJECTED_THRESHOLD.value,
                "OpenAPI stale auto-top-up example",
            ),
        ];
        return require_results(top_up_results.chain(rejected_results));
    }

    fn require_billing_schema_shapes(&self) -> CheckResult {
        return require_results(BILLING_SCHEMA_REQUIREMENTS.iter().map(|requirement| {
            return require_named_schema_properties(
                self,
                requirement.name,
                requirement.fields,
                requirement.label,
            );
        }));
    }

    fn require_billing_specifics(&self) -> CheckResult {
        let checkout = check_try!(openapi_schema(self, "BillingCheckout"));
        let checkout_response = check_try!(openapi_schema(self, "BillingCheckoutResponse"));
        return require_results([
            reject_schema(
                self,
                "BillingPortalResponse",
                "retired portal response envelope",
            ),
            reject_schema_property(
                checkout_response,
                "portal",
                "OpenAPI billing portal must use the checkout envelope returned by the API",
            ),
            require_example_string(
                checkout,
                &["reason"],
                "Open Tovuk plus checkout.",
                "OpenAPI checkout response reason",
            ),
            require_json_response_example_string(
                self,
                ("BillingPortalCreated", &["checkout", "reason"]),
                "Manage Tovuk billing.",
                "OpenAPI portal response reason",
            ),
        ]);
    }
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0001] = [size_of_val(&require_openapi_billing_contract)];

/// Contract implementation for `require_openapi_billing_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_billing_contract(openapi: &OpenApi) -> CheckResult {
    check_try!(openapi.require_billing_examples());
    check_try!(openapi.require_billing_schema_shapes());
    return openapi.require_billing_specifics();
}
