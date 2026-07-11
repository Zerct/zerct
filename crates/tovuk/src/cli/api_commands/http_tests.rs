use core::error::Error;
use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    thread::{self, JoinHandle},
};

use reqwest::Method;
use rustls::crypto::{CryptoProvider, aws_lc_rs::default_provider};

use crate::cli::args::options_with_api_url_for_test;

use super::{
    ApiRequestContent, MAX_JSON_RESPONSE_READ_BYTES_USIZE, ResponseBodyLimit, ResponseText,
    api_request,
};

/// Result returned by HTTP transport tests and their helpers.
type TestResult<Value = ()> = Result<Value, Box<dyn Error>>;
/// One-shot HTTP server handle used by transport tests.
type TestServer = JoinHandle<Result<(), String>>;

#[derive(Debug)]
/// Address and thread handle for a started one-shot test server.
struct TestServerStart {
    /// Local API base URL served by the listener.
    api_url: String,
    /// Thread serving the single HTTP exchange.
    server: TestServer,
}

#[test]
/// Verifies transport failures direct automation to public status documentation.
///
/// # Errors
///
/// Returns an error when test setup fails or the error payload violates the contract.
fn api_unreachable_points_agents_to_status_docs() -> TestResult {
    result_or_return!(ensure_test_crypto());
    let listener = result_or_return!(
        TcpListener::bind("127.0.0.1:0").map_err(|error| return Box::<dyn Error>::from(error))
    );
    let address = result_or_return!(
        listener
            .local_addr()
            .map_err(|error| return Box::<dyn Error>::from(error))
    );
    let api_url = format!("http://{address}");
    drop(listener);
    let cli = options_with_api_url_for_test(api_url);

    let error = match api_request(
        &cli,
        Method::GET,
        "/v1/status",
        ApiRequestContent::Anonymous,
    ) {
        Ok(_response) => return Err("request unexpectedly succeeded".into()),
        Err(error) => error,
    };
    let payload = error.payload();

    if payload.code() != "api_unreachable" {
        return Err(format!("unexpected code: {}", payload.code()).into());
    }
    if payload.docs_url() != Some("https://docs.tovuk.com/status") {
        return Err(format!("unexpected docs url: {:?}", payload.docs_url()).into());
    }
    return Ok(());
}

#[test]
/// Verifies declared oversized responses fail before body buffering.
///
/// # Errors
///
/// Returns an error when test setup fails or the response-size guard is not enforced.
fn declared_oversized_response_is_rejected_before_buffering() -> TestResult {
    result_or_return!(ensure_test_crypto());
    let TestServerStart { api_url, server } = result_or_return!(serve_once(
        "200 OK",
        "",
        Some(MAX_JSON_RESPONSE_READ_BYTES_USIZE),
    ));
    let cli = options_with_api_url_for_test(api_url);

    let error = match api_request(
        &cli,
        Method::GET,
        "/v1/status",
        ApiRequestContent::Anonymous,
    ) {
        Ok(_response) => return Err("oversized response unexpectedly succeeded".into()),
        Err(error) => error,
    };
    result_or_return!(join_server(server));
    if !error.message().contains("100 MiB JSON response limit") {
        return Err(format!("unexpected message: {}", error.message()).into());
    }
    return Ok(());
}

/// Installs a deterministic TLS cryptography provider for concurrent tests.
///
/// # Errors
///
/// Returns an error when no provider can be installed or observed.
fn ensure_test_crypto() -> TestResult {
    if CryptoProvider::get_default().is_none() {
        let installation = default_provider().install_default();
        if installation.is_err() && CryptoProvider::get_default().is_none() {
            return Err("test TLS cryptography provider could not be initialized".into());
        }
    }
    return Ok(());
}

/// Waits for a one-shot HTTP test server to finish.
///
/// # Errors
///
/// Returns an error when the server reports an I/O failure or its thread panics.
fn join_server(server: TestServer) -> TestResult {
    match server.join() {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(error)) => return Err(error.into()),
        Err(_) => return Err("test server panicked".into()),
    }
}

