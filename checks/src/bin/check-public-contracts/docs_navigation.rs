use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, file_exists, reject_contains, require_contains},
};

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
const _: [usize; 0x0002] = [
    size_of_val(&require_navigation_contract),
    size_of_val(&require_navigation_pages_exist),
];

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
