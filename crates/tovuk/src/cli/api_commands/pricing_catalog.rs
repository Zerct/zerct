use serde::Serialize;
use serde_json::{Value, json};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanCatalogEntry {
    plan: &'static str,
    monthly_price_usd_cents: u32,
    included_balance_usd_cents: u32,
    bonus_balance_usd_cents: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScraperPrice {
    scraper: &'static str,
    unit: &'static str,
    usd_micros_per_result: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TopUpPolicy {
    minimum_usd_cents: u32,
    expires_after_inactive_days: u16,
}

const PLAN_CATALOG: [PlanCatalogEntry; 3] = [
    PlanCatalogEntry {
        plan: "plus",
        monthly_price_usd_cents: 2_000,
        included_balance_usd_cents: 2_000,
        bonus_balance_usd_cents: 0,
    },
    PlanCatalogEntry {
        plan: "pro",
        monthly_price_usd_cents: 10_000,
        included_balance_usd_cents: 12_000,
        bonus_balance_usd_cents: 2_000,
    },
    PlanCatalogEntry {
        plan: "max",
        monthly_price_usd_cents: 20_000,
        included_balance_usd_cents: 30_000,
        bonus_balance_usd_cents: 10_000,
    },
];

const SCRAPER_PRICES: [ScraperPrice; 6] = [
    ScraperPrice {
        scraper: "reddit",
        unit: "record",
        usd_micros_per_result: 700,
    },
    ScraperPrice {
        scraper: "github",
        unit: "record",
        usd_micros_per_result: 600,
    },
    ScraperPrice {
        scraper: "linkedin",
        unit: "record",
        usd_micros_per_result: 900,
    },
    ScraperPrice {
        scraper: "tiktok",
        unit: "record",
        usd_micros_per_result: 1_700,
    },
    ScraperPrice {
        scraper: "instagram",
        unit: "record",
        usd_micros_per_result: 800,
    },
    ScraperPrice {
        scraper: "x",
        unit: "post",
        usd_micros_per_result: 300,
    },
];

const TOP_UP_POLICY: TopUpPolicy = TopUpPolicy {
    minimum_usd_cents: 2_000,
    expires_after_inactive_days: 365,
};

const NEXT_ACTIONS: [&str; 3] = [
    "Use `tovuk scraper list --json` and `tovuk scraper show <scraper> --json` to choose a public-data scraper.",
    "Use `priceEvents[].usdMicros`, request limits, and `tovuk usage --json` to estimate account balance impact before high-count requests.",
    "Choose a plan, then use `tovuk billing checkout plus --json`, `tovuk billing checkout pro --json`, or `tovuk billing checkout max --json` when an upgrade is required.",
];

pub(super) fn pricing_payload() -> Value {
    json!({
        "plans": PLAN_CATALOG,
        "scraperPrices": SCRAPER_PRICES,
        "topUp": TOP_UP_POLICY,
        "nextActions": NEXT_ACTIONS,
    })
}