#[test]
/// Verifies malformed error bodies retain their HTTP status diagnostic.
///
/// # Errors
///
/// Returns an error when test setup fails or the fallback payload violates the contract.
fn malformed_error_response_still_reports_http_status() -> TestResult {
    result_or_return!(ensure_test_crypto());
    let TestServerStart { api_url, server } = result_or_return!(serve_once(
        "503 Service Unavailable",
        "<html>down</html>",
        None,
    ));
    let cli = options_with_api_url_for_test(api_url);

    let error = match api_request(
        &cli,
        Method::GET,
        "/v1/status",
        ApiRequestContent::Anonymous,
    ) {
        Ok(_response) => return Err("request unexpectedly succeeded".into()),
        Err(error) => error,
    };
    result_or_return!(join_server(server));
    let payload = error.payload();

    if payload.code() != "api_error" {
        return Err(format!("unexpected code: {}", payload.code()).into());
    }
    if payload.message() != "Tovuk API returned HTTP 503." {
        return Err(format!("unexpected message: {}", payload.message()).into());
    }
    return Ok(());
}

/// Starts a one-shot local HTTP server with an optional declared body length.
///
/// # Errors
///
/// Returns an error when the listener cannot bind or report its local address.
fn serve_once(
    status: &str,
    body: &str,
    declared_length: Option<usize>,
) -> TestResult<TestServerStart> {
    let listener = result_or_return!(
        TcpListener::bind("127.0.0.1:0").map_err(|error| return Box::<dyn Error>::from(error))
    );
    let address = result_or_return!(
        listener
            .local_addr()
            .map_err(|error| return Box::<dyn Error>::from(error))
    );
    let api_url = format!("http://{address}");
    let owned_status = status.to_owned();
    let owned_body = body.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) =
            result_or_return!(listener.accept().map_err(|error| return error.to_string()));
        let mut request: [u8; 0x0400] = [0; 0x0400];
        let _request_size = result_or_return!(
            stream
                .read(&mut request)
                .map_err(|error| return error.to_string())
        );
        let content_length = declared_length.unwrap_or(owned_body.len());
        let response = format!(
            "HTTP/1.1 {owned_status}\r\ncontent-type: application/json\r\ncontent-length: {content_length}\r\nconnection: close\r\n\r\n{owned_body}",
        );
        result_or_return!(
            stream
                .write_all(response.as_bytes())
                .map_err(|error| return error.to_string())
        );
        return Ok(());
    });
    return Ok(TestServerStart { api_url, server });
}

#[test]
/// Verifies streaming accepts a response exactly at its configured limit.
///
/// # Errors
///
/// Returns an error when exact-limit input is rejected or decoded incorrectly.
fn streamed_response_accepts_exact_limit() -> TestResult {
    let limit = ResponseBodyLimit {
        maximum: 0b100,
        read_ceiling: 0b101,
    };
    let response = match ResponseText::try_from((b"four".as_slice(), limit)) {
        Ok(response) => response,
        Err(error) => return Err(error.message().to_owned().into()),
    };
    if response.0 != "four" {
        return Err(format!("unexpected response text: {}", response.0).into());
    }
    return Ok(());
}

#[test]
/// Verifies streaming rejects a response beyond its configured limit.
///
/// # Errors
///
/// Returns an error when oversized input is accepted or reports a different error.
fn streamed_response_rejects_oversized_body() -> TestResult {
    let limit = ResponseBodyLimit {
        maximum: 0b100,
        read_ceiling: 0b101,
    };
    let error = match ResponseText::try_from((b"three".as_slice(), limit)) {
        Ok(_response) => return Err("oversized response unexpectedly succeeded".into()),
        Err(error) => error,
    };
    if !error.message().contains("100 MiB JSON response limit") {
        return Err(format!("unexpected message: {}", error.message()).into());
    }
    return Ok(());
}

#[test]
/// Verifies successful responses require valid JSON.
///
/// # Errors
///
/// Returns an error when test setup fails or invalid JSON is accepted.
fn successful_api_response_must_be_valid_json() -> TestResult {
    result_or_return!(ensure_test_crypto());
    let TestServerStart { api_url, server } =
        result_or_return!(serve_once("200 OK", "not-json", None));
    let cli = options_with_api_url_for_test(api_url);

    let error = match api_request(
        &cli,
        Method::GET,
        "/v1/status",
        ApiRequestContent::Anonymous,
    ) {
        Ok(_response) => return Err("request unexpectedly succeeded".into()),
        Err(error) => error,
    };
    result_or_return!(join_server(server));
    let payload = error.payload();

    if payload.code() != "internal_error" {
        return Err(format!("unexpected code: {}", payload.code()).into());
    }
    if !payload.message().contains("invalid JSON") {
        return Err(format!("unexpected message: {}", payload.message()).into());
    }
    return Ok(());
}
