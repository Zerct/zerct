use serde_json::Value;

pub(crate) fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn string_alias(value: &Value, aliases: &[&str]) -> String {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_str))
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn number_alias(value: &Value, aliases: &[&str]) -> Option<u64> {
    aliases
        .iter()
        .find_map(|alias| value.get(alias).and_then(Value::as_u64))
}
