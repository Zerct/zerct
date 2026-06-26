use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::Value;

use crate::helpers::{
    CheckResult, env_int, has_markdown_link, number_field, read_json,
    reject_forbidden_public_copy_terms, require_contains, require_contains_all,
    retired_public_names,
};
use crate::mintlify_fetch::{fetch_text, normalize_target_url, retry_delay};

pub(crate) fn check_agent_readiness(target: &str) -> CheckResult {
    let base_url = normalize_target_url(target);
    let retries = env_int("TOVUK_DOCS_CHECK_RETRIES", 8)?;
    let retry_delay = retry_delay()?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("build HTTP client: {error}"))?;

    check_required_agent_paths(&client, base_url.as_str(), retries, retry_delay)?;
    check_html_paths(&client, base_url.as_str(), retries, retry_delay)?;
    check_llms_skill_and_robots(&client, base_url.as_str(), retries, retry_delay)?;
    check_content_negotiation(&client, base_url.as_str(), retries, retry_delay)?;
    check_mcp_discovery(&client, base_url.as_str(), retries, retry_delay)?;

    println!("Mintlify agent readiness checks passed for {base_url}");
    Ok(())
}

pub(crate) fn check_score(path: &str) -> CheckResult {
    let score: Value = read_json(path)?;
    let mut value = number_field(&score, "score");
    if value == 0.0 {
        value = number_field(&score, "overallScore");
    }
    let minimum = f64::from(
        i32::try_from(env_int("MINTLIFY_SCORE_MIN", 90)?)
            .map_err(|_| "MINTLIFY_SCORE_MIN must fit in an i32".to_owned())?,
    );
    if value < minimum {
        return Err(format!(
            "Mintlify score is {value:.0}/100; expected at least {minimum:.0}/100"
        ));
    }
    println!("Mintlify score is {value:.0}/100");
    Ok(())
}

fn check_required_agent_paths(
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
        "/.well-known/mcp",
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

fn check_html_paths(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
    for path in ["/", "/quickstart", "/pricing", "/reference/limits"] {
        let response = fetch_text(
            client,
            base_url,
            path,
            &[("Accept", "text/html")],
            retries,
            retry_delay,
        )?;
        reject_retired_public_names(path, response.as_str())?;
    }
    Ok(())
}

fn check_llms_skill_and_robots(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
    let llms = fetch_text(client, base_url, "/llms.txt", &[], retries, retry_delay)?;
    if !llms.lines().any(|line| line.starts_with("# ")) {
        return Err("llms.txt did not include a Markdown heading".to_owned());
    }
    if !has_markdown_link(llms.as_str()) {
        return Err("llms.txt did not include a Markdown link".to_owned());
    }

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

fn check_content_negotiation(
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

fn check_mcp_discovery(
    client: &Client,
    base_url: &str,
    retries: i64,
    retry_delay: Duration,
) -> CheckResult {
    let mcp_discovery = fetch_text(
        client,
        base_url,
        "/.well-known/mcp",
        &[],
        retries,
        retry_delay,
    )?;
    require_contains_all(
        mcp_discovery.as_str(),
        &[
            (r#""url""#, "MCP discovery"),
            (":", "MCP discovery"),
            ("/mcp", "MCP discovery"),
        ],
    )
}

fn reject_retired_public_names(label: &str, source: &str) -> CheckResult {
    let lower = source.to_lowercase();
    for retired in retired_public_names() {
        if lower.contains(retired) {
            return Err(format!("{label} contains retired public branding"));
        }
    }
    reject_forbidden_public_copy_terms(label, source)
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
