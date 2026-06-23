use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::types::PackageJson;

pub(crate) type CheckResult<T = ()> = Result<T, String>;

#[derive(Clone, Copy, Debug)]
struct PublicCopyForbiddenTerm {
    value_bytes: &'static [u8],
    whole_word: bool,
}

pub(crate) fn read_sorted_texts_recursive(
    directory: &str,
    suffix: &str,
) -> CheckResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_paths_with_suffix(Path::new(directory), suffix, &mut paths)?;
    paths.sort();

    paths
        .iter()
        .map(|path| read_text(path.as_path()))
        .collect::<CheckResult<Vec<_>>>()
}

fn collect_paths_with_suffix(root: &Path, suffix: &str, paths: &mut Vec<PathBuf>) -> CheckResult {
    let entries = fs::read_dir(root)
        .map_err(|error| format!("read directory {}: {error}", root.display()))?;
    for entry_result in entries {
        let entry = entry_result
            .map_err(|error| format!("read entry under {}: {error}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read file type for {}: {error}", path.display()))?;
        if file_type.is_dir() {
            collect_paths_with_suffix(path.as_path(), suffix, paths)?;
            continue;
        }
        if entry.file_name().to_string_lossy().ends_with(suffix) {
            paths.push(path);
        }
    }
    Ok(())
}

pub(crate) fn read_text(path: impl AsRef<Path>) -> CheckResult<String> {
    let path = path.as_ref();
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

pub(crate) fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> CheckResult<T> {
    let path = path.as_ref();
    let source = read_text(path)?;
    serde_json::from_str(source.as_str())
        .map_err(|error| format!("parse {}: {error}", path.display()))
}

pub(crate) fn read_package_json(path: impl AsRef<Path>) -> CheckResult<PackageJson> {
    read_json(path)
}

pub(crate) fn reject_forbidden_public_copy_terms(label: &str, source: &str) -> CheckResult {
    let lower = source.to_lowercase();
    for term in public_copy_forbidden_terms() {
        let value = ascii_term(term.value_bytes);
        if term.whole_word {
            if contains_ascii_word(lower.as_str(), &[value.as_str()]) {
                return Err(format!(
                    "{label} contains forbidden public positioning term: {value}"
                ));
            }
            continue;
        }
        if lower.contains(value.as_str()) {
            return Err(format!(
                "{label} contains forbidden public positioning term: {value}"
            ));
        }
    }
    Ok(())
}

fn public_copy_forbidden_terms() -> Vec<PublicCopyForbiddenTerm> {
    vec![
        PublicCopyForbiddenTerm {
            value_bytes: &[99, 108, 111, 117, 100, 102, 108, 97, 114, 101],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[118, 101, 114, 99, 101, 108],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[115, 117, 112, 97, 98, 97, 115, 101],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[104, 101, 116, 122, 110, 101, 114],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[115, 101, 114, 118, 101, 114, 108, 101, 115, 115],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[101, 100, 103, 101],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[99, 100, 110],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[
                100, 117, 114, 97, 98, 108, 101, 32, 111, 98, 106, 101, 99, 116,
            ],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[119, 111, 114, 107, 101, 114, 115, 32, 107, 118],
            whole_word: false,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[100, 49],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[114, 50],
            whole_word: true,
        },
        PublicCopyForbiddenTerm {
            value_bytes: &[
                112, 97, 103, 101, 115, 32, 102, 117, 110, 99, 116, 105, 111, 110, 115,
            ],
            whole_word: false,
        },
    ]
}

pub(crate) fn retired_public_names() -> Vec<String> {
    vec![
        ascii_term(&[122, 101, 114, 99, 116]),
        ascii_term(&[120, 113, 117, 105, 107]),
    ]
}

pub(crate) fn ascii_term(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

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

pub(crate) fn env_int(name: &str, fallback: i64) -> CheckResult<i64> {
    let raw = std::env::var(name).unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(fallback);
    }
    trimmed
        .parse::<i64>()
        .map_err(|_| format!("{name} must be an integer"))
}

pub(crate) fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}

pub(crate) fn must_abs(path: impl AsRef<Path>) -> CheckResult<String> {
    let path = path.as_ref();
    path.canonicalize()
        .map(|absolute| absolute.display().to_string())
        .map_err(|error| format!("resolve {}: {error}", path.display()))
}

pub(crate) fn find_repo_root() -> String {
    match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        _ => must_abs(".").unwrap_or_else(|_| ".".to_owned()),
    }
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

pub(crate) fn contains_ascii_word(value: &str, forbidden_words: &[&str]) -> bool {
    value
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| {
            forbidden_words
                .iter()
                .any(|forbidden| word.eq_ignore_ascii_case(forbidden))
        })
}

pub(crate) fn has_markdown_link(source: &str) -> bool {
    let Some((_, after_bracket)) = source.split_once('[') else {
        return false;
    };
    after_bracket.contains("](") && after_bracket.contains(')')
}
