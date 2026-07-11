//! Shared HTTP transport policy verification.

use super::{Client, Response, TransportResult};

use super::config::{build_headers, parse_user_agent, validate_configuration};

use super::redirect::{
    MAXIMUM_REDIRECT_LIMIT, parse_request_url, redirect_headers, validate_redirect_transition,
};

use super::response::append_body_chunk;

use core::time::Duration;

use http::{
    StatusCode,
    header::{ACCEPT, AUTHORIZATION, COOKIE, USER_AGENT},
};

use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    thread,
};

/// Compile-time references preserve named loopback fixture helpers.
const _: [usize; 0x07] = [
    size_of_val(&Client::build),
    size_of_val(&Client::get),
    size_of_val(&Response::body),
    size_of_val(&Response::content_length),
    size_of_val(&Response::status),
    size_of_val(&join_loopback_server),
    size_of_val(&serve_loopback_response),
];

/// Loopback fixture thread result.
type ServerHandle = thread::JoinHandle<TransportResult>;

/// Verify the only accepted plaintext URLs are explicit loopback endpoints.
///
/// # Errors
///
/// Returns an error when a safe loopback URL is rejected or an external
/// plaintext URL is accepted.
#[test]
fn accepts_only_loopback_plaintext() -> Result<(), String> {
    for accepted in [
        "http://localhost:8080/path#fragment",
        "http://127.0.0.1:8080/path",
        "http://[::1]:8080/path",
    ] {
        let parsed = check_try!(parse_request_url(accepted));
        if parsed.fragment().is_some() {
            return Err(format!("request fragment was not removed from {accepted}"));
        }
    }
    if parse_request_url("http://example.com/path").is_ok() {
        return Err("external plaintext HTTP URL was accepted".to_owned());
    }
    return Ok(());
}

/// Verify body accumulation cannot cross its caller-provided hard ceiling.
///
/// # Errors
///
/// Returns an error when a valid body is changed or an oversized chunk is
/// accepted.
#[test]
fn enforces_streamed_body_limit() -> Result<(), String> {
    let mut body = Vec::new();
    check_try!(append_body_chunk(&mut body, b"public", 0x06));
    if body.as_slice() != b"public" {
        return Err("bounded HTTP body changed during accumulation".to_owned());
    }
    if append_body_chunk(&mut body, b"!", 0x06).is_ok() {
        return Err("oversized HTTP body chunk was accepted".to_owned());
    }
    return Ok(());
}

/// Verify the transport can reach a bounded plaintext loopback test server.
///
/// # Errors
///
/// Returns an error when the client, server, status, length, or body contract
/// fails.
#[test]
fn fetches_bounded_loopback_response() -> Result<(), String> {
    let client = check_try!(Client::build(
        Duration::from_secs(0x02),
        Duration::from_secs(0x02),
        0x00,
        "public-check",
    ));
    let listener = check_try!(
        TcpListener::bind("127.0.0.1:0")
            .map_err(|error| format!("bind loopback HTTP fixture: {error}"))
    );
    let address = check_try!(
        listener
            .local_addr()
            .map_err(|error| format!("read loopback HTTP fixture address: {error}"))
    );
    let server = thread::spawn(move || return serve_loopback_response(&listener));
    let response = check_try!(Client::get(
        &client,
        format!("http://{address}/").as_str(),
        &[],
        0x06,
    ));
    check_try!(join_loopback_server(server));
    if Response::status(&response) != StatusCode::OK
        || Response::content_length(&response) != Some(0x06)
        || Response::body(&response) != b"public"
    {
        return Err(format!("unexpected loopback HTTP response: {response:?}"));
    }
    return Ok(());
}

/// Join the loopback fixture without hiding a server failure.
///
/// # Errors
///
/// Returns an error when the fixture reports a failure or its thread aborts.
fn join_loopback_server(server: ServerHandle) -> TransportResult {
    return match server.join() {
        Ok(result) => result,
        Err(thread_failure) => {
            drop(thread_failure);
            Err("loopback HTTP fixture thread failed".to_owned())
        }
    };
}

