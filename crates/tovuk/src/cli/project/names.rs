use std::path::Path;

pub(crate) fn service_name_from_dir(project_dir: &Path) -> String {
    service_name_from_value(
        project_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default(),
    )
    .unwrap_or_else(|| "api".to_owned())
}

pub(super) fn service_name_from_value(value: &str) -> Option<String> {
    let mut result = String::new();
    let mut last_dash = false;
    for character in value.to_ascii_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character);
            last_dash = false;
        } else if !last_dash {
            result.push('-');
            last_dash = true;
        }
        if result.len() >= 48 {
            break;
        }
    }
    let result = result.trim_matches('-').to_owned();
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

pub(crate) fn is_dns_safe_name(value: &str) -> bool {
    if value.is_empty() || value.len() > 48 {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
    {
        return false;
    }
    value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
