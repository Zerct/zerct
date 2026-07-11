use super::{ResponseConstraints, bounded_response_text};

use http::StatusCode;

/// Verify declared and chunked public docs bodies obey the hard ceiling.
///
/// # Panics
///
/// Panics when bounded response enforcement accepts an oversized body.
#[test]
fn enforces_declared_and_streamed_response_limits() {
    let mut declared_reader: &[u8] = b"small";
    let declared_error = bounded_response_text(
        &mut declared_reader,
        &ResponseConstraints {
            content_length: Some(0x0009),
            maximum: 0x0008,
            path: "/declared".to_owned(),
            status: StatusCode::OK,
        },
    );
    assert!(
        declared_error.is_err(),
        "an oversized Content-Length must fail before body acceptance"
    );

    let mut chunked_reader: &[u8] = b"123456789";
    let chunked_error = bounded_response_text(
        &mut chunked_reader,
        &ResponseConstraints {
            content_length: None,
            maximum: 0x0008,
            path: "/chunked".to_owned(),
            status: StatusCode::OK,
        },
    );
    assert!(
        chunked_error.is_err(),
        "an oversized chunked body must fail at the streaming ceiling"
    );
}

/// Verify a bounded public docs body is returned unchanged.
///
/// # Panics
///
/// Panics when a valid bounded response cannot be decoded.
#[test]
fn returns_bounded_utf8_body() {
    let mut reader: &[u8] = b"public docs";
    let result = bounded_response_text(
        &mut reader,
        &ResponseConstraints {
            content_length: Some(0x000b),
            maximum: 0x0010,
            path: "/docs".to_owned(),
            status: StatusCode::OK,
        },
    );
    assert_eq!(
        result.map_err(|error| return error.message),
        Ok("public docs".to_owned()),
        "a bounded UTF-8 response must be preserved"
    );
}
