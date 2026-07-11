use crate::helpers::{CheckResult, require_contains};

use crate::mintlify_fetch::{FetchContext, fetch_text};

use super::copy::{reject_retired_public_names, reject_retired_public_names_in_html};

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 6] = [
    size_of_val(&check_content_negotiation),
    size_of_val(&check_html_paths),
    size_of_val(&check_llms_skill_and_robots),
    size_of_val(&check_required_agent_paths),
    size_of_val(&require_llms_docs_index),
    size_of_val(&robots_blocks_crawlers),
];

/// Contract implementation for `check_content_negotiation`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_content_negotiation(context: &FetchContext) -> CheckResult {
    let markdown = check_try!(fetch_text(context, "/", &[("Accept", "text/markdown")],));
    check_try!(require_contains(
        markdown.as_str(),
        "Tovuk",
        "Markdown content negotiation"
    ));

    let plaintext = check_try!(fetch_text(context, "/", &[("Accept", "text/plain")],));
    return require_contains(
        plaintext.as_str(),
        "Tovuk",
        "Plain text content negotiation",
    );
}

/// Contract implementation for `check_html_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_html_paths(context: &FetchContext) -> CheckResult {
    for path in [
        "/",
        "/quickstart",
        "/scrapers",
        "/pricing",
        "/support",
        "/reference/packages",
    ] {
        let response = check_try!(fetch_text(context, path, &[("Accept", "text/html")],));
        check_try!(reject_retired_public_names_in_html(path, response.as_str()));
    }
    return Ok(());
}

/// Contract implementation for `check_llms_skill_and_robots`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_llms_skill_and_robots(context: &FetchContext) -> CheckResult {
    let llms = check_try!(fetch_text(context, "/llms.txt", &[],));
    if !llms.lines().any(|line| return line.starts_with("# ")) {
        return Err("llms.txt did not include a Markdown heading".to_owned());
    }
    check_try!(require_llms_docs_index(llms.as_str()));

    let skill = check_try!(fetch_text(context, "/skill.md", &[],));
    if !skill.starts_with("---\n") {
        return Err("skill.md did not include frontmatter".to_owned());
    }
    if !skill.lines().any(|line| {
        return line.to_lowercase().starts_with("name:") && line.to_lowercase().contains("tovuk");
    }) {
        return Err("skill.md did not include name: tovuk".to_owned());
    }

    let robots = check_try!(fetch_text(context, "/robots.txt", &[],));
    if robots_blocks_crawlers(robots.as_str()) {
        return Err("robots.txt appears to block crawlers".to_owned());
    }
    return Ok(());
}

/// Contract implementation for `check_required_agent_paths`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_required_agent_paths(context: &FetchContext) -> CheckResult {
    for path in [
        "/llms.txt",
        "/llms-full.txt",
        "/skill.md",
        "/.well-known/skills/index.json",
        "/.well-known/agent-skills/index.json",
        "/sitemap.xml",
        "/robots.txt",
        "/openapi.json",
    ] {
        let response = check_try!(fetch_text(context, path, &[],));
        if response.trim().is_empty() {
            return Err(format!("{path} is empty"));
        }
        check_try!(reject_retired_public_names(path, response.as_str()));
    }
    return Ok(());
}

/// Contract implementation for `require_llms_docs_index`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_llms_docs_index(source: &str) -> CheckResult {
    for path in [
        "docs/quickstart.mdx",
        "docs/scrapers.mdx",
        "docs/agents.mdx",
        "docs/pricing.mdx",
        "docs/reference/packages.mdx",
        "docs/support.mdx",
    ] {
        check_try!(require_contains(source, path, "llms.txt docs index"));
    }
    return Ok(());
}

/// Contract implementation for `robots_blocks_crawlers`.
pub(super) fn robots_blocks_crawlers(source: &str) -> bool {
    let disallows_all = source.lines().any(|line| {
        let lower = line.to_lowercase();
        return lower
            .split_once(':')
            .filter(|pair| return pair.0.trim() == "disallow")
            .is_some_and(|pair| return pair.1.trim_start().starts_with('/'));
    });
    let allows_all = source
        .lines()
        .any(|line| return line.to_lowercase().trim() == "allow: /");
    return disallows_all && !allows_all;
}
