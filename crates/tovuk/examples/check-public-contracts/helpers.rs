use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

pub(crate) use crate::{
    helpers_io::{
        env_int, file_exists, find_repo_root, must_abs, read_json, read_package_json,
        read_sorted_texts_recursive, read_text,
    },
    helpers_public_copy::{ascii_term, reject_forbidden_public_copy_terms, retired_public_names},
};

pub(crate) type CheckResult<T = ()> = Result<T, String>;

pub(crate) fn require_contains(source: &str, snippet: &str, label: &str) -> CheckResult {
    if source.contains(snippet) {
        Ok(())
    } else {
        Err(format!("{label} is missing"))
    }
}

pub(crate) fn reject_contains(source: &str, snippet: &str, label: &str) -> CheckResult {
    if source.contains(snippet) {
        Err(format!("{label} is present"))
    } else {
        Ok(())
    }
}

pub(crate) fn require_equal(actual: &str, expected: &str, label: &str) -> CheckResult {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{label} must be {expected:?}, got {actual:?}"))
    }
}

pub(crate) fn require_string_slice_exactly(
    actual: &[String],
    expected: &[String],
    label: &str,
) -> CheckResult {
    let actual_set = actual.iter().cloned().collect::<BTreeSet<_>>();
    let expected_set = expected.iter().cloned().collect::<BTreeSet<_>>();
    if actual_set == expected_set {
        return Ok(());
    }

    let unexpected = actual_set
        .difference(&expected_set)
        .cloned()
        .collect::<Vec<_>>();
    let missing = expected_set
        .difference(&actual_set)
        .cloned()
        .collect::<Vec<_>>();
    Err(format!(
        "{label} must have exactly {}; unexpected: {}; missing: {}",
        expected_set.iter().cloned().collect::<Vec<_>>().join(", "),
        list_or_none(&unexpected),
        list_or_none(&missing)
    ))
}

pub(crate) fn require_string_map_keys_exactly(
    actual: &BTreeMap<String, String>,
    expected: &[String],
    label: &str,
) -> CheckResult {
    require_string_slice_exactly(&map_keys(actual), expected, label)
}

pub(crate) fn map_keys(values: &BTreeMap<String, String>) -> Vec<String> {
    values.keys().cloned().collect()
}

fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_owned()
    } else {
        values.join(", ")
    }
}

pub(crate) fn number_field(values: &Value, name: &str) -> f64 {
    values.get(name).and_then(Value::as_f64).unwrap_or_default()
}

pub(crate) fn extract_line_quoted_value(
    source: &str,
    prefix: &str,
    label: &str,
) -> CheckResult<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return quoted_value(rest.trim(), label);
        }
    }
    Err(format!("could not read {label}"))
}

pub(crate) fn extract_rust_const_str(source: &str, name: &str, label: &str) -> CheckResult<String> {
    let prefix = format!("const {name}: &str = ");
    for line in source.lines() {
        let trimmed = line.trim();
        let Some((_, value)) = trimmed.split_once(prefix.as_str()) else {
            continue;
        };
        return quoted_value(value.trim(), label);
    }
    Err(format!("could not read {label}"))
}

pub(crate) fn extract_cargo_lock_package_version(
    source: &str,
    package_name: &str,
) -> CheckResult<String> {
    let name_line = format!("name = {package_name:?}");
    let mut in_package = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == "[[package]]" {
            in_package = false;
            continue;
        }
        if trimmed == name_line {
            in_package = true;
            continue;
        }
        if in_package && trimmed.starts_with("version = ") {
            return quoted_value(
                trimmed.trim_start_matches("version = ").trim(),
                "Cargo.lock version",
            );
        }
    }
    Err(format!(
        "could not read Cargo.lock package {package_name} version"
    ))
}

fn quoted_value(source: &str, label: &str) -> CheckResult<String> {
    let Some(rest) = source.strip_prefix('"') else {
        return Err(format!("could not read {label}"));
    };
    let Some(end) = rest.find('"') else {
        return Err(format!("could not read {label}"));
    };
    Ok(rest.chars().take(end).collect())
}

pub(crate) fn has_markdown_link(source: &str) -> bool {
    let Some((_, after_bracket)) = source.split_once('[') else {
        return false;
    };
    after_bracket.contains("](") && after_bracket.contains(')')
}
