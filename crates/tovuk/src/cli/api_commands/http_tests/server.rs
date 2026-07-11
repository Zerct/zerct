//! Loopback HTTP fixtures for transport policy tests.

use super::TestResult;

use core::error::Error;

use std::{
    io::{Read as _, Write as _},
    net::{TcpListener, TcpStream},
    thread::{self, JoinHandle},
};

/// Compile-time references preserve the named fixture boundaries.
const _: [usize; 0x04] = [
    size_of_val(&join_server),
    size_of_val(&serve_once),
    size_of_val(&serve_redirect),
    size_of_val(&write_exchange),
];

/// One-shot HTTP server handle used by transport tests.
type TestServer = JoinHandle<Result<(), String>>;

#[derive(Debug)]
/// Address and thread handle for a started one-shot test server.
pub(super) struct TestServerStart {
    /// Local API base URL served by the listener.
    api_url: String,
    /// Thread serving the single HTTP exchange.
    server: TestServer,
}

impl TestServerStart {
    /// Consume the fixture and return its URL and thread handle.
    #[inline]
    pub(super) fn into_parts(self) -> (String, TestServer) {
        return (self.api_url, self.server);
    }
}

/// Wait for a one-shot HTTP test server to finish.
///
/// # Errors
///
/// Returns an error when the server reports an I/O failure or its thread panics.
pub(super) fn join_server(server: TestServer) -> TestResult {
    match server.join() {
        Ok(Ok(())) => return Ok(()),
        Ok(Err(error)) => return Err(error.into()),
        Err(_) => return Err("test server panicked".into()),
    }
}

/// Start a one-shot local HTTP server with an optional declared body length.
///
/// # Errors
///
/// Returns an error when the listener cannot bind or report its local address.
pub(super) fn serve_once(
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
        let content_length = declared_length.unwrap_or(owned_body.len());
        let response = format!(
            "HTTP/1.1 {owned_status}\r\ncontent-type: application/json\r\ncontent-length: {content_length}\r\nconnection: close\r\n\r\n{owned_body}",
        );
        return write_exchange(&mut stream, &response);
    });
    return Ok(TestServerStart { api_url, server });
}

/// Start a local server that emits one redirect and an optional final response.
///
/// # Errors
///
/// Returns an error when the listener cannot bind or report its local address.
pub(super) fn serve_redirect(
    status: &str,
    location: &str,
    final_body: Option<&str>,
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
    let owned_body = final_body.map(str::to_owned);
    let owned_location = location.to_owned();
    let owned_status = status.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) =
            result_or_return!(listener.accept().map_err(|error| return error.to_string()));
        let response = format!(
            "HTTP/1.1 {owned_status}\r\nlocation: {owned_location}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        );
        result_or_return!(write_exchange(&mut stream, &response));
        if let Some(body) = owned_body {
            let (mut final_stream, _) =
                result_or_return!(listener.accept().map_err(|error| return error.to_string()));
            let final_response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len(),
            );
            result_or_return!(write_exchange(&mut final_stream, &final_response));
        }
        return Ok(());
    });
    return Ok(TestServerStart { api_url, server });
}

/// Read one local request and write its complete test response.
///
/// # Errors
///
/// Returns an error when the local socket cannot be read or written.
fn write_exchange(stream: &mut TcpStream, response: &str) -> Result<(), String> {
    let mut request: [u8; 0x0400] = [0; 0x0400];
    let _request_size = result_or_return!(
        stream
            .read(&mut request)
            .map_err(|error| return error.to_string())
    );
    result_or_return!(
        stream
            .write_all(response.as_bytes())
            .map_err(|error| return error.to_string())
    );
    return Ok(());
}
