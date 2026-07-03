use std::{thread::sleep, time::Duration};

use reqwest::{Url, blocking::Client};
use serde::Deserialize;
use serde_json::Value;

use crate::helpers::{
    CheckResult, env_int, number_field, read_json, reject_forbidden_public_copy_terms,
    require_contains, retired_public_names,
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
    fetch_text_until_valid(
        client,
        base_url,
        "/.well-known/mcp",
        &[],
        retries,
        retry_delay,
        |source| {
            if source.trim().is_empty() {
                return Err("/.well-known/mcp is empty".to_owned());
            }
            reject_retired_public_names_in_mcp_discovery(source)?;
            require_mcp_urls_on_base_host(base_url, source)
        },
    )?;
    Ok(())
}

fn fetch_text_until_valid(
    client: &Client,
    base_url: &str,
    path: &str,
    headers: &[(&str, &str)],
    retries: i64,
    retry_delay: Duration,
    validate: impl Fn(&str) -> CheckResult,
) -> CheckResult<String> {
    let mut last_error = "request was not attempted".to_owned();
    for attempt in 0..=retries {
        match fetch_text(client, base_url, path, headers, 0, retry_delay) {
            Ok(text) => match validate(text.as_str()) {
                Ok(()) => return Ok(text),
                Err(error) => last_error = error,
            },
            Err(error) => last_error = error,
        }
        if attempt < retries {
            sleep(retry_delay);
        }
    }
    Err(last_error)
}

fn require_mcp_urls_on_base_host(base_url: &str, source: &str) -> CheckResult {
    let base = Url::parse(base_url).map_err(|error| format!("parse docs base URL: {error}"))?;
    let base_host = base
        .host_str()
        .ok_or_else(|| format!("docs base URL must include a host: {base_url}"))?;
    let discovery = serde_json::from_str::<McpDiscovery>(source)
        .map_err(|error| format!("parse MCP discovery JSON: {error}"))?;
    require_mcp_url_on_base_host(base.scheme(), base_host, discovery.url.as_str())?;
    for server in &discovery.servers {
        require_mcp_url_on_base_host(base.scheme(), base_host, server.url.as_str())?;
    }
    Ok(())
}

fn require_mcp_url_on_base_host(base_scheme: &str, base_host: &str, url: &str) -> CheckResult {
    let parsed =
        Url::parse(url).map_err(|error| format!("parse MCP discovery URL {url}: {error}"))?;
    let Some(host) = parsed.host_str() else {
        return Err(format!("MCP discovery URL {url} must include a host"));
    };
    if parsed.scheme() != base_scheme || (host != base_host && !host.ends_with(".mintlify.me")) {
        return Err(format!(
            "MCP discovery URL {url} must stay on {base_scheme}://{base_host} or a Mintlify MCP host"
        ));
    }
    if parsed.path() != "/mcp" {
        return Err(format!(
            "MCP discovery URL {url} must use the public /mcp path"
        ));
    }
    Ok(())
}

