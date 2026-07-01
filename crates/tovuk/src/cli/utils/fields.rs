use serde_json::Value;

pub(crate) fn optional_string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(crate) fn optional_string_alias(value: &Value, aliases: &[&str]) -> Option<String> {
    aliases
        .iter()
        .find_map(|alias| optional_string_field(value, alias))
}

pub(crate) fn number_alias(value: &Value, aliases: &[&str]) -> Option<u64> {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_u64))
}
