/// Validated CDN cache identity for exact documentation deployments.
#[path = "mintlify_fetch/cache_identity.rs"]
pub mod request_cache;

use core::time::Duration;

use crate::helpers::{CheckResult, env_int};

use request_cache::{Identity as DocsCacheIdentity, render_cache_path};

pub(super) use request_cache::read_identity as docs_cache_identity;

#[cfg(test)]
use request_cache::{validate_check_id, validate_revision};

use http::StatusCode;

use std::{io::Read, time::Instant};

use tovuk_public_checks::http_transport::Client;

/// Largest accepted public documentation response.
const MAX_PUBLIC_DOC_BYTES: usize = 0x800_000;

/// Largest accepted delay between full public readiness attempts.
const MAX_RETRY_DELAY_MS: i64 = 0x7530;

/// Compile-time references preserve the named helper boundaries.
const _: [usize; 0x000b] = [
    size_of_val(&FetchContext::new),
    size_of_val(&FetchPolicy::new),
    size_of_val(&bounded_validate_declared_length),
    size_of_val(&bounded_validate_read_body),
    size_of_val(&bounded_response_text),
    size_of_val(&fetch_text_from_base),
    size_of_val(&fetch_text_once),
    size_of_val(&normalize_target_url),
    size_of_val(&request_text),
    size_of_val(&request_url),
    size_of_val(&retry_delay),
];

/// Shared bounded HTTP client and retry configuration for public docs checks.
#[derive(Debug)]
pub(super) struct FetchContext {
    /// Zero-based full readiness attempt used to avoid stale cached results.
    attempt: i64,
    /// Normalized public docs base URL.
    base_url: String,
    /// Bounded blocking HTTP client.
    client: Client,
    /// Retry, propagation, and revision controls.
    policy: FetchPolicy,
}

impl FetchContext {
    /// Return the normalized public docs base URL.
    #[inline]
    pub(super) const fn base_url(&self) -> &str {
        return self.base_url.as_str();
    }

    /// Return the validated cache identity used to bypass stale CDN objects.
    #[inline]
    const fn cache_identity(&self) -> Option<&DocsCacheIdentity> {
        return self.policy.cache_identity.as_ref();
    }

    /// Return whether another delayed readiness attempt fits before the deadline.
    pub(super) fn can_retry_after_delay(&self) -> bool {
        return Instant::now()
            .checked_add(self.retry_delay())
            .is_some_and(|next_attempt| return next_attempt < self.policy.deadline);
    }

    /// Return the current commit revision used to resolve the deployed docs ancestor.
    #[inline]
    pub(super) fn commit_revision(&self) -> Option<&str> {
        return self.policy.cache_identity.as_ref().map(AsRef::as_ref);
    }

    /// Construct a bounded public documentation fetch context.
    #[inline]
    pub(super) const fn new(base_url: String, client: Client, policy: FetchPolicy) -> Self {
        return Self {
            attempt: 0,
            base_url,
            client,
            policy,
        };
    }

    /// Return the zero-based full readiness attempt.
    #[inline]
    pub(super) const fn readiness_attempt(&self) -> i64 {
        return self.attempt;
    }

    /// Reject a new network request after the shared readiness deadline.
    ///
    /// # Errors
    ///
    /// Returns an error after the shared public readiness deadline expires.
    fn require_request_time(&self) -> FetchResult<()> {
        if Instant::now() < self.policy.deadline {
            return Ok(());
        }
        return Err(FetchError {
            message: "public docs readiness exceeded its shared wall-clock deadline".to_owned(),
        });
    }

    /// Return the configured retry count.
    #[inline]
    pub(super) const fn retries(&self) -> i64 {
        return self.policy.retries;
    }

    /// Return the delay between retryable requests.
    #[inline]
    pub(super) const fn retry_delay(&self) -> Duration {
        return self.policy.retry_delay;
    }

    /// Set the zero-based full readiness attempt before a contract pass.
    #[inline]
    pub(super) const fn set_readiness_attempt(&mut self, attempt: i64) {
        self.attempt = attempt;
    }
}

#[derive(Debug)]
/// Contract representation for `FetchError`.
pub(super) struct FetchError {
    /// Contract data stored in `message`.
    message: String,
}

/// Retry, propagation, and immutable revision controls for public docs checks.
#[derive(Debug)]
pub(super) struct FetchPolicy {
    /// Validated deployment and workflow-run cache identity.
    cache_identity: Option<DocsCacheIdentity>,
    /// Shared wall-clock deadline for every readiness attempt and request.
    deadline: Instant,
    /// Maximum number of full readiness retry attempts.
    retries: i64,
    /// Delay between full readiness attempts.
    retry_delay: Duration,
}

impl FetchPolicy {
    /// Construct a public documentation fetch policy.
    #[inline]
    pub(super) const fn new(
        retries: i64,
        retry_delay: Duration,
        cache_identity: Option<DocsCacheIdentity>,
        deadline: Instant,
    ) -> Self {
        return Self {
            cache_identity,
            deadline,
            retries,
            retry_delay,
        };
    }
}

