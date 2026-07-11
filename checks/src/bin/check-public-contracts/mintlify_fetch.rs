use core::time::Duration;

use crate::helpers::{CheckResult, env_int};

use http::StatusCode;

use std::{io::Read, thread::sleep};

use tovuk_public_checks::http_transport::Client;

/// Largest accepted public documentation response.
const MAX_PUBLIC_DOC_BYTES: usize = 0x800_000;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x0009] = [
    size_of_val(&FetchContext::new),
    size_of_val(&bounded_validate_declared_length),
    size_of_val(&bounded_validate_read_body),
    size_of_val(&bounded_response_text),
    size_of_val(&fetch_text_once),
    size_of_val(&is_retryable_fetch_error),
    size_of_val(&normalize_target_url),
    size_of_val(&request_text),
    size_of_val(&retry_delay),
];

/// Shared bounded HTTP client and retry configuration for public docs checks.
#[derive(Debug)]
pub(super) struct FetchContext {
    /// Normalized public docs base URL.
    base_url: String,
    /// Bounded blocking HTTP client.
    client: Client,
    /// Maximum number of retry attempts.
    retries: i64,
    /// Delay between retryable requests.
    retry_delay: Duration,
}

impl FetchContext {
    /// Return the normalized public docs base URL.
    #[inline]
    pub(super) const fn base_url(&self) -> &str {
        return self.base_url.as_str();
    }

    /// Construct a bounded public documentation fetch context.
    #[inline]
    pub(super) const fn new(
        base_url: String,
        client: Client,
        retries: i64,
        retry_delay: Duration,
    ) -> Self {
        return Self {
            base_url,
            client,
            retries,
            retry_delay,
        };
    }

    /// Return the configured retry count.
    #[inline]
    pub(super) const fn retries(&self) -> i64 {
        return self.retries;
    }

    /// Return the delay between retryable requests.
    #[inline]
    pub(super) const fn retry_delay(&self) -> Duration {
        return self.retry_delay;
    }
}

#[derive(Debug)]
/// Contract representation for `FetchError`.
pub(super) struct FetchError {
    /// Contract data stored in `message`.
    message: String,
    /// Contract data stored in `status`.
    status: Option<StatusCode>,
}

/// Result returned by bounded public documentation fetch helpers.
type FetchResult<T> = Result<T, FetchError>;

/// HTTP request headers used by Mintlify checks.
pub(super) type RequestHeaders = [(&'static str, &'static str)];

/// Limits and diagnostics associated with one response body.
#[derive(Debug)]
struct ResponseConstraints {
    /// Declared response size when supplied by the server.
    content_length: Option<u64>,
    /// Maximum accepted body size.
    maximum: usize,
    /// Public endpoint path used in diagnostics.
    path: String,
    /// HTTP response status.
    status: StatusCode,
}

/// Read one response through a hard byte ceiling, including chunked bodies.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
fn bounded_response_text(
    response: &mut dyn Read,
    constraints: &ResponseConstraints,
) -> FetchResult<String> {
    check_try!(bounded_validate_declared_length(constraints));
    let bytes = check_try!(bounded_validate_read_body(response, constraints));
    return String::from_utf8(bytes).map_err(|error| {
        return FetchError {
            message: format!("{} is not UTF-8: {error}", constraints.path),
            status: Some(constraints.status),
        };
    });
}

/// Reject a declared public response body above the configured ceiling.
///
/// # Errors
///
/// Returns an error when the declared response exceeds the ceiling.
fn bounded_validate_declared_length(constraints: &ResponseConstraints) -> FetchResult<()> {
    let maximum = check_try!(u64::try_from(constraints.maximum).map_err(|error| {
        return FetchError {
            message: format!("convert public docs response limit: {error}"),
            status: Some(constraints.status),
        };
    }));
    if constraints
        .content_length
        .is_some_and(|length| return length > maximum)
    {
        return Err(bounded_validate_response_limit_error(constraints));
    }
    return Ok(());
}

