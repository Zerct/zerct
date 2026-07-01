use serde_json::json;

use super::pricing_payload;

#[test]
fn pricing_payload_keeps_capability_arrays() -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({
        "plans": [{"plan": "plus"}],
        "products": [{"key": "scrapers"}],
        "ignored": true
    });

    let payload = pricing_payload(&response, false)?;

    if payload["plans"][0]["plan"] != "plus" {
        return Err(format!("unexpected plans: {}", payload["plans"]).into());
    }
    if payload["products"][0]["key"] != "scrapers" {
        return Err(format!("unexpected products: {}", payload["products"]).into());
    }
    if payload["nextActions"].as_array().is_none_or(Vec::is_empty) {
        return Err("nextActions must be non-empty".into());
    }
    Ok(())
}

#[test]
fn pricing_payload_rejects_missing_plans() -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({
        "products": []
    });
    let error = match pricing_payload(&response, false) {
        Ok(payload) => return Err(format!("unexpected payload: {payload}").into()),
        Err(error) => error,
    };

    let payload = error.payload();
    if payload.code != "capabilities_invalid" {
        return Err(format!("unexpected code: {}", payload.code).into());
    }
    if !payload.message.contains("`plans`") {
        return Err(format!("unexpected message: {}", payload.message).into());
    }
    Ok(())
}

#[test]
fn pricing_payload_rejects_non_array_products() -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({
        "plans": [],
        "products": {}
    });
    let error = match pricing_payload(&response, false) {
        Ok(payload) => return Err(format!("unexpected payload: {payload}").into()),
        Err(error) => error,
    };

    let payload = error.payload();
    if !payload.message.contains("`products`") {
        return Err(format!("unexpected message: {}", payload.message).into());
    }
    Ok(())
}
