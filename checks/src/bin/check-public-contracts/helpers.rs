pub(super) use crate::{
    helpers_io::{
        OutputChannel, env_int, file_exists, find_repo_root, must_abs, read_json,
        read_package_json, read_sorted_texts_recursive, read_text, read_text_corpus, write_line,
    },
    helpers_public_copy::{reject_forbidden_public_copy_terms, retired_public_names},
};

use alloc::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0002] = [
    size_of_val(&extract_cargo_lock_package_version),
    size_of_val(&require_string_map_keys_exactly),
];

/// Contract representation for `CheckResult`.
pub(super) type CheckResult<T = ()> = Result<T, String>;

/// A required or rejected snippet paired with its diagnostic label.
pub(super) type LabeledSnippet = (&'static str, &'static str);

/// Contract implementation for `extract_cargo_lock_package_version`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn extract_cargo_lock_package_version(
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
    return Err(format!(
        "could not read Cargo.lock package {package_name} version"
    ));
}

/// Contract implementation for `extract_line_quoted_value`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn extract_line_quoted_value(
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
    return Err(format!("could not read {label}"));
}

/// Contract implementation for `list_or_none`.
fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    return values.join(", ");
}

/// Contract implementation for `map_keys`.
pub(super) fn map_keys(values: &BTreeMap<String, String>) -> Vec<String> {
    return values.keys().cloned().collect();
}

/// Contract implementation for `number_field`.
pub(super) fn number_field(values: &Value, name: &str) -> f64 {
    return values.get(name).and_then(Value::as_f64).unwrap_or_default();
}

/// Contract implementation for `quoted_value`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn quoted_value(source: &str, label: &str) -> CheckResult<String> {
    let Some(rest) = source.strip_prefix('"') else {
        return Err(format!("could not read {label}"));
    };
    let Some(end) = rest.find('"') else {
        return Err(format!("could not read {label}"));
    };
    return Ok(rest.chars().take(end).collect());
}

/// Contract implementation for `reject_contains`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_contains(source: &str, snippet: &str, label: &str) -> CheckResult {
    if source.contains(snippet) {
        return Err(format!("{label} is present"));
    }
    return Ok(());
}

/// Contract implementation for `reject_contains_any`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_contains_any(source: &str, rejections: &[LabeledSnippet]) -> CheckResult {
    for &(snippet, label) in rejections {
        check_try!(reject_contains(source, snippet, label));
    }
    return Ok(());
}

/// Contract implementation for `require_contains`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_contains(source: &str, snippet: &str, label: &str) -> CheckResult {
    if source.contains(snippet) {
        return Ok(());
    }
    return Err(format!("{label} is missing"));
}

/// Contract implementation for `require_contains_all`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_contains_all(source: &str, requirements: &[LabeledSnippet]) -> CheckResult {
    for &(snippet, label) in requirements {
        check_try!(require_contains(source, snippet, label));
    }
    return Ok(());
}

/// Contract implementation for `require_equal`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_equal(actual: &str, expected: &str, label: &str) -> CheckResult {
    if actual == expected {
        return Ok(());
    }
    return Err(format!("{label} must be {expected:?}, got {actual:?}"));
}

/// Require every lazily produced contract result to succeed.
///
/// # Errors
///
/// Returns the first contract error produced by the iterator.
pub(super) fn require_results(results: impl IntoIterator<Item = CheckResult>) -> CheckResult {
    for result in results {
        check_try!(result);
    }
    return Ok(());
}

/// Contract implementation for `require_snippets`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_snippets(source: &str, label: &str, snippets: &[&str]) -> CheckResult {
    for snippet in snippets {
        check_try!(require_contains(
            source,
            snippet,
            format!("{label} missing {snippet}").as_str(),
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_string_map_keys_exactly`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_string_map_keys_exactly(
    actual: &BTreeMap<String, String>,
    expected: &[String],
    label: &str,
) -> CheckResult {
    return require_string_slice_exactly(&map_keys(actual), expected, label);
}

/// Contract implementation for `require_string_slice_exactly`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_string_slice_exactly(
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
    return Err(format!(
        "{label} must have exactly {}; unexpected: {}; missing: {}",
        expected_set.iter().cloned().collect::<Vec<_>>().join(", "),
        list_or_none(&unexpected),
        list_or_none(&missing)
    ));
}