/// Read a public response body through the configured hard ceiling.
///
/// # Errors
///
/// Returns an error when the response cannot be read within the ceiling.
fn bounded_validate_read_body(
    response: &mut dyn Read,
    constraints: &ResponseConstraints,
) -> FetchResult<Vec<u8>> {
    let read_limit = check_try!(constraints.maximum.checked_add(0x0001).ok_or_else(|| {
        return FetchError {
            message: "public docs response limit overflow".to_owned(),
            status: Some(constraints.status),
        };
    }));
    let read_limit_u64 = check_try!(u64::try_from(read_limit).map_err(|error| {
        return FetchError {
            message: format!("convert public docs response limit: {error}"),
            status: Some(constraints.status),
        };
    }));
    let mut bytes = Vec::new();
    let read_count = check_try!(
        response
            .take(read_limit_u64)
            .read_to_end(&mut bytes)
            .map_err(|error| return FetchError {
                message: error.to_string(),
                status: Some(constraints.status),
            })
    );
    if read_count != bytes.len() {
        return Err(FetchError {
            message: format!("{} changed while its response was read", constraints.path),
            status: Some(constraints.status),
        });
    }
    if bytes.len() > constraints.maximum {
        return Err(bounded_validate_response_limit_error(constraints));
    }
    return Ok(bytes);
}

/// Build a consistent response-size failure.
fn bounded_validate_response_limit_error(constraints: &ResponseConstraints) -> FetchError {
    return FetchError {
        message: format!(
            "{} exceeds the {}-byte public docs limit",
            constraints.path, constraints.maximum
        ),
        status: Some(constraints.status),
    };
}

/// Contract implementation for `fetch_text`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn fetch_text(
    context: &FetchContext,
    path: &str,
    headers: &RequestHeaders,
) -> CheckResult<String> {
    let mut last_error = FetchError {
        message: "request was not attempted".to_owned(),
        status: None,
    };
    for attempt in 0..=context.retries() {
        let error = match request_text(context, path, headers) {
            Ok(text) => return Ok(text),
            Err(error) => error,
        };
        let should_retry = attempt < context.retries() && is_retryable_fetch_error(&error);
        last_error = error;
        if !should_retry {
            break;
        }
        sleep(context.retry_delay());
    }
    return Err(last_error.message);
}

/// Fetch one public documentation response without applying retries.
///
/// # Errors
///
/// Returns an error when the request fails or its response violates the bounded body contract.
pub(super) fn fetch_text_once(
    context: &FetchContext,
    path: &str,
    headers: &RequestHeaders,
) -> CheckResult<String> {
    return request_text(context, path, headers).map_err(|error| return error.message);
}

/// Contract implementation for `is_retryable_fetch_error`.
pub(super) fn is_retryable_fetch_error(error: &FetchError) -> bool {
    return error.status.is_none_or(|status| {
        return status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
    });
}

/// Contract implementation for `normalize_target_url`.
pub(super) fn normalize_target_url(target: &str) -> String {
    let with_scheme = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_owned()
    } else {
        format!("https://{target}")
    };
    return with_scheme.trim_end_matches('/').to_owned();
}

/// Contract implementation for `request_text`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn request_text(
    context: &FetchContext,
    path: &str,
    headers: &RequestHeaders,
) -> Result<String, FetchError> {
    let url = format!("{}{path}", context.base_url());
    let response = check_try!(
        context
            .client
            .get(url.as_str(), headers, MAX_PUBLIC_DOC_BYTES)
            .map_err(|error| return FetchError {
                message: error,
                status: None,
            })
    );
    let status = response.status();
    if !status.is_success() {
        return Err(FetchError {
            message: format!("{path} returned {}", status.as_u16()),
            status: Some(status),
        });
    }
    let content_length = response.content_length();
    let mut body = response.body();
    return bounded_response_text(
        &mut body,
        &ResponseConstraints {
            content_length,
            maximum: MAX_PUBLIC_DOC_BYTES,
            path: path.to_owned(),
            status,
        },
    );
}

/// Contract implementation for `retry_delay`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn retry_delay() -> CheckResult<Duration> {
    let retry_delay_ms = check_try!(env_int("TOVUK_DOCS_CHECK_RETRY_DELAY_MS", 5_000));
    return Ok(Duration::from_millis(check_try!(
        u64::try_from(retry_delay_ms).map_err(|error| {
            return format!("TOVUK_DOCS_CHECK_RETRY_DELAY_MS must be non-negative: {error}");
        })
    )));
}
#[cfg(test)]
#[path = "mintlify_fetch_tests/verification.rs"]
mod tests;