/// Result returned by bounded public documentation fetch helpers.
type FetchResult<T> = Result<T, FetchError>;

/// HTTP request headers used by Mintlify checks.
pub(super) type RequestHeaders<'headers> = [(&'headers str, &'headers str)];

/// Limits and diagnostics associated with one response body.
#[derive(Debug)]
struct ResponseConstraints {
    /// Declared response size when supplied by the server.
    content_length: Option<u64>,
    /// Maximum accepted body size.
    maximum: usize,
    /// Public endpoint path used in diagnostics.
    path: String,
    /// HTTP response status used in bounded-body diagnostics.
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
        };
    }));
    let read_limit_u64 = check_try!(u64::try_from(read_limit).map_err(|error| {
        return FetchError {
            message: format!("convert public docs response limit: {error}"),
        };
    }));
    let mut bytes = Vec::new();
    let read_count = check_try!(
        response
            .take(read_limit_u64)
            .read_to_end(&mut bytes)
            .map_err(|error| return FetchError {
                message: error.to_string(),
            })
    );
    if read_count != bytes.len() {
        return Err(FetchError {
            message: format!("{} changed while its response was read", constraints.path),
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
            "{} returned HTTP {} and exceeds the {}-byte public docs limit",
            constraints.path,
            constraints.status.as_u16(),
            constraints.maximum
        ),
    };
}

/// Contract implementation for `fetch_text`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn fetch_text<'headers>(
    context: &FetchContext,
    path: &str,
    headers: &'headers RequestHeaders<'headers>,
) -> CheckResult<String> {
    return fetch_text_once(context, path, headers);
}

/// Fetch one bounded response from a separately validated public base URL.
///
/// # Errors
///
/// Returns an error when the request fails or its response violates the bounded body contract.
pub(super) fn fetch_text_from_base<'headers>(
    context: &FetchContext,
    base_url: &str,
    path: &str,
    headers: &'headers RequestHeaders<'headers>,
) -> CheckResult<String> {
    let url = format!("{base_url}{path}");
    return request_url(context, url.as_str(), path, headers).map_err(|error| return error.message);
}

/// Fetch one public documentation response without applying retries.
///
/// # Errors
///
/// Returns an error when the request fails or its response violates the bounded body contract.
pub(super) fn fetch_text_once<'headers>(
    context: &FetchContext,
    path: &str,
    headers: &'headers RequestHeaders<'headers>,
) -> CheckResult<String> {
    return request_text(context, path, headers).map_err(|error| return error.message);
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
pub(super) fn request_text<'headers>(
    context: &FetchContext,
    path: &str,
    headers: &'headers RequestHeaders<'headers>,
) -> Result<String, FetchError> {
    let request_path =
        render_cache_path(path, context.cache_identity(), context.readiness_attempt());
    let url = format!("{}{request_path}", context.base_url());
    return request_url(context, url.as_str(), path, headers);
}

/// Fetch one bounded URL through the shared deadline and transport policy.
///
/// # Errors
///
/// Returns an error when the request fails or its response violates the bounded body contract.
fn request_url<'headers>(
    context: &FetchContext,
    url: &str,
    path: &str,
    headers: &'headers RequestHeaders<'headers>,
) -> Result<String, FetchError> {
    check_try!(context.require_request_time());
    let response = check_try!(
        context
            .client
            .get(url, headers, MAX_PUBLIC_DOC_BYTES)
            .map_err(|error| return FetchError { message: error })
    );
    let status = response.status();
    if !status.is_success() {
        return Err(FetchError {
            message: format!("{path} returned {}", status.as_u16()),
        });
    }
    let content_length = response.content_length();
    let mut body = response.body();
    let text = check_try!(bounded_response_text(
        &mut body,
        &ResponseConstraints {
            content_length,
            maximum: MAX_PUBLIC_DOC_BYTES,
            path: path.to_owned(),
            status,
        },
    ));
    check_try!(context.require_request_time());
    return Ok(text);
}

/// Contract implementation for `retry_delay`.
///
/// # Errors
///
/// Returns an error when the contract requirement cannot be verified.
pub(super) fn retry_delay() -> CheckResult<Duration> {
    let retry_delay_ms = check_try!(env_int("TOVUK_DOCS_CHECK_RETRY_DELAY_MS", 5_000));
    if !(0..=MAX_RETRY_DELAY_MS).contains(&retry_delay_ms) {
        return Err(format!(
            "TOVUK_DOCS_CHECK_RETRY_DELAY_MS must be between 0 and {MAX_RETRY_DELAY_MS}."
        ));
    }
    return Ok(Duration::from_millis(check_try!(
        u64::try_from(retry_delay_ms).map_err(|error| {
            return format!("TOVUK_DOCS_CHECK_RETRY_DELAY_MS must be non-negative: {error}");
        })
    )));
}

#[cfg(test)]
#[path = "mintlify_fetch_tests/verification.rs"]
mod tests;
