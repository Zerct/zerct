use super::super::pricing_catalog::pricing_payload;

#[test]
fn pricing_payload_keeps_public_plan_contract() -> Result<(), Box<dyn std::error::Error>> {
    let payload = pricing_payload();

    if payload["plans"][0]["plan"] != "plus" {
        return Err(format!("unexpected plans: {}", payload["plans"]).into());
    }
    if payload["plans"][1]["includedBalanceUsdCents"] != 12000 {
        return Err(format!("unexpected Pro plan: {}", payload["plans"][1]).into());
    }
    if payload["plans"][2]["bonusBalanceUsdCents"] != 10000 {
        return Err(format!("unexpected Max plan: {}", payload["plans"][2]).into());
    }
    Ok(())
}

#[test]
fn pricing_payload_keeps_public_scraper_prices() -> Result<(), Box<dyn std::error::Error>> {
    let payload = pricing_payload();

    let prices = payload["scraperPrices"]
        .as_array()
        .ok_or("scraperPrices must be an array")?;
    let tiktok = prices
        .iter()
        .find(|price| price["scraper"] == "tiktok")
        .ok_or("missing TikTok price")?;
    if tiktok["usdMicrosPerResult"] != 1700 {
        return Err(format!("unexpected TikTok price: {tiktok}").into());
    }
    if payload["nextActions"].as_array().is_none_or(Vec::is_empty) {
        return Err("nextActions must be non-empty".into());
    }
    Ok(())
}
