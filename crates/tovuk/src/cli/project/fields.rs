use serde_json::Value;

pub(crate) fn string_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

pub(crate) fn number_field(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_else(|| {
        value
            .get(key)
            .and_then(Value::as_i64)
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(0)
    })
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

pub(crate) fn nested_string(value: &Value, path: &[&str]) -> String {
    let mut cursor = value;
    for part in path {
        cursor = cursor.get(part).unwrap_or(&Value::Null);
    }
    cursor.as_str().unwrap_or_default().to_owned()
}
