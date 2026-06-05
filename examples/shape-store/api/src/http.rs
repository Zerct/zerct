use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

const HEADER_DELIMITER_LENGTH: usize = 4;
const MAX_REQUEST_BYTES: usize = 64 * 1024;
const READ_CHUNK_BYTES: usize = 4096;

pub(crate) struct Request {
    pub(crate) body: String,
    pub(crate) method: String,
    pub(crate) origin: String,
    pub(crate) path: String,
}

pub(crate) struct Response {
    pub(crate) body: String,
    pub(crate) status: &'static str,
}

pub(crate) fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;

    let mut bytes = Vec::new();
    let mut buffer = [0_u8; READ_CHUNK_BYTES];

    loop {
        let size = stream.read(&mut buffer)?;
        if size == 0 {
            break;
        }

        bytes.extend_from_slice(&buffer[..size]);

        if request_is_complete(&bytes) || bytes.len() >= MAX_REQUEST_BYTES {
            break;
        }
    }

    Ok(parse_request(&bytes))
}

#[must_use]
fn request_is_complete(bytes: &[u8]) -> bool {
    let Some(header_end) = header_end(bytes) else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..header_end]);
    let expected_body_bytes = content_length(&head);
    bytes.len() >= header_end + HEADER_DELIMITER_LENGTH + expected_body_bytes
}

#[must_use]
fn parse_request(bytes: &[u8]) -> Request {
    let header_end = header_end(bytes).unwrap_or(bytes.len());
    let head = String::from_utf8_lossy(&bytes[..header_end]);
    let body_start = (header_end + HEADER_DELIMITER_LENGTH).min(bytes.len());
    let body = String::from_utf8_lossy(&bytes[body_start..]).into_owned();
    let mut request_line_parts = head.lines().next().unwrap_or_default().split_whitespace();
    let method = request_line_parts.next().unwrap_or_default().to_owned();
    let raw_path = request_line_parts.next().unwrap_or("/");
    let path = raw_path.split('?').next().unwrap_or("/").to_owned();
    let origin = header_value(&head, "Origin").unwrap_or("*").to_owned();

    Request {
        body,
        method,
        origin,
        path,
    }
}

#[must_use]
pub(crate) fn json(status: &'static str, body: &str) -> Response {
    Response {
        body: body.to_owned(),
        status,
    }
}

#[must_use]
pub(crate) fn json_value(status: &'static str, body: &serde_json::Value) -> Response {
    json(status, &body.to_string())
}

#[must_use]
fn header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(HEADER_DELIMITER_LENGTH)
        .position(|window| window == b"\r\n\r\n")
}

#[must_use]
fn content_length(head: &str) -> usize {
    header_value(head, "Content-Length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

#[must_use]
fn header_value<'a>(head: &'a str, expected_name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case(expected_name) {
            Some(value.trim())
        } else {
            None
        }
    })
}

#[must_use]
pub(crate) fn allowed_origin(request_origin: &str) -> String {
    let configured =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_error| request_origin.to_owned());
    if configured == "*" || configured == request_origin {
        configured
    } else {
        "null".to_owned()
    }
}

pub(crate) fn write_response(
    stream: &mut TcpStream,
    response: &Response,
    origin: &str,
) -> std::io::Result<()> {
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    write!(
        stream,
        "HTTP/1.1 {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\naccess-control-allow-origin: {origin}\r\naccess-control-allow-methods: GET, POST, OPTIONS\r\naccess-control-allow-headers: content-type, authorization\r\nconnection: close\r\n\r\n{}",
        response.status,
        response.body.len(),
        response.body
    )
}
