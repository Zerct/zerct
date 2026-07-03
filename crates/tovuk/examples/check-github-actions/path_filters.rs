//! Workflow path-filter parsing and tracked-file matching.

use std::collections::BTreeSet;

pub(super) fn workflow_path_filters(contents: &str) -> Vec<String> {
    let mut filters = Vec::new();
    let mut block_indent = None;
    for line in contents.lines() {
        let indent = leading_spaces(line);
        let trimmed = line.trim();
        if matches!(trimmed, "paths:" | "paths-ignore:") {
            block_indent = Some(indent);
            continue;
        }
        let Some(parent_indent) = block_indent else {
            continue;
        };
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if indent <= parent_indent {
            block_indent = None;
            continue;
        }
        if let Some(value) = trimmed.strip_prefix("- ") {
            filters.push(unquote_yaml_string(value.trim()).to_owned());
        }
    }
    filters
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn unquote_yaml_string(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value)
}

pub(super) fn path_filter_matches_tracked(filter: &str, tracked_files: &BTreeSet<String>) -> bool {
    if !filter.contains('*') {
        return tracked_files.contains(filter);
    }
    tracked_files.iter().any(|path| glob_matches(filter, path))
}

fn glob_matches(pattern: &str, path: &str) -> bool {
    let parts = pattern.split('*').collect::<Vec<_>>();
    let Some(first) = parts.first() else {
        return path.is_empty();
    };
    if !first.is_empty() && path != first.trim_end_matches('/') && !path.starts_with(first) {
        return false;
    }

    let mut cursor = first.len().min(path.len());
    for part in parts
        .iter()
        .copied()
        .skip(1)
        .take(parts.len().saturating_sub(2))
    {
        let Some(offset) = path[cursor..].find(part) else {
            return false;
        };
        cursor += offset + part.len();
    }

    let Some(last) = parts.last() else {
        return true;
    };
    last.is_empty() || path[cursor..].ends_with(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn workflow_path_filters_read_paths_and_paths_ignore_lists() {
        let filters = workflow_path_filters(
            r#"
on:
  push:
    paths:
      - "crates/**"
      - 'docs/reference/**'
  pull_request:
    paths-ignore:
      - "*.md"
      - .github/workflows/ci.yml
"#,
        );

        assert_eq!(
            filters,
            [
                "crates/**",
                "docs/reference/**",
                "*.md",
                ".github/workflows/ci.yml"
            ]
        );
    }

    #[test]
    fn workflow_path_filters_stop_at_next_yaml_key() {
        let filters = workflow_path_filters(
            r#"
on:
  push:
    paths:
      - "crates/**"
    branches:
      - main
"#,
        );

        assert_eq!(filters, ["crates/**"]);
    }

    #[test]
    fn wildcard_path_filters_match_tracked_files_by_directory_prefix() {
        let tracked_files = tracked(&[
            "crates/tovuk/src/lib.rs",
            "examples-private/demo.rs",
            "docs/reference/packages.mdx",
        ]);

        assert!(path_filter_matches_tracked("crates/**", &tracked_files));
        assert!(path_filter_matches_tracked("docs/**/*.mdx", &tracked_files));
        assert!(!path_filter_matches_tracked("examples/**", &tracked_files));
        assert!(!path_filter_matches_tracked("*.md", &tracked_files));
    }

    #[test]
    fn exact_path_filters_match_only_tracked_files() {
        let tracked_files = tracked(&["docs/llms.txt"]);

        assert!(path_filter_matches_tracked("docs/llms.txt", &tracked_files));
        assert!(!path_filter_matches_tracked(
            "docs/missing.txt",
            &tracked_files
        ));
    }
}
