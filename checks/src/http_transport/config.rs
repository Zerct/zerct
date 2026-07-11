//! Shared HTTP client and request construction.

use super::{LoopbackClient, RequestHeaders, RequestResult, SecureClient, TransportResult};

use super::redirect::MAXIMUM_REDIRECT_LIMIT;

use core::time::Duration;

use http::{
    HeaderMap, HeaderName, HeaderValue, Method, Request, Uri,
    header::{
        CONNECTION, CONTENT_LENGTH, HOST, PROXY_AUTHORIZATION, TE, TRAILER, TRANSFER_ENCODING,
        UPGRADE, USER_AGENT,
    },
};

use http_body_util::Empty;

use hyper_rustls::HttpsConnectorBuilder;

use hyper_util::{
    client::legacy::{Client as HyperClient, connect::HttpConnector},
    rt::TokioExecutor,
};

use rustls::crypto::ring::default_provider;

use tokio::runtime::{Builder as RuntimeBuilder, Runtime};

use url::Url;

/// Compile-time references preserve the named configuration boundaries.
const _: [usize; 0x09] = [
    size_of_val(&build_headers),
    size_of_val(&build_loopback_client),
    size_of_val(&build_request),
    size_of_val(&build_runtime),
    size_of_val(&build_secure_client),
    size_of_val(&header_is_forbidden),
    size_of_val(&insert_header),
    size_of_val(&parse_user_agent),
    size_of_val(&validate_configuration),
];

/// Build the immutable caller and user-agent header map.
///
/// # Errors
///
/// Returns an error for an invalid, reserved, or duplicate header.
pub(super) fn build_headers<'headers>(
    headers: &'headers RequestHeaders<'headers>,
    user_agent: &HeaderValue,
) -> TransportResult<HeaderMap> {
    let mut result = HeaderMap::new();
    if result.insert(USER_AGENT, user_agent.clone()).is_some() {
        return Err("HTTP user-agent header was duplicated".to_owned());
    }
    for (name, value) in headers.iter().copied() {
        check_try!(insert_header(&mut result, name, value));
    }
    return Ok(result);
}

/// Build a loopback-only plaintext Hyper client.
pub(super) fn build_loopback_client(connect_timeout: Duration) -> LoopbackClient {
    let mut connector = HttpConnector::new();
    connector.enforce_http(true);
    connector.set_connect_timeout(Some(connect_timeout));
    connector.set_nodelay(true);
    return HyperClient::builder(TokioExecutor::new()).build(connector);
}

/// Build one GET request from a validated URL and header map.
///
/// # Errors
///
/// Returns an error when the URL cannot be represented as an HTTP request target.
pub(super) fn build_request(url: &Url, headers: HeaderMap) -> RequestResult {
    let uri = check_try!(
        url.as_str()
            .parse::<Uri>()
            .map_err(|error| format!("convert URL {url} to an HTTP URI: {error}"))
    );
    let mut request = check_try!(
        Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Empty::new())
            .map_err(|error| format!("build HTTP request for {url}: {error}"))
    );
    *request.headers_mut() = headers;
    return Ok(request);
}

/// Build the current-thread Tokio runtime used by the blocking facade.
///
/// # Errors
///
/// Returns an error when Tokio cannot construct the runtime.
pub(super) fn build_runtime() -> TransportResult<Runtime> {
    return RuntimeBuilder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|error| format!("build HTTP runtime: {error}"));
}

/// Build the certificate- and hostname-verifying Rustls Hyper client.
///
/// # Errors
///
/// Returns an error when native roots or the Ring-backed client cannot be built.
pub(super) fn build_secure_client(connect_timeout: Duration) -> TransportResult<SecureClient> {
    let provider = default_provider();
    let tls_builder = check_try!(
        HttpsConnectorBuilder::new()
            .with_provider_and_native_roots(provider)
            .map_err(|error| format!("load native TLS roots: {error}"))
    );
    let mut connector = HttpConnector::new();
    connector.enforce_http(false);
    connector.set_connect_timeout(Some(connect_timeout));
    connector.set_nodelay(true);
    let secure_connector = tls_builder
        .https_only()
        .enable_http1()
        .enable_http2()
        .wrap_connector(connector);
    return Ok(HyperClient::builder(TokioExecutor::new()).build(secure_connector));
}

/// Reject request headers that can subvert routing or HTTP message framing.
fn header_is_forbidden(name: &HeaderName) -> bool {
    return [
        CONNECTION,
        CONTENT_LENGTH,
        HOST,
        PROXY_AUTHORIZATION,
        TE,
        TRAILER,
        TRANSFER_ENCODING,
        UPGRADE,
        USER_AGENT,
    ]
    .contains(name);
}

/// Insert one validated caller-provided header without silent replacement.
///
/// # Errors
///
/// Returns an error for an invalid, reserved, or duplicate header.
fn insert_header(headers: &mut HeaderMap, name: &str, value: &str) -> TransportResult {
    let header_name = check_try!(
        HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| format!("invalid HTTP header name {name}: {error}"))
    );
    if header_is_forbidden(&header_name) {
        return Err(format!(
            "HTTP header {header_name} is reserved by the transport"
        ));
    }
    let header_value = check_try!(
        HeaderValue::from_str(value)
            .map_err(|error| format!("invalid HTTP header {header_name}: {error}"))
    );
    if headers.insert(header_name.clone(), header_value).is_some() {
        return Err(format!("HTTP header {header_name} was duplicated"));
    }
    return Ok(());
}

/// Parse the required non-empty public user agent.
///
/// # Errors
///
/// Returns an error when the value is empty or invalid for an HTTP header.
pub(super) fn parse_user_agent(source: &str) -> TransportResult<HeaderValue> {
    if source.trim().is_empty() {
        return Err("HTTP user agent must not be empty".to_owned());
    }
    return HeaderValue::from_str(source)
        .map_err(|error| format!("invalid HTTP user agent: {error}"));
}

/// Require non-zero deadlines, a bounded redirect count, and a valid user agent.
///
/// # Errors
///
/// Returns an error when any client policy value is invalid.
pub(super) fn validate_configuration(
    connect_timeout: Duration,
    request_timeout: Duration,
    redirect_limit: u8,
    user_agent: &str,
) -> TransportResult {
    if connect_timeout.is_zero() || request_timeout.is_zero() {
        return Err("HTTP connect and request timeouts must be non-zero".to_owned());
    }
    if redirect_limit > MAXIMUM_REDIRECT_LIMIT {
        return Err(format!(
            "HTTP redirect limit {redirect_limit} exceeds {MAXIMUM_REDIRECT_LIMIT}"
        ));
    }
    drop(check_try!(parse_user_agent(user_agent)));
    return Ok(());
}