/// Verify impossible deadlines and excessive redirects fail client policy.
///
/// # Errors
///
/// Returns an error when an invalid client configuration is accepted.
#[test]
fn rejects_unbounded_configuration() -> Result<(), String> {
    let valid_duration = Duration::from_secs(0x01);
    if validate_configuration(Duration::ZERO, valid_duration, 0x00, "public-check").is_ok() {
        return Err("zero HTTP connection timeout was accepted".to_owned());
    }
    let excessive_redirects = check_try!(
        MAXIMUM_REDIRECT_LIMIT
            .checked_add(0x01)
            .ok_or_else(|| return "redirect limit test overflow".to_owned())
    );
    if validate_configuration(
        valid_duration,
        valid_duration,
        excessive_redirects,
        "public-check",
    )
    .is_ok()
    {
        return Err("excessive HTTP redirect limit was accepted".to_owned());
    }
    return Ok(());
}

/// Verify redirects cannot downgrade TLS or pivot a public origin into loopback.
///
/// # Errors
///
/// Returns an error when an unsafe transition is admitted or safe HTTPS remains blocked.
#[test]
fn rejects_unsafe_redirect_transitions() -> Result<(), String> {
    let public = check_try!(parse_request_url("https://example.com/source"));
    let public_target = check_try!(parse_request_url("https://download.example.net/asset"));
    let plaintext_loopback = check_try!(parse_request_url("http://127.0.0.1:8080/private"));
    let secure_loopback = check_try!(parse_request_url("https://localhost/private"));
    if validate_redirect_transition(&public, &plaintext_loopback).is_ok() {
        return Err("HTTPS redirect downgraded into plaintext loopback".to_owned());
    }
    if validate_redirect_transition(&public, &secure_loopback).is_ok() {
        return Err("public redirect entered secure loopback".to_owned());
    }
    check_try!(validate_redirect_transition(&public, &public_target));
    return Ok(());
}

/// Serve one fixed loopback response and then close the connection.
///
/// # Errors
///
/// Returns an error when accepting, reading, writing, or flushing fails.
fn serve_loopback_response(listener: &TcpListener) -> TransportResult {
    let (mut stream, peer) = check_try!(
        listener
            .accept()
            .map_err(|error| format!("accept loopback HTTP request: {error}"))
    );
    if !peer.ip().is_loopback() {
        return Err(format!(
            "loopback HTTP fixture accepted non-loopback peer {peer}"
        ));
    }
    let mut request: [u8; 0x0400] = [0x00; 0x0400];
    let read_count = check_try!(
        stream
            .read(&mut request)
            .map_err(|error| format!("read loopback HTTP request: {error}"))
    );
    if read_count == 0x00 {
        return Err("loopback HTTP fixture received an empty request".to_owned());
    }
    check_try!(
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\npublic")
            .map_err(|error| format!("write loopback HTTP response: {error}"))
    );
    return stream
        .flush()
        .map_err(|error| format!("flush loopback HTTP response: {error}"));
}

/// Verify cross-origin redirects retain only representation-safe headers.
///
/// # Errors
///
/// Returns an error when credentials cross origins or safe headers disappear.
#[test]
fn strips_sensitive_cross_origin_headers() -> Result<(), String> {
    let user_agent = check_try!(parse_user_agent("public-check"));
    let headers = check_try!(build_headers(
        &[
            (ACCEPT.as_str(), "text/plain"),
            (AUTHORIZATION.as_str(), "Bearer secret"),
            (COOKIE.as_str(), "private=value"),
        ],
        &user_agent,
    ));
    let initial_url = check_try!(parse_request_url("https://example.com/source"));
    let redirected_url = check_try!(parse_request_url("https://download.example.net/asset"));
    let retained = redirect_headers(&initial_url, &redirected_url, &headers);
    if retained.get(AUTHORIZATION).is_some() || retained.get(COOKIE).is_some() {
        return Err("cross-origin redirect retained credential headers".to_owned());
    }
    if retained.get(ACCEPT).is_none() || retained.get(USER_AGENT).is_none() {
        return Err("cross-origin redirect removed safe representation headers".to_owned());
    }
    return Ok(());
}
