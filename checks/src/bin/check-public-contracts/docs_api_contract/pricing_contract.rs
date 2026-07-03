use serde_json::Value;

use crate::helpers::{CheckResult, read_json, require_contains, require_contains_all};

const PUBLIC_PRICING_CATALOG_PATH: &str = "crates/tovuk/src/cli/api_commands/pricing_catalog.json";

pub(crate) fn require_pricing_contract(pricing: &str) -> CheckResult {
    require_contains_all(
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
    )?;
    let catalog = read_json(PUBLIC_PRICING_CATALOG_PATH)?;
    require_plan_price_rows(pricing, &catalog)?;
    require_scraper_price_rows(pricing, &catalog)
}

fn require_plan_price_rows(pricing: &str, catalog: &Value) -> CheckResult {
    for plan in catalog_array(catalog, "plans")? {
        let plan_name = string_field(plan, "plan")?;
        let label = plan_label(plan_name)?;
        let monthly_price = format_usd_cents(u64_field(plan, "monthlyPriceUsdCents")?);
        let included_balance = format_usd_cents(u64_field(plan, "includedBalanceUsdCents")?);
        let expected = format!("| {label} | `{monthly_price}/month` | `{included_balance}`");
        let check_label = format!("pricing {label} balance docs");
        require_contains(pricing, expected.as_str(), check_label.as_str())?;
    }
    Ok(())
}

fn require_scraper_price_rows(pricing: &str, catalog: &Value) -> CheckResult {
    for scraper_price in catalog_array(catalog, "scraperPrices")? {
        let scraper = string_field(scraper_price, "scraper")?;
        let label = scraper_label(scraper)?;
        let unit = string_field(scraper_price, "unit")?;
        let price =
            format_usd_micros_per_thousand(u64_field(scraper_price, "usdMicrosPerResult")?)?;
        let expected = format!("| {label} Scraper | {unit} | `{price}` |");
        let check_label = format!("pricing {label} per-result docs");
        require_contains(pricing, expected.as_str(), check_label.as_str())?;
    }
    Ok(())
}

fn catalog_array<'a>(catalog: &'a Value, field: &str) -> CheckResult<&'a [Value]> {
    let Some(values) = catalog.get(field).and_then(Value::as_array) else {
        return Err(format!(
            "public pricing catalog must contain array field {field}"
        ));
    };
    Ok(values.as_slice())
}

fn string_field<'a>(value: &'a Value, field: &str) -> CheckResult<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("public pricing catalog must contain string field {field}"))
}

fn u64_field(value: &Value, field: &str) -> CheckResult<u64> {
    value.get(field).and_then(Value::as_u64).ok_or_else(|| {
        format!("public pricing catalog must contain positive integer field {field}")
    })
}

fn plan_label(plan: &str) -> CheckResult<&'static str> {
    match plan {
        "plus" => Ok("Plus"),
        "pro" => Ok("Pro"),
        "max" => Ok("Max"),
        _unsupported => Err(format!(
            "public pricing catalog has unsupported plan {plan}"
        )),
    }
}

fn scraper_label(scraper: &str) -> CheckResult<&'static str> {
    match scraper {
        "reddit" => Ok("Reddit"),
        "github" => Ok("GitHub"),
        "linkedin" => Ok("LinkedIn"),
        "tiktok" => Ok("TikTok"),
        "instagram" => Ok("Instagram"),
        "x" => Ok("X"),
        _unsupported => Err(format!(
            "public pricing catalog has unsupported scraper {scraper}"
        )),
    }
}

fn format_usd_cents(cents: u64) -> String {
    let dollars = cents / 100;
    let remaining_cents = cents % 100;
    if remaining_cents == 0 {
        format!("${dollars}")
    } else {
        format!("${dollars}.{remaining_cents:02}")
    }
}

fn format_usd_micros_per_thousand(usd_micros_per_result: u64) -> CheckResult<String> {
    let usd_micros_per_thousand = usd_micros_per_result
        .checked_mul(1_000)
        .ok_or_else(|| "public pricing catalog price is too large".to_owned())?;
    if usd_micros_per_thousand % 10_000 != 0 {
        return Err("public pricing docs require whole-cent prices per 1,000 results".to_owned());
    }
    Ok(format_usd_cents(usd_micros_per_thousand / 10_000))
}
