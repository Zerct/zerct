use crate::helpers::CheckResult;

use crate::mintlify_fetch::{FetchContext, RequestHeaders, fetch_text_once};

use serde::Deserialize;

use serde_json::from_str;

use std::thread::sleep;

use super::copy::reject_retired_public_names;

use url::Url;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0005] = [
    size_of_val(&check_mcp_discovery),
    size_of_val(&fetch_text_until_valid),
    size_of_val(&reject_retired_public_names_in_mcp_discovery),
    size_of_val(&require_mcp_urls_on_base_host),
    size_of_val(&validate_mcp_discovery),
];

#[derive(Deserialize)]
/// Contract representation for `McpDiscovery`.
struct McpDiscovery {
    #[serde(default)]
    /// Contract data stored in `servers`.
    servers: Vec<McpServer>,
    /// Contract data stored in `url`.
    url: String,
}

#[derive(Deserialize)]
/// Contract representation for `McpServer`.
struct McpServer {
    /// Contract data stored in `name`.
    name: String,
    /// Contract data stored in `url`.
    url: String,
}

/// Validation callback applied to one fetched discovery document.
type McpValidator = fn(&FetchContext, &str) -> CheckResult;

/// Contract implementation for `check_mcp_discovery`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn check_mcp_discovery(context: &FetchContext) -> CheckResult {
    drop(check_try!(fetch_text_until_valid(
        context,
        "/.well-known/mcp",
        &[],
        validate_mcp_discovery,
    )));
    return Ok(());
}

/// Contract implementation for `fetch_text_until_valid`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn fetch_text_until_valid(
    context: &FetchContext,
    path: &str,
    headers: &RequestHeaders,
    validate: McpValidator,
) -> CheckResult<String> {
    let mut last_error = "request was not attempted".to_owned();
    for attempt in 0..=context.retries() {
        match fetch_text_once(context, path, headers) {
            Ok(text) => match validate(context, text.as_str()) {
                Ok(()) => return Ok(text),
                Err(error) => last_error = error,
            },
            Err(error) => last_error = error,
        }
        if attempt < context.retries() {
            sleep(context.retry_delay());
        }
    }
    return Err(last_error);
}

/// Contract implementation for `reject_retired_public_names_in_mcp_discovery`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn reject_retired_public_names_in_mcp_discovery(source: &str) -> CheckResult {
    let discovery = check_try!(
        from_str::<McpDiscovery>(source)
            .map_err(|error| format!("parse MCP discovery JSON: {error}"))
    );
    for server in discovery.servers {
        check_try!(reject_retired_public_names(
            "/.well-known/mcp server name",
            server.name.as_str()
        ));
    }
    return Ok(());
}

/// Contract implementation for `require_mcp_url_on_base_host`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn require_mcp_url_on_base_host(base_scheme: &str, base_host: &str, url: &str) -> CheckResult {
    let parsed = check_try!(
        Url::parse(url).map_err(|error| format!("parse MCP discovery URL {url}: {error}"))
    );
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
    return Ok(());
}

/// Contract implementation for `require_mcp_urls_on_base_host`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn require_mcp_urls_on_base_host(base_url: &str, source: &str) -> CheckResult {
    let base =
        check_try!(Url::parse(base_url).map_err(|error| format!("parse docs base URL: {error}")));
    let base_host = check_try!(
        base.host_str()
            .ok_or_else(|| format!("docs base URL must include a host: {base_url}"))
    );
    let discovery = check_try!(
        from_str::<McpDiscovery>(source)
            .map_err(|error| format!("parse MCP discovery JSON: {error}"))
    );
    check_try!(require_mcp_url_on_base_host(
        base.scheme(),
        base_host,
        discovery.url.as_str()
    ));
    for server in &discovery.servers {
        check_try!(require_mcp_url_on_base_host(
            base.scheme(),
            base_host,
            server.url.as_str()
        ));
    }
    return Ok(());
}

/// Validate a fetched MCP discovery document against the public docs context.
///
/// # Errors
///
/// Returns an error when discovery content is empty or violates the public MCP contract.
fn validate_mcp_discovery(context: &FetchContext, source: &str) -> CheckResult {
    if source.trim().is_empty() {
        return Err("/.well-known/mcp is empty".to_owned());
    }
    check_try!(reject_retired_public_names_in_mcp_discovery(source));
    return require_mcp_urls_on_base_host(context.base_url(), source);
}
#[cfg(test)]
mod tests {
    use super::require_mcp_urls_on_base_host;

    /// Verify generated Mintlify hosts are accepted for MCP endpoints.
    ///
    /// # Panics
    ///
    /// Panics when a valid generated Mintlify MCP host is rejected.
    #[test]
    fn accepts_generated_mintlify_hosts_for_mcp_endpoint() {
        let source = r#"{"url":"https://project-123.main-kill-isr.mintlify.me/mcp","servers":[{"name":"public","url":"https://project-123.main-kill-isr.mintlify.me/mcp"}]}"#;

        assert!(
            require_mcp_urls_on_base_host("https://docs.tovuk.com", source).is_ok(),
            "Mintlify custom-domain MCP discovery may advertise a generated Mintlify MCP host"
        );
    }

    /// Verify the public docs host is accepted for MCP endpoints.
    ///
    /// # Panics
    ///
    /// Panics when a valid docs-host MCP endpoint is rejected.
    #[test]
    fn accepts_public_mcp_urls_on_docs_host() {
        let source = r#"{"url":"https://docs.tovuk.com/mcp","servers":[{"name":"public","url":"https://docs.tovuk.com/mcp"}]}"#;

        assert!(
            require_mcp_urls_on_base_host("https://docs.tovuk.com", source).is_ok(),
            "MCP discovery should accept public docs-domain URLs"
        );
    }

    /// Verify unrelated URL metadata is ignored by endpoint validation.
    ///
    /// # Panics
    ///
    /// Panics when unrelated discovery metadata is treated as an endpoint.
    #[test]
    fn ignores_unrelated_discovery_url_metadata() {
        let source = r#"{"url":"https://docs.tovuk.com/mcp","metadata":{"url":"https://example.com/not-an-endpoint"}}"#;

        assert!(
            require_mcp_urls_on_base_host("https://docs.tovuk.com", source).is_ok(),
            "MCP discovery should validate endpoint fields, not unrelated metadata"
        );
    }
    /// Verify non-public MCP endpoint paths are rejected.
    ///
    /// # Panics
    ///
    /// Panics when an authenticated or otherwise non-public path is accepted.
    #[test]
    fn rejects_non_public_mcp_paths() {
        let source = r#"{"url":"https://docs.tovuk.com/authed/mcp","servers":[{"name":"public","url":"https://docs.tovuk.com/authed/mcp"}]}"#;

        let result = require_mcp_urls_on_base_host("https://docs.tovuk.com", source);
        assert!(
            matches!(result, Err(message) if message.contains("public /mcp path")),
            "MCP discovery should use the public /mcp path"
        );
    }

    /// Verify unrelated hosts are rejected for MCP endpoints.
    ///
    /// # Panics
    ///
    /// Panics when an unrelated host is accepted.
    #[test]
    fn rejects_unrelated_mcp_hosts() {
        let source = r#"{"url":"https://example.com/mcp","servers":[{"name":"public","url":"https://example.com/mcp"}]}"#;

        let result = require_mcp_urls_on_base_host("https://docs.tovuk.com", source);
        assert!(
            matches!(result, Err(message) if message.contains("Mintlify MCP host")),
            "MCP discovery should reject hosts outside the public docs host and Mintlify MCP hosts"
        );
    }
}
