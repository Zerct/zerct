use serde_json::Value;

/// Extracts a trimmed non-empty string field from a JSON object.
pub(in crate::cli) fn optional_string_field(value: &Value, key: &str) -> Option<String> {
    return value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|field_value| return !field_value.is_empty())
        .map(str::to_owned);
}
