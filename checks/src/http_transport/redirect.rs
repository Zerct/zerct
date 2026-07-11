//! Redirect and URL safety policy for the shared HTTP transport.

use super::{RedirectResult, TransportResult};

use core::net::IpAddr;

use alloc::collections::BTreeSet;

use http::{
    HeaderMap, StatusCode,
    header::{ACCEPT, ACCEPT_ENCODING, LOCATION, USER_AGENT},
};

use hyper::{Response as HyperResponse, body::Incoming};

use url::Url;

/// Largest redirect count accepted by the shared transport.
pub(super) const MAXIMUM_REDIRECT_LIMIT: u8 = 0x0a;

/// Compile-time references preserve the named redirect boundaries.
const _: [usize; 0x0b] = [
    size_of_val(&advance_redirect),
    size_of_val(&is_loopback_host),
    size_of_val(&is_redirect),
    size_of_val(&joined_redirect_url),
    size_of_val(&next_redirect),
    size_of_val(&normalize_url),
    size_of_val(&parse_request_url),
    size_of_val(&record_visit),
    size_of_val(&redirect_headers),
    size_of_val(&validate_redirect_transition),
    size_of_val(&validate_request_url),
];

/// Advance one redirect state or report that the response is terminal.
///
/// # Errors
///
/// Returns an error when the redirect count cannot be incremented.
pub(super) fn advance_redirect(
    current_url: &mut Url,
    followed_redirects: &mut u8,
    redirect_url: Option<Url>,
) -> TransportResult<Option<()>> {
    let Some(next_url) = redirect_url else {
        return Ok(None);
    };
    *followed_redirects = check_try!(
        followed_redirects
            .checked_add(0x01)
            .ok_or_else(|| return "HTTP redirect count overflow".to_owned())
    );
    *current_url = next_url;
    return Ok(Some(()));
}

/// Return whether a URL host is an explicit loopback endpoint.
fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let address_text = host
        .strip_prefix('[')
        .and_then(|without_prefix| return without_prefix.strip_suffix(']'))
        .unwrap_or(host);
    return address_text
        .parse::<IpAddr>()
        .is_ok_and(|address| return address.is_loopback());
}

/// Return whether one status requires redirect processing for a GET request.
const fn is_redirect(status: StatusCode) -> bool {
    return matches!(
        status,
        StatusCode::MOVED_PERMANENTLY
            | StatusCode::FOUND
            | StatusCode::SEE_OTHER
            | StatusCode::TEMPORARY_REDIRECT
            | StatusCode::PERMANENT_REDIRECT
    );
}

/// Parse the required Location header against the current request URL.
///
/// # Errors
///
/// Returns an error when Location is missing, invalid, or resolves to an unsafe URL.
fn joined_redirect_url(
    current_url: &Url,
    response: &HyperResponse<Incoming>,
) -> TransportResult<Url> {
    let location = check_try!(
        response
            .headers()
            .get(LOCATION)
            .ok_or_else(|| format!("redirect from {current_url} omitted the Location header"))
    );
    let location_text = check_try!(
        location
            .to_str()
            .map_err(|error| format!("redirect from {current_url} has invalid Location: {error}"))
    );
    let joined = check_try!(
        current_url
            .join(location_text)
            .map_err(|error| format!("resolve redirect from {current_url}: {error}"))
    );
    let normalized = check_try!(normalize_url(joined));
    check_try!(validate_redirect_transition(current_url, &normalized));
    return Ok(normalized);
}

/// Determine the next safe redirect, if the response is terminal return none.
///
/// # Errors
///
/// Returns an error when the redirect limit is reached or Location is unsafe.
pub(super) fn next_redirect(
    current_url: &Url,
    response: &HyperResponse<Incoming>,
    followed_redirects: u8,
    redirect_limit: u8,
) -> RedirectResult {
    if !is_redirect(response.status()) {
        return Ok(None);
    }
    if followed_redirects >= redirect_limit {
        return Err(format!(
            "redirect from {current_url} exceeds the {redirect_limit}-redirect limit"
        ));
    }
    return joined_redirect_url(current_url, response).map(Some);
}

/// Strip URL fragments, which never belong in an HTTP request target.
///
/// # Errors
///
/// Returns an error when the normalized URL violates transport policy.
fn normalize_url(mut url: Url) -> TransportResult<Url> {
    url.set_fragment(None);
    check_try!(validate_request_url(&url));
    return Ok(url);
}

/// Parse and normalize one absolute request URL.
///
/// # Errors
///
/// Returns an error when the URL is malformed or violates transport policy.
pub(super) fn parse_request_url(source: &str) -> TransportResult<Url> {
    let parsed = check_try!(
        Url::parse(source).map_err(|error| format!("parse HTTP request URL {source}: {error}"))
    );
    return normalize_url(parsed);
}

/// Record one URL and reject redirect cycles before another request is sent.
///
/// # Errors
///
/// Returns an error when the URL has already been visited.
pub(super) fn record_visit(visited: &mut BTreeSet<String>, url: &Url) -> TransportResult {
    if !visited.insert(url.as_str().to_owned()) {
        return Err(format!("HTTP redirect cycle reached {url}"));
    }
    return Ok(());
}

/// Retain only representation headers when a redirect crosses an origin.
pub(super) fn redirect_headers(
    initial_url: &Url,
    current_url: &Url,
    headers: &HeaderMap,
) -> HeaderMap {
    if initial_url.origin() == current_url.origin() {
        return headers.clone();
    }
    let mut retained = HeaderMap::new();
    for name in [ACCEPT, ACCEPT_ENCODING, USER_AGENT] {
        if let Some(value) = headers.get(&name)
            && retained.insert(name, value.clone()).is_some()
        {
            return HeaderMap::new();
        }
    }
    return retained;
}

/// Reject redirect transitions that downgrade TLS or enter a loopback trust zone.
///
/// # Errors
///
/// Returns an error for HTTPS-to-HTTP transitions or public-to-loopback pivots.
pub(super) fn validate_redirect_transition(current: &Url, next: &Url) -> TransportResult {
    if current.scheme() == "https" && next.scheme() == "http" {
        return Err(format!(
            "HTTP redirect must not downgrade HTTPS: {current} -> {next}"
        ));
    }
    let current_is_loopback = current
        .host_str()
        .is_some_and(|host| return is_loopback_host(host));
    let next_is_loopback = next
        .host_str()
        .is_some_and(|host| return is_loopback_host(host));
    if !current_is_loopback && next_is_loopback {
        return Err(format!(
            "HTTP redirect must not enter a loopback trust zone: {current} -> {next}"
        ));
    }
    return Ok(());
}

/// Reject credentials and non-HTTPS non-loopback request URLs.
///
/// # Errors
///
/// Returns an error when a URL lacks a host, embeds credentials, or permits unsafe plaintext.
fn validate_request_url(url: &Url) -> TransportResult {
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!(
            "HTTP request URL must not contain credentials: {url}"
        ));
    }
    let host = check_try!(
        url.host_str()
            .ok_or_else(|| format!("HTTP request URL must include a host: {url}"))
    );
    if url.scheme() == "https" {
        return Ok(());
    }
    if url.scheme() == "http" && is_loopback_host(host) {
        return Ok(());
    }
    return Err(format!(
        "HTTP request URL must use HTTPS or loopback plaintext: {url}"
    ));
}