fn reject_retired_public_names_in_mcp_discovery(source: &str) -> CheckResult {
    let discovery = serde_json::from_str::<McpDiscovery>(source)
        .map_err(|error| format!("parse MCP discovery JSON: {error}"))?;
    for server in discovery.servers {
        reject_retired_public_names("/.well-known/mcp server name", server.name.as_str())?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct McpDiscovery {
    url: String,
    #[serde(default)]
    servers: Vec<McpServer>,
}

#[derive(Deserialize)]
struct McpServer {
    name: String,
    url: String,
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

fn reject_retired_public_names_in_html(label: &str, source: &str) -> CheckResult {
    let visible_copy = html_visible_copy(source);
    reject_retired_public_names(label, visible_copy.as_str())
}

fn html_visible_copy(source: &str) -> String {
    let source = html_body(source).unwrap_or(source);
    let without_scripts = remove_html_element_blocks(source, "script");
    let without_styles = remove_html_element_blocks(without_scripts.as_str(), "style");
    html_text_nodes(without_styles.as_str())
}

fn html_body(source: &str) -> Option<&str> {
    let lower = source.to_lowercase();
    let body_start_tag = lower.find("<body")?;
    let body_start = lower[body_start_tag..]
        .find('>')
        .map(|offset| body_start_tag + offset + 1)?;
    let body_end = lower[body_start..]
        .find("</body>")
        .map_or(source.len(), |offset| body_start + offset);
    source.get(body_start..body_end)
}

fn remove_html_element_blocks(source: &str, tag: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut remaining = source;
    let open = format!("<{tag}");
    let close = format!("</{tag}>");

    loop {
        let lower = remaining.to_lowercase();
        let Some(start) = lower.find(open.as_str()) else {
            output.push_str(remaining);
            return output;
        };
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start..];
        let after_start_lower = after_start.to_lowercase();
        let Some(end) = after_start_lower.find(close.as_str()) else {
            return output;
        };
        remaining = &after_start[end + close.len()..];
    }
}

fn html_text_nodes(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut in_tag = false;
    for character in source.chars() {
        match character {
            '<' => {
                in_tag = true;
                output.push(' ');
            }
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    decode_html_text(output.as_str())
}

fn decode_html_text(source: &str) -> String {
    source
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
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

#[cfg(test)]
mod tests {
    use super::{reject_retired_public_names_in_html, require_mcp_urls_on_base_host};

    #[test]
    fn accepts_public_mcp_urls_on_docs_host() {
        let source = r#"{"url":"https://docs.tovuk.com/mcp","servers":[{"name":"public","url":"https://docs.tovuk.com/mcp"}]}"#;

        assert!(
            require_mcp_urls_on_base_host("https://docs.tovuk.com", source).is_ok(),
            "MCP discovery should accept public docs-domain URLs"
        );
    }

    #[test]
    fn accepts_generated_mintlify_hosts_for_mcp_endpoint() {
        let source = r#"{"url":"https://project-123.main-kill-isr.mintlify.me/mcp","servers":[{"name":"public","url":"https://project-123.main-kill-isr.mintlify.me/mcp"}]}"#;

        assert!(
            require_mcp_urls_on_base_host("https://docs.tovuk.com", source).is_ok(),
            "Mintlify custom-domain MCP discovery may advertise a generated Mintlify MCP host"
        );
    }

    #[test]
    fn rejects_unrelated_mcp_hosts() {
        let source = r#"{"url":"https://example.com/mcp","servers":[{"name":"public","url":"https://example.com/mcp"}]}"#;

        let result = require_mcp_urls_on_base_host("https://docs.tovuk.com", source);
        assert!(
            matches!(result, Err(message) if message.contains("Mintlify MCP host")),
            "MCP discovery should reject hosts outside the public docs host and Mintlify MCP hosts"
        );
    }

    #[test]
    fn rejects_non_public_mcp_paths() {
        let source = r#"{"url":"https://docs.tovuk.com/authed/mcp","servers":[{"name":"public","url":"https://docs.tovuk.com/authed/mcp"}]}"#;

        let result = require_mcp_urls_on_base_host("https://docs.tovuk.com", source);
        assert!(
            matches!(result, Err(message) if message.contains("public /mcp path")),
            "MCP discovery should use the public /mcp path"
        );
    }

    #[test]
    fn ignores_unrelated_discovery_url_metadata() {
        let source = r#"{"url":"https://docs.tovuk.com/mcp","metadata":{"url":"https://example.com/not-an-endpoint"}}"#;

        assert!(
            require_mcp_urls_on_base_host("https://docs.tovuk.com", source).is_ok(),
            "MCP discovery should validate endpoint fields, not unrelated metadata"
        );
    }

    #[test]
    fn ignores_generated_mintlify_project_slug_in_html_assets() {
        let source = r#"
            <!doctype html>
            <html>
              <head>
                <meta property="og:image" content="https://zerct-4cdab021.mintlify.app/og.png">
                <link rel="preload" href="/mintlify-assets/zerct-4cdab021/logo.svg">
              </head>
              <body>
                <main>
                  <h1>Tovuk</h1>
                  <p>Paid public-data scraper API.</p>
                </main>
              </body>
            </html>
        "#;

        assert!(
            reject_retired_public_names_in_html("/", source).is_ok(),
            "Mintlify immutable internal slugs in asset URLs should not fail visible copy checks"
        );
    }

    #[test]
    fn rejects_visible_retired_branding_in_html() {
        let source = r"
            <!doctype html>
            <html>
              <body>
                <main>
                  <h1>Zerct</h1>
                </main>
              </body>
            </html>
        ";

        let result = reject_retired_public_names_in_html("/", source);
        assert!(
            matches!(result, Err(message) if message.contains("retired public branding")),
            "Visible retired public branding must still fail the docs readiness check"
        );
    }
}
