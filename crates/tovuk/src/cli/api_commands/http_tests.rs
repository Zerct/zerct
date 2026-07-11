#[path = "http_tests/server.rs"]
/// Loopback HTTP test fixtures.
mod server;

use core::error::Error;
use std::net::TcpListener;

use http_body_util::Full;
use hyper::{
    Method, Request, StatusCode, Uri,
    body::{Bytes, Frame},
};

use serde_json::Value;

use crate::cli::args::options_with_api_url_for_test;

use super::{
    ApiRequestContent, ClientConfiguration, ExecuteExchange as _,
    MAX_JSON_RESPONSE_READ_BYTES_USIZE, OutputFormat, ResponseBodyLimit, ResponseData,
    ResponseText, Result as CliResult, RuntimeConfiguration, TimedExchange, TransportClient,
    TransportRuntime, UntimedExchange, ValidatedUri, api_request,
};

use super::url_policy::{BodyPresence, LoopbackHost, redirect_transition_is_allowed};

use server::{join_server, serve_once, serve_redirect};

/// Result returned by HTTP transport tests and their helpers.
type TestResult<Value = ()> = Result<Value, Box<dyn Error>>;

#[test]
/// Verifies transport failures direct automation to public status documentation.
///
/// # Errors
///
/// Returns an error when test setup fails or the error payload violates the contract.
fn api_unreachable_points_agents_to_status_docs() -> TestResult {
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

/// Converts an internal CLI result into the transport test error type.
///
/// # Errors
///
/// Returns the stable CLI error message when the operation failed.
fn cli_value<Value>(value: CliResult<Value>) -> TestResult<Value> {
    return match value {
        Ok(result) => Ok(result),
        Err(error) => Err(error.message().to_owned().into()),
    };
}

#[test]
/// Verifies declared oversized responses fail before body buffering.
///
/// # Errors
///
/// Returns an error when test setup fails or the response-size guard is not enforced.
fn declared_oversized_response_is_rejected_before_buffering() -> TestResult {
    let (api_url, server) = result_or_return!(serve_once(
        "200 OK",
        "",
        Some(MAX_JSON_RESPONSE_READ_BYTES_USIZE),
    ))
    .into_parts();
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

#[test]
/// Verifies the deadline-wrapped transport returns the complete response.
///
/// # Errors
///
/// Returns an error when setup, request construction, transport, or response validation fails.
fn direct_transport_honors_deadline() -> TestResult {
    let (api_url, server) = result_or_return!(serve_once("200 OK", "{}", None)).into_parts();
    let request = result_or_return!(
        Request::builder()
            .method(Method::GET)
            .uri(api_url)
            .body(Full::new(Bytes::new()))
            .map_err(|error| return Box::<dyn Error>::from(error))
    );
    let client =
        result_or_return!(cli_value(TransportClient::try_from(ClientConfiguration))).into_inner();
    let runtime = result_or_return!(cli_value(TransportRuntime::try_from(RuntimeConfiguration,)))
        .into_inner();
    let response = result_or_return!(cli_value(
        runtime.block_on(
            TimedExchange {
                client,
                output_format: OutputFormat::Text,
                request,
            }
            .execute()
        )
    ));
    result_or_return!(join_server(server));
    if response.status != StatusCode::OK || response.body.0 != "{}" {
        return Err("deadline-wrapped transport returned an unexpected response".into());
    }
    return Ok(());
}

#[test]
/// Verifies the underlying transport buffers a bounded response.
///
/// # Errors
///
/// Returns an error when setup, request construction, transport, or response validation fails.
fn direct_transport_without_deadline_buffers_response() -> TestResult {
    let (api_url, server) = result_or_return!(serve_once("200 OK", "[]", None)).into_parts();
    let request = result_or_return!(
        Request::builder()
            .method(Method::GET)
            .uri(api_url)
            .body(Full::new(Bytes::new()))
            .map_err(|error| return Box::<dyn Error>::from(error))
    );
    let client =
        result_or_return!(cli_value(TransportClient::try_from(ClientConfiguration))).into_inner();
    let runtime = result_or_return!(cli_value(TransportRuntime::try_from(RuntimeConfiguration,)))
        .into_inner();
    let response = result_or_return!(cli_value(
        runtime.block_on(
            UntimedExchange {
                client,
                output_format: OutputFormat::Text,
                request,
            }
            .execute()
        )
    ));
    result_or_return!(join_server(server));
    if response.status != StatusCode::OK || response.body.0 != "[]" {
        return Err("underlying transport returned an unexpected response".into());
    }
    return Ok(());
}

#[test]
/// Verifies malformed error bodies retain their HTTP status diagnostic.
///
/// # Errors
///
/// Returns an error when test setup fails or the fallback payload violates the contract.
fn malformed_error_response_still_reports_http_status() -> TestResult {
    let (api_url, server) = result_or_return!(serve_once(
        "503 Service Unavailable",
        "<html>down</html>",
        None,
    ))
    .into_parts();
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

#[test]
/// Verifies plaintext transport is restricted to literal loopback hosts.
///
/// # Errors
///
/// Returns an error when URI parsing fails or transport policy accepts an unsafe endpoint.
fn plaintext_transport_is_limited_to_loopback() -> TestResult {
    let _secure = result_or_return!(cli_value(ValidatedUri::try_from((
        "https://example.test",
        "/v1/status",
    ))));
    let _local = result_or_return!(cli_value(ValidatedUri::try_from((
        "http://127.0.0.1:8080",
        "/v1/status",
    ))));
    if LoopbackHost::try_from("127.0.0.1").is_err()
        || LoopbackHost::try_from("example.test").is_ok()
    {
        return Err("literal loopback host classification failed".into());
    }
    if ValidatedUri::try_from(("http://example.test", "/v1/status")).is_ok() {
        return Err("non-loopback plaintext host was accepted".into());
    }
    return Ok(());
}

#[test]
/// Verifies redirects cannot downgrade, enter loopback, or replay bodies across origins.
///
/// # Errors
///
/// Returns an error when a prohibited transition is admitted or a safe one is blocked.
fn redirect_policy_rejects_trust_boundary_crossings() -> TestResult {
    let public = Uri::from_static("https://example.test/source");
    let public_target = Uri::from_static("https://download.example.test/target");
    let plaintext_loopback = Uri::from_static("http://127.0.0.1:8080/private");
    let secure_loopback = Uri::from_static("https://localhost/private");
    if redirect_transition_is_allowed(
        &public,
        &plaintext_loopback,
        &Method::GET,
        BodyPresence::Empty,
    ) {
        return Err("HTTPS redirect downgraded into plaintext loopback".into());
    }
    if redirect_transition_is_allowed(&public, &secure_loopback, &Method::GET, BodyPresence::Empty)
    {
        return Err("public redirect entered secure loopback".into());
    }
    if redirect_transition_is_allowed(
        &public,
        &public_target,
        &Method::POST,
        BodyPresence::Present,
    ) {
        return Err("cross-origin redirect retained a request body method".into());
    }
    if redirect_transition_is_allowed(&public, &public_target, &Method::GET, BodyPresence::Present)
    {
        return Err("cross-origin redirect retained a GET request body".into());
    }
    if !redirect_transition_is_allowed(&public, &public_target, &Method::GET, BodyPresence::Empty)
        || !redirect_transition_is_allowed(
            &public,
            &Uri::from_static("https://example.test/final"),
            &Method::POST,
            BodyPresence::Present,
        )
    {
        return Err("safe redirect transition was blocked".into());
    }
    return Ok(());
}

#[test]
/// Verifies a 307 response cannot replay GET or POST JSON bodies across origins.
///
/// # Errors
///
/// Returns an error when setup fails or the transport follows the unsafe redirect.
fn redirect_replay_across_origins_is_rejected() -> TestResult {
    for method in [Method::GET, Method::POST] {
        let (api_url, server) = result_or_return!(serve_redirect(
            "307 Temporary Redirect",
            "https://example.test/final",
            None,
        ))
        .into_parts();
        let cli = options_with_api_url_for_test(api_url);
        let error = match api_request(
            &cli,
            method,
            "/v1/status",
            ApiRequestContent::Authenticated {
                body: Some(Value::Null),
                token: "test-token".to_owned(),
            },
        ) {
            Ok(_payload) => return Err("cross-origin 307 replay unexpectedly succeeded".into()),
            Err(error) => error,
        };
        result_or_return!(join_server(server));
        if error.payload().message() != "Tovuk API returned HTTP 307." {
            return Err(format!("unexpected message: {}", error.message()).into());
        }
    }
    return Ok(());
}

#[test]
/// Verifies a safe relative redirect reaches its final JSON response.
///
/// # Errors
///
/// Returns an error when setup fails or the redirect is not followed exactly once.
fn redirect_response_is_followed() -> TestResult {
    let (api_url, server) =
        result_or_return!(serve_redirect("302 Found", "/final", Some("{}"))).into_parts();
    let cli = options_with_api_url_for_test(api_url);
    let payload = match api_request(
        &cli,
        Method::GET,
        "/v1/status",
        ApiRequestContent::Anonymous,
    ) {
        Ok(payload) => payload,
        Err(error) => return Err(error.message().to_owned().into()),
    };
    result_or_return!(join_server(server));
    if !payload
        .as_object()
        .is_some_and(|object| return object.is_empty())
    {
        return Err("redirect did not return the final JSON body".into());
    }
    return Ok(());
}

#[test]
/// Verifies redirects cannot escape to a public plaintext endpoint.
///
/// # Errors
///
/// Returns an error when setup fails or the unsafe destination is followed.
fn redirect_to_public_plaintext_is_rejected() -> TestResult {
    let (api_url, server) = result_or_return!(serve_redirect(
        "302 Found",
        "http://example.test/final",
        None,
    ))
    .into_parts();
    let cli = options_with_api_url_for_test(api_url);
    let error = match api_request(
        &cli,
        Method::GET,
        "/v1/status",
        ApiRequestContent::Anonymous,
    ) {
        Ok(_payload) => return Err("unsafe redirect unexpectedly succeeded".into()),
        Err(error) => error,
    };
    result_or_return!(join_server(server));
    if error.payload().message() != "Tovuk API returned HTTP 302." {
        return Err(format!("unexpected message: {}", error.message()).into());
    }
    return Ok(());
}

#[test]
/// Verifies streaming accepts a response exactly at its configured limit.
///
/// # Errors
///
/// Returns an error when exact-limit input is rejected or decoded incorrectly.
fn streamed_response_accepts_exact_limit() -> TestResult {
    let ResponseData(frame) = result_or_return!(cli_value(ResponseData::try_from(Ok(
        Frame::data(Bytes::from_static(b"frame")),
    ))));
    if frame.as_ref() != b"frame" {
        return Err("response frame data was not preserved".into());
    }
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
    let (api_url, server) = result_or_return!(serve_once("200 OK", "not-json", None)).into_parts();
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
