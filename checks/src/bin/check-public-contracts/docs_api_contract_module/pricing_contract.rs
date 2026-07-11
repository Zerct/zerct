use crate::helpers::{CheckResult, require_contains, require_contains_all};

use serde_json::{Value, from_str};

use super::openapi::{
    OpenApi, openapi_schema, require_operation_id, require_schema_properties,
    require_schema_property_enum, require_schema_property_example_u64,
};

/// Required public pricing schema shapes.
const PRICING_SCHEMA_REQUIREMENTS: &[SchemaRequirement] = &[
    SchemaRequirement {
        fields: &["plans", "scraperPrices", "topUp", "nextActions"],
        label: "OpenAPI pricing response",
        name: "PricingResponse",
    },
    SchemaRequirement {
        fields: &[
            "plan",
            "monthlyPriceUsdCents",
            "planBalanceUsdCents",
            "includedBalanceUsdCents",
            "bonusBalanceUsdCents",
            "paidOveragesEnabled",
        ],
        label: "OpenAPI pricing plan",
        name: "PricingPlan",
    },
    SchemaRequirement {
        fields: &["scraper", "priceEvent", "unit", "usdMicrosPerResult"],
        label: "OpenAPI pricing scraper price",
        name: "PricingScraperPrice",
    },
    SchemaRequirement {
        fields: &[
            "minimumUsdCents",
            "maximumUsdCents",
            "expiresAfterInactiveDays",
        ],
        label: "OpenAPI pricing top-up policy",
        name: "PricingTopUpPolicy",
    },
];

/// Canonical public top-up policy examples.
const TOP_UP_POLICY_EXAMPLES: &[NumericField] = &[
    NumericField {
        field: "minimumUsdCents",
        value: 2_000,
    },
    NumericField {
        field: "maximumUsdCents",
        value: 100_000,
    },
    NumericField {
        field: "expiresAfterInactiveDays",
        value: 365,
    },
];

/// One required numeric field example.
struct NumericField {
    /// Component field name.
    field: &'static str,
    /// Required example value.
    value: u64,
}

/// One required public pricing schema shape.
struct SchemaRequirement {
    /// Required property names.
    fields: &'static [&'static str],
    /// Diagnostic label.
    label: &'static str,
    /// Component schema name.
    name: &'static str,
}

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0008] = [
    size_of_val(&format_usd_micros_per_thousand),
    size_of_val(&plan_label),
    size_of_val(&pricing_catalog_example),
    size_of_val(&require_openapi_pricing_contract),
    size_of_val(&require_plan_price_rows),
    size_of_val(&require_pricing_contract),
    size_of_val(&require_scraper_price_rows),
    size_of_val(&scraper_label),
];

/// Contract implementation for `catalog_array`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn catalog_array<'value>(catalog: &'value Value, field: &str) -> CheckResult<&'value [Value]> {
    let Some(values) = catalog.get(field).and_then(Value::as_array) else {
        return Err(format!(
            "public pricing catalog must contain array field {field}"
        ));
    };
    return Ok(values.as_slice());
}

/// Contract implementation for `format_usd_cents`.
fn format_usd_cents(cents: u64) -> String {
    let dollars = cents.checked_div(100).unwrap_or_default();
    let remaining_cents = cents.checked_rem(100).unwrap_or_default();
    if remaining_cents == 0 {
        return format!("${dollars}");
    }
    return format!("${dollars}.{remaining_cents:02}");
}

/// Contract implementation for `format_usd_micros_per_thousand`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn format_usd_micros_per_thousand(usd_micros_per_result: u64) -> CheckResult<String> {
    let usd_micros_per_thousand = check_try!(
        usd_micros_per_result
            .checked_mul(1_000)
            .ok_or_else(|| return "public pricing catalog price is too large".to_owned())
    );
    let remaining_micros = usd_micros_per_thousand
        .checked_rem(10_000)
        .unwrap_or_default();
    if remaining_micros != 0 {
        return Err("public pricing docs require whole-cent prices per 1,000 results".to_owned());
    }
    let cents_per_thousand = usd_micros_per_thousand
        .checked_div(10_000)
        .unwrap_or_default();
    return Ok(format_usd_cents(cents_per_thousand));
}

/// Contract implementation for `plan_label`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn plan_label(plan: &str) -> CheckResult<&'static str> {
    match plan {
        "plus" => return Ok("Plus"),
        "pro" => return Ok("Pro"),
        "max" => return Ok("Max"),
        _unsupported => {
            return Err(format!(
                "public pricing catalog has unsupported plan {plan}"
            ));
        }
    }
}

/// Contract implementation for `pricing_catalog_example`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn pricing_catalog_example(openapi: &str) -> CheckResult<Value> {
    let document: Value = check_try!(
        from_str(openapi).map_err(|error| format!("docs/openapi.json must be valid JSON: {error}"))
    );
    let catalog = check_try!(
        document
            .get("components")
            .and_then(|components| return components.get("schemas"))
            .and_then(|schemas| return schemas.get("PricingResponse"))
            .and_then(|pricing_response| return pricing_response.get("example"))
            .ok_or_else(
                || return "OpenAPI PricingResponse schema must contain an example".to_owned()
            )
    );
    return Ok(catalog.clone());
}

