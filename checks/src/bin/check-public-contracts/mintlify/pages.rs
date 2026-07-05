use std::time::Duration;

use reqwest::blocking::Client;

use crate::helpers::{CheckResult, require_contains};
use crate::mintlify_fetch::fetch_text;

use super::copy::{reject_retired_public_names, reject_retired_public_names_in_html};

pub(super) fn check_required_agent_paths(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
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
        let response = fetch_text(client, base_url, path, &[], retries, retry_delay)?;
        if response.trim().is_empty() {
            return Err(format!("{path} is empty"));
        }
        reject_retired_public_names(path, response.as_str())?;
    }
    Ok(())
}

pub(super) fn check_html_paths(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
    for path in [
        "/",
        "/quickstart",
        "/scrapers",
        "/pricing",
        "/support",
        "/reference/packages",
    ] {
        let response = fetch_text(
            client,
            base_url,
            path,
            &[("Accept", "text/html")],
            retries,
            retry_delay,
        )?;
        reject_retired_public_names_in_html(path, response.as_str())?;
    }
    Ok(())
}

pub(super) fn check_llms_skill_and_robots(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
    let llms = fetch_text(client, base_url, "/llms.txt", &[], retries, retry_delay)?;
    if !llms.lines().any(|line| line.starts_with("# ")) {
        return Err("llms.txt did not include a Markdown heading".to_owned());
    }
    require_llms_docs_index(llms.as_str())?;

    let skill = fetch_text(client, base_url, "/skill.md", &[], retries, retry_delay)?;
    if !skill.starts_with("---\n") {
        return Err("skill.md did not include frontmatter".to_owned());
    }
    if !skill.lines().any(|line| {
        line.to_lowercase().starts_with("name:") && line.to_lowercase().contains("tovuk")
    }) {
        return Err("skill.md did not include name: tovuk".to_owned());
    }

    let robots = fetch_text(client, base_url, "/robots.txt", &[], retries, retry_delay)?;
    if robots_blocks_crawlers(robots.as_str()) {
        return Err("robots.txt appears to block crawlers".to_owned());
    }
    Ok(())
}

pub(super) fn check_content_negotiation(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
    let markdown = fetch_text(
        client,
        base_url,
        "/",
        &[("Accept", "text/markdown")],
        retries,
        retry_delay,
    )?;
    require_contains(markdown.as_str(), "Tovuk", "Markdown content negotiation")?;

    let plaintext = fetch_text(
        client,
        base_url,
        "/",
        &[("Accept", "text/plain")],
        retries,
        retry_delay,
    )?;
    require_contains(
        plaintext.as_str(),
        "Tovuk",
        "Plain text content negotiation",
    )
}

fn require_llms_docs_index(source: &str) -> CheckResult {
    for path in [
        "docs/quickstart.mdx",
        "docs/scrapers.mdx",
        "docs/agents.mdx",
        "docs/pricing.mdx",
        "docs/reference/packages.mdx",
        "docs/support.mdx",
    ] {
        require_contains(source, path, "llms.txt docs index")?;
    }
    Ok(())
}

fn robots_blocks_crawlers(source: &str) -> bool {
    let disallows_all = source.lines().any(|line| {
        let lower = line.to_lowercase();
        lower
            .split_once(':')
            .filter(|(name, _)| name.trim() == "disallow")
            .is_some_and(|(_, value)| value.trim_start().starts_with('/'))
    });
    let allows_all = source
        .lines()
        .any(|line| line.to_lowercase().trim() == "allow: /");
    disallows_all && !allows_all
}
