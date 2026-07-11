//! Workflow path-filter parsing and tracked-file matching.

use alloc::collections::BTreeSet;

use super::{HostedActionsCheck, PathFilterBlock, PathFilters};

impl PathFilters for HostedActionsCheck {
    fn advance_glob_cursor(&self, cursor: usize, part: &str, path: &str) -> Option<usize> {
        let remaining_path = check_some!(path.get(cursor..));
        let offset = check_some!(remaining_path.find(part));
        return Some(cursor.saturating_add(offset).saturating_add(part.len()));
    }

    fn glob_matches(&self, pattern: &str, path: &str) -> bool {
        let parts = pattern.split('*').collect::<Vec<_>>();
        let Some(first) = parts.first() else {
            return path.is_empty();
        };
        if !first.is_empty() && path != first.trim_end_matches('/') && !path.starts_with(first) {
            return false;
        }

        let initial_cursor = first.len().min(path.len());
        let Some(cursor) = parts
            .iter()
            .copied()
            .skip(0x1)
            .take(parts.len().saturating_sub(0x2))
            .try_fold(initial_cursor, |cursor, part| {
                return self.advance_glob_cursor(cursor, part, path);
            })
        else {
            return false;
        };

        let Some(last) = parts.last() else {
            return true;
        };
        return last.is_empty()
            || path
                .get(cursor..)
                .is_some_and(|remaining_path| return remaining_path.ends_with(last));
    }

    fn leading_spaces(&self, line: &str) -> usize {
        return line.bytes().take_while(|byte| return *byte == b' ').count();
    }

    fn path_filter_matches_tracked(&self, filter: &str, tracked_files: &BTreeSet<String>) -> bool {
        if !filter.contains('*') {
            return tracked_files.contains(filter);
        }
        return tracked_files
            .iter()
            .any(|path| return self.glob_matches(filter, path));
    }

    fn process_path_filter_line(&self, line: &str, state: &mut PathFilterBlock) {
        let indent = self.leading_spaces(line);
        let trimmed = line.trim();
        if matches!(trimmed, "paths:" | "paths-ignore:") {
            state.block_indent = Some(indent);
            return;
        }
        let Some(parent_indent) = state.block_indent else {
            return;
        };
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return;
        }
        if indent <= parent_indent {
            state.block_indent = None;
            return;
        }
        let Some(value) = trimmed.strip_prefix("- ") else {
            return;
        };
        state
            .filters
            .push(self.unquote_yaml_string(value.trim()).to_owned());
    }

    fn unquote_yaml_string<'value>(&self, value: &'value str) -> &'value str {
        return value
            .strip_prefix('"')
            .and_then(|inner| return inner.strip_suffix('"'))
            .or_else(|| {
                let inner = check_some!(value.strip_prefix('\''));
                return inner.strip_suffix('\'');
            })
            .unwrap_or(value);
    }

    fn workflow_path_filters(&self, contents: &str) -> Vec<String> {
        let mut state = PathFilterBlock {
            block_indent: None,
            filters: Vec::new(),
        };
        contents
            .lines()
            .for_each(|line| self.process_path_filter_line(line, &mut state));
        return state.filters;
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use super::{HostedActionsCheck, PathFilters as _};

    /// Verify that exact filters accept only tracked paths.
    ///
    /// # Panics
    ///
    /// Panics when exact path matching violates the workflow contract.
    #[test]
    fn exact_path_filters_match_only_tracked_files() {
        let checker = HostedActionsCheck;
        let tracked_files = tracked(&["docs/llms.txt"]);

        assert!(
            checker.path_filter_matches_tracked("docs/llms.txt", &tracked_files),
            "the exact tracked path should match"
        );
        assert!(
            !checker.path_filter_matches_tracked("docs/missing.txt", &tracked_files),
            "an absent exact path should not match"
        );
    }

    /// Build the tracked-file fixture used by path-filter tests.
    fn tracked(paths: &[&str]) -> BTreeSet<String> {
        return paths.iter().map(|path| return (*path).to_owned()).collect();
    }

    /// Verify that wildcard filters respect directory and suffix boundaries.
    ///
    /// # Panics
    ///
    /// Panics when wildcard path matching violates the workflow contract.
    #[test]
    fn wildcard_path_filters_match_tracked_files_by_directory_prefix() {
        let checker = HostedActionsCheck;
        let tracked_files = tracked(&[
            "crates/tovuk/src/lib.rs",
            "examples-private/demo.rs",
            "docs/reference/packages.mdx",
        ]);

        assert!(
            checker.path_filter_matches_tracked("crates/**", &tracked_files),
            "the crate directory filter should match"
        );
        assert!(
            checker.path_filter_matches_tracked("docs/**/*.mdx", &tracked_files),
            "the nested documentation filter should match"
        );
        assert!(
            !checker.path_filter_matches_tracked("examples/**", &tracked_files),
            "a prefix must not match a longer sibling directory name"
        );
        assert!(
            !checker.path_filter_matches_tracked("*.md", &tracked_files),
            "a suffix filter should not match different extensions"
        );
    }

    /// Verify that both supported path-filter lists are extracted.
    ///
    /// # Panics
    ///
    /// Panics when workflow path-filter parsing loses or changes a filter.
    #[test]
    fn workflow_path_filters_read_paths_and_paths_ignore_lists() {
        let checker = HostedActionsCheck;
        let filters = checker.workflow_path_filters(
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
            ],
            "both supported workflow path-filter lists should be parsed"
        );
    }

    /// Verify that a sibling YAML key closes a path-filter block.
    ///
    /// # Panics
    ///
    /// Panics when path-filter parsing crosses into a sibling YAML key.
    #[test]
    fn workflow_path_filters_stop_at_next_yaml_key() {
        let checker = HostedActionsCheck;
        let filters = checker.workflow_path_filters(
            r#"
on:
  push:
    paths:
      - "crates/**"
    branches:
      - main
"#,
        );

        assert_eq!(
            filters,
            ["crates/**"],
            "a sibling YAML key should end the path-filter block"
        );
    }
}