/// Contract implementation for `require_openapi_pricing_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_openapi_pricing_contract(openapi: &OpenApi) -> CheckResult {
    check_try!(require_operation_id(
        openapi,
        ("/v1/pricing", "get"),
        "getPricing",
        "OpenAPI pricing operation",
    ));
    for requirement in PRICING_SCHEMA_REQUIREMENTS {
        check_try!(require_schema_properties(
            check_try!(openapi_schema(openapi, requirement.name)),
            requirement.fields,
            requirement.label,
        ));
    }
    check_try!(require_schema_property_enum(
        check_try!(openapi_schema(openapi, "PricingPlan")),
        "plan",
        &["plus", "pro", "max"],
        "OpenAPI pricing plan enum",
    ));
    let top_up_schema = check_try!(openapi_schema(openapi, "PricingTopUpPolicy"));
    for example in TOP_UP_POLICY_EXAMPLES {
        check_try!(require_schema_property_example_u64(
            top_up_schema,
            example.field,
            example.value,
            "OpenAPI pricing top-up policy",
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_plan_price_rows`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_plan_price_rows(pricing: &str, catalog: &Value) -> CheckResult {
    for plan in check_try!(catalog_array(catalog, "plans")) {
        let plan_name = check_try!(string_field(plan, "plan"));
        let label = check_try!(plan_label(plan_name));
        let monthly_price = format_usd_cents(check_try!(u64_field(plan, "monthlyPriceUsdCents")));
        let included_balance =
            format_usd_cents(check_try!(u64_field(plan, "includedBalanceUsdCents")));
        let expected = format!("| {label} | `{monthly_price}/month` | `{included_balance}`");
        let check_label = format!("pricing {label} balance docs");
        check_try!(require_contains(
            pricing,
            expected.as_str(),
            check_label.as_str()
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_pricing_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_pricing_contract(pricing: &str, openapi: &str) -> CheckResult {
    check_try!(require_contains_all(
        pricing,
        &[
            (
                "There is no free scraper tier",
                "pricing paid-only scraper docs",
            ),
            (
                "deducts from that balance for each successful stored",
                "pricing balance debit docs",
            ),
            (
                "`priceEvents[].usdMicros`",
                "pricing scraper event price docs",
            ),
        ],
    ));
    let catalog = check_try!(pricing_catalog_example(openapi));
    check_try!(require_plan_price_rows(pricing, &catalog));
    return require_scraper_price_rows(pricing, &catalog);
}

/// Contract implementation for `require_scraper_price_rows`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_scraper_price_rows(pricing: &str, catalog: &Value) -> CheckResult {
    for scraper_price in check_try!(catalog_array(catalog, "scraperPrices")) {
        let scraper = check_try!(string_field(scraper_price, "scraper"));
        let label = check_try!(scraper_label(scraper));
        let unit = check_try!(string_field(scraper_price, "unit"));
        let price = check_try!(format_usd_micros_per_thousand(check_try!(u64_field(
            scraper_price,
            "usdMicrosPerResult"
        ))));
        let expected = format!("| {label} Scraper | {unit} | `{price}` |");
        let check_label = format!("pricing {label} per-result docs");
        check_try!(require_contains(
            pricing,
            expected.as_str(),
            check_label.as_str()
        ));
    }
    return Ok(());
}

/// Contract implementation for `scraper_label`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn scraper_label(scraper: &str) -> CheckResult<&'static str> {
    match scraper {
        "reddit" => return Ok("Reddit"),
        "github" => return Ok("GitHub"),
        "google-maps" => return Ok("Google Maps"),
        "amazon" => return Ok("Amazon"),
        "alibaba" => return Ok("Alibaba"),
        "temu" => return Ok("Temu"),
        "etsy" => return Ok("Etsy"),
        "apple-app-store" => return Ok("Apple App Store"),
        "google-play-store" => return Ok("Google Play Store"),
        "trendyol" => return Ok("Trendyol"),
        "hepsiburada" => return Ok("Hepsiburada"),
        "youtube" => return Ok("YouTube"),
        "zillow" => return Ok("Zillow"),
        "indeed" => return Ok("Indeed"),
        "trustpilot" => return Ok("Trustpilot"),
        "linkedin" => return Ok("LinkedIn"),
        "tiktok" => return Ok("TikTok"),
        "instagram" => return Ok("Instagram"),
        "x" => return Ok("X"),
        _unsupported => {
            return Err(format!(
                "public pricing catalog has unsupported scraper {scraper}"
            ));
        }
    }
}

/// Contract implementation for `string_field`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn string_field<'value>(value: &'value Value, field: &str) -> CheckResult<&'value str> {
    return value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("public pricing catalog must contain string field {field}"));
}

/// Contract implementation for `u64_field`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn u64_field(value: &Value, field: &str) -> CheckResult<u64> {
    return value.get(field).and_then(Value::as_u64).ok_or_else(|| {
        return format!("public pricing catalog must contain positive integer field {field}");
    });
}
