use serde_json::Value;

use super::super::errors::{Result, internal_error};

const PRICING_CATALOG_SOURCE: &str = include_str!("pricing_catalog.json");

pub(super) fn pricing_payload() -> Result<Value> {
    serde_json::from_str(PRICING_CATALOG_SOURCE)
        .map_err(|error| internal_error(format!("Tovuk pricing catalog is invalid JSON: {error}")))
}
