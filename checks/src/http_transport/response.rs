//! Bounded HTTP response collection.

use super::{LengthResult, Response, TransportResult};

use http::{HeaderMap, header::CONTENT_LENGTH};

use http_body_util::BodyExt as _;

use hyper::{Response as HyperResponse, body::Incoming};

/// Compile-time references preserve the named response boundaries.
const _: [usize; 0x05] = [
    size_of_val(&append_body_chunk),
    size_of_val(&collect_body),
    size_of_val(&collect_response),
    size_of_val(&parse_content_length),
    size_of_val(&validate_declared_length),
];

/// Append one body chunk without crossing the caller's byte ceiling.
///
/// # Errors
///
/// Returns an error when the accumulated length overflows or exceeds the ceiling.
pub(super) fn append_body_chunk(
    body: &mut Vec<u8>,
    chunk: &[u8],
    maximum: usize,
) -> TransportResult {
    let next_length = check_try!(
        body.len()
            .checked_add(chunk.len())
            .ok_or_else(|| return "HTTP response body length overflow".to_owned())
    );
    if next_length > maximum {
        return Err(format!("HTTP response exceeds the {maximum}-byte limit"));
    }
    body.extend_from_slice(chunk);
    return Ok(());
}

/// Read a complete response through the hard caller-provided byte ceiling.
///
/// # Errors
///
/// Returns an error for a body transport failure, overflow, or oversized chunk.
async fn collect_body(body: &mut Incoming, maximum: usize) -> TransportResult<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(frame_result) = body.frame().await {
        let frame = check_try!(frame_result.map_err(|error| format!("read HTTP body: {error}")));
        match frame.into_data() {
            Ok(chunk) => check_try!(append_body_chunk(&mut bytes, chunk.as_ref(), maximum)),
            Err(trailer_frame) => drop(trailer_frame),
        }
    }
    return Ok(bytes);
}

/// Validate and collect one terminal response.
///
/// # Errors
///
/// Returns an error for an invalid declared length or a body read failure.
pub(super) async fn collect_response(
    mut response: HyperResponse<Incoming>,
    maximum: usize,
) -> TransportResult<Response> {
    let content_length = check_try!(parse_content_length(response.headers()));
    check_try!(validate_declared_length(content_length, maximum));
    let status = response.status();
    let body = check_try!(collect_body(response.body_mut(), maximum).await);
    return Ok(Response {
        body,
        content_length,
        status,
    });
}

/// Parse one optional Content-Length header without accepting ambiguity.
///
/// # Errors
///
/// Returns an error when the server supplied an invalid length.
fn parse_content_length(headers: &HeaderMap) -> LengthResult {
    let Some(value) = headers.get(CONTENT_LENGTH) else {
        return Ok(None);
    };
    let text = check_try!(
        value
            .to_str()
            .map_err(|error| format!("invalid Content-Length header: {error}"))
    );
    return text
        .parse::<u64>()
        .map(Some)
        .map_err(|error| format!("invalid Content-Length value {text}: {error}"));
}

/// Reject server-declared bodies above the caller's hard ceiling.
///
/// # Errors
///
/// Returns an error when the limit cannot be represented or the body is oversized.
fn validate_declared_length(content_length: Option<u64>, maximum: usize) -> TransportResult {
    let maximum_u64 = check_try!(
        u64::try_from(maximum).map_err(|error| format!("convert HTTP response limit: {error}"))
    );
    if content_length.is_some_and(|length| return length > maximum_u64) {
        return Err(format!("HTTP response exceeds the {maximum}-byte limit"));
    }
    return Ok(());
}
