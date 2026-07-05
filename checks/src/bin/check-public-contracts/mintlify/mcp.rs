use std::{thread::sleep, time::Duration};

use reqwest::{Url, blocking::Client};
use serde::Deserialize;

use crate::helpers::CheckResult;
use crate::mintlify_fetch::fetch_text;

use super::copy::reject_retired_public_names;

pub(super) fn check_mcp_discovery(
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

#[cfg(test)]
mod tests {
    use super::require_mcp_urls_on_base_host;

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
}
