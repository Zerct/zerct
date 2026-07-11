use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, file_exists, reject_contains, require_contains},
    repo_hygiene_git::git_lines,
};

use alloc::collections::BTreeSet;

use std::path::Path;

/// Public pages required in Mintlify navigation.
const ACTIVE_NAVIGATION_PAGES: &[&str] = &[
    "index",
    "quickstart",
    "scrapers",
    "agents",
    "pricing",
    "status",
    "support",
    "changelog",
    "reference/packages",
];

/// Retired implementation pages forbidden from public navigation.
const RETIRED_NAVIGATION_PAGES: &[&str] = &[
    "deploy",
    "templates",
    "production-readiness",
    "reference/project-contract",
    "reference/workers",
    "reference/resources",
    "reference/sqlite",
    "reference/state",
    "reference/kv",
    "reference/secrets",
    "reference/storage",
    "reference/queues",
    "reference/cron",
    "reference/bindings",
    "reference/domains",
    "reference/logs-builds",
    "reference/usage-caps",
];

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&docs_page_name),
    size_of_val(&require_navigation_contract),
    size_of_val(&require_navigation_is_exhaustive),
    size_of_val(&require_navigation_pages_exist),
    size_of_val(&tracked_mdx_pages),
];

/// Convert one docs-relative MDX path into its navigation page identifier.
///
/// # Errors
///
/// Returns an error when the path is outside `docs` or contains a non-UTF-8 component.
fn docs_page_name(path: &str) -> CheckResult<String> {
    let relative = check_try!(
        Path::new(path)
            .strip_prefix("docs")
            .map_err(|error| return format!("resolve docs page {path}: {error}"))
    );
    let page_path = relative.with_extension("");
    let mut components = Vec::new();
    for component in page_path.components() {
        let Some(value) = component.as_os_str().to_str() else {
            return Err(format!(
                "docs page path is not UTF-8: {}",
                page_path.display()
            ));
        };
        components.push(value);
    }
    return Ok(components.join("/"));
}

/// Contract implementation for `require_navigation_contract`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_navigation_contract(sources: &DocsSources) -> CheckResult {
    for page in ACTIVE_NAVIGATION_PAGES {
        check_try!(require_contains(
            sources.nav_pages.as_str(),
            page,
            format!("Mintlify scraper-only navigation {page}").as_str(),
        ));
    }
    for page in RETIRED_NAVIGATION_PAGES {
        check_try!(reject_contains(
            sources.nav_pages.as_str(),
            page,
            format!("retired Mintlify navigation {page}").as_str(),
        ));
    }
    return Ok(());
}

/// Require every tracked Mintlify page to appear in public navigation.
///
/// # Errors
///
/// Returns an error when navigation contains a missing page or docs contain an orphan page.
pub(super) fn require_navigation_is_exhaustive(pages: &[String]) -> CheckResult {
    let navigated = pages
        .iter()
        .filter(|page| return !page.starts_with("http://") && !page.starts_with("https://"))
        .cloned()
        .collect::<BTreeSet<_>>();
    let present = check_try!(tracked_mdx_pages());
    if navigated == present {
        return Ok(());
    }
    let missing = navigated.difference(&present).cloned().collect::<Vec<_>>();
    let orphaned = present.difference(&navigated).cloned().collect::<Vec<_>>();
    return Err(format!(
        "Mintlify navigation and tracked MDX pages differ; missing files: [{}]; orphaned pages: [{}]",
        missing.join(", "),
        orphaned.join(", ")
    ));
}

/// Contract implementation for `require_navigation_pages_exist`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_navigation_pages_exist(pages: &[String]) -> CheckResult {
    let mut missing_pages = Vec::new();
    for page in pages {
        if page.starts_with("http://") || page.starts_with("https://") {
            continue;
        }
        let page_path = Path::new("docs").join(format!("{page}.mdx"));
        if !file_exists(page_path.as_path()) {
            missing_pages.push(page_path.display().to_string());
        }
    }
    if missing_pages.is_empty() {
        return Ok(());
    }
    return Err(format!(
        "Missing Mintlify pages:\n{}",
        missing_pages.join("\n")
    ));
}

/// Return the exact tracked Mintlify MDX page identifiers from the Git index.
///
/// # Errors
///
/// Returns an error when Git cannot list pages or a tracked page path is invalid.
fn tracked_mdx_pages() -> CheckResult<BTreeSet<String>> {
    return check_try!(git_lines(&[
        "ls-files",
        "--",
        ":(glob)docs/*.mdx",
        ":(glob)docs/**/*.mdx",
    ]))
    .iter()
    .map(|path| return docs_page_name(path.as_str()))
    .collect();
}
