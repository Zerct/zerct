use std::path::Path;

use crate::{
    docs_sources::DocsSources,
    helpers::{CheckResult, file_exists, reject_contains, require_contains},
};

pub(crate) fn require_navigation_pages_exist(pages: &[String]) -> CheckResult {
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
        Ok(())
    } else {
        Err(format!(
            "Missing Mintlify pages:\n{}",
            missing_pages.join("\n")
        ))
    }
}

pub(crate) fn require_navigation_contract(sources: &DocsSources) -> CheckResult {
    for page in [
        "index",
        "quickstart",
        "scrapers",
        "agents",
        "pricing",
        "status",
        "support",
        "changelog",
        "reference/packages",
    ] {
        require_contains(
            sources.nav_pages.as_str(),
            page,
            format!("Mintlify scraper-only navigation {page}").as_str(),
        )?;
    }
    for page in [
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
    ] {
        reject_contains(
            sources.nav_pages.as_str(),
            page,
            format!("retired Mintlify navigation {page}").as_str(),
        )?;
    }
    Ok(())
}
