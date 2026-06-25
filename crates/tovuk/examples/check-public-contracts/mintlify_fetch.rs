use std::{thread::sleep, time::Duration};

use reqwest::{StatusCode, blocking::Client};

use crate::helpers::{CheckResult, env_int};

#[derive(Debug)]
struct FetchError {
    message: String,
    status: Option<StatusCode>,
}

pub(crate) fn retry_delay() -> CheckResult<Duration> {
    let retry_delay_ms = env_int("TOVUK_DOCS_CHECK_RETRY_DELAY_MS", 5_000)?;
    Ok(Duration::from_millis(
        u64::try_from(retry_delay_ms)
            .map_err(|_| "TOVUK_DOCS_CHECK_RETRY_DELAY_MS must be non-negative".to_owned())?,
    ))
}

pub(crate) fn fetch_text(
    client: &Client,
    base_url: &str,
    path: &str,
    headers: &[(&str, &str)],
    retries: i64,
    retry_delay: Duration,
) -> CheckResult<String> {
    let mut last_error = FetchError {
        message: "request was not attempted".to_owned(),
        status: None,
    };
    for attempt in 0..=retries {
        match request_text(client, base_url, path, headers) {
            Ok(text) => return Ok(text),
            Err(error) => {
                let retryable = is_retryable_fetch_error(&error);
                last_error = error;
                if attempt == retries || !retryable {
                    break;
                }
                sleep(retry_delay);
            }
        }
    }
    Err(last_error.message)
}

pub(crate) fn normalize_target_url(target: &str) -> String {
    let with_scheme = if target.starts_with("http://") || target.starts_with("https://") {
        target.to_owned()
    } else {
        format!("https://{target}")
    };
    with_scheme.trim_end_matches('/').to_owned()
}

fn request_text(
    client: &Client,
    base_url: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> Result<String, FetchError> {
    let url = format!("{base_url}{path}");
    let mut request = client.get(url);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = request.send().map_err(|error| FetchError {
        message: error.to_string(),
        status: None,
    })?;
    let status = response.status();
    let body = response.text().map_err(|error| FetchError {
        message: error.to_string(),
        status: Some(status),
    })?;
    if !status.is_success() {
        return Err(FetchError {
            message: format!("{path} returned {}", status.as_u16()),
            status: Some(status),
        });
    }
    Ok(body)
}

fn is_retryable_fetch_error(error: &FetchError) -> bool {
    error
        .status
        .is_none_or(|status| status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
}
