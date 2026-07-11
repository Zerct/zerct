#[cfg(test)]
#[path = "http_tests.rs"]
/// HTTP transport tests.
mod tests;

use super::super::{
    args::CliOptions,
    constants::VERSION,
    errors::{
        AgentErrorContext, AgentErrorPayload, CliError, Result, agent_error_with_context,
        internal_error,
    },
};
use reqwest::{
    Method,
    blocking::{Client, Response},
};
use serde_json::{Value, from_str};
use std::io::Read;

/// Public JSON response body limit of 100 mebibytes.
const JSON_RESPONSE_LIMIT: ResponseBodyLimit = ResponseBodyLimit {
    maximum: MAX_JSON_RESPONSE_BYTES,
    read_ceiling: MAX_JSON_RESPONSE_READ_BYTES,
};
/// Maximum accepted public JSON response body size in bytes.
const MAX_JSON_RESPONSE_BYTES: usize = 0x0640_0000;
/// Maximum declared public JSON response body size in bytes.
const MAX_JSON_RESPONSE_BYTES_U64: u64 = 0x0640_0000;
/// Streaming read ceiling of one byte beyond the accepted response limit.
const MAX_JSON_RESPONSE_READ_BYTES: u64 = 0x0640_0001;
/// One byte beyond the accepted response limit for declared-length tests.
#[cfg(test)]
const MAX_JSON_RESPONSE_READ_BYTES_USIZE: usize = 0x0640_0001;
/// Public status documentation used for transport and server failures.
const STATUS_DOCS_URL: &str = "https://docs.tovuk.com/status";
/// Recovery guidance used for transport and server failures.
const STATUS_INSTRUCTION: &str =
    "Retry the command. If it keeps failing, check Tovuk status before changing your request.";

impl From<(&Value, u16)> for AgentErrorPayload {
    fn from(value: (&Value, u16)) -> Self {
        let (payload, status_code) = value;
        let Some(object) = payload.as_object() else {
            return status_payload(status_code);
        };
        let Some(code) = object.get("code").and_then(Value::as_str) else {
            return status_payload(status_code);
        };
        let Some(message) = object.get("message").and_then(Value::as_str) else {
            return status_payload(status_code);
        };
        return Self::new(
            code.to_owned(),
            message.to_owned(),
            AgentErrorContext::new(
                object
                    .get("agent_instruction")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                object
                    .get("docs_url")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                object
                    .get("checkout_url")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
        );
    }
}

/// Authentication and optional body supplied with an API request.
pub(in crate::cli) enum ApiRequestContent {
    /// Anonymous request without a body.
    Anonymous,
    /// Authenticated request with an optional JSON body.
    Authenticated {
        /// Optional JSON request body.
        body: Option<Value>,
        /// Bearer token used for authentication.
        token: String,
    },
}

#[derive(Debug)]
/// JSON value parsed under an explicit response policy.
struct JsonPayload(Value);

impl TryFrom<(&str, JsonPolicy)> for JsonPayload {
    type Error = CliError;

    fn try_from(value: (&str, JsonPolicy)) -> Result<Self> {
        let (text, policy) = value;
        if text.trim().is_empty() {
            return Ok(Self(Value::Null));
        }
        return match from_str(text) {
            Ok(payload) => Ok(Self(payload)),
            Err(_) if matches!(policy, JsonPolicy::Lenient) => Ok(Self(Value::Null)),
            Err(error) => Err(internal_error(format!(
                "Tovuk API returned invalid JSON: {error}"
            ))),
        };
    }
}

#[derive(Clone, Copy, Debug)]
/// Response JSON parsing behavior.
enum JsonPolicy {
    /// Accept an empty or invalid non-success response body as null.
    Lenient,
    /// Require a valid JSON success response.
    Strict,
}

#[derive(Clone, Copy, Debug)]
/// Hard byte limits applied while buffering one response body.
struct ResponseBodyLimit {
    /// Largest accepted response body.
    maximum: usize,
    /// Number of bytes read to detect an oversized stream.
    read_ceiling: u64,
}

#[derive(Debug)]
/// Fully buffered, size-limited, valid UTF-8 HTTP response body.
struct ResponseText(String);

impl<Reader> TryFrom<(Reader, ResponseBodyLimit)> for ResponseText
where
    Reader: Read,
{
    type Error = CliError;

    fn try_from(value: (Reader, ResponseBodyLimit)) -> Result<Self> {
        let (reader, limit) = value;
        let mut bytes = Vec::new();
        let mut limited_reader = reader.take(limit.read_ceiling);
        let bytes_read = result_or_return!(
            limited_reader
                .read_to_end(&mut bytes)
                .map_err(|error| return internal_error(error.to_string()))
        );
        if bytes_read > limit.maximum {
            return Err(oversized_response_error());
        }
        return String::from_utf8(bytes)
            .map(Self)
            .map_err(|error| return internal_error(error.to_string()));
    }
}

impl TryFrom<Response> for ResponseText {
    type Error = CliError;

    fn try_from(value: Response) -> Result<Self> {
        if value
            .content_length()
            .is_some_and(|length| return length > MAX_JSON_RESPONSE_BYTES_U64)
        {
            return Err(oversized_response_error());
        }
        return Self::try_from((value, JSON_RESPONSE_LIMIT));
    }
}

/// Sends one public API request and validates its JSON response.
///
/// # Errors
///
/// Returns an error for client creation, transport, response size, decoding, or non-success status.
pub(in crate::cli) fn api_request(
    cli: &CliOptions,
    method: Method,
    route: &str,
    content: ApiRequestContent,
) -> Result<Value> {
    let client = result_or_return!(
        Client::builder()
            .user_agent(format!("tovuk-cli/{VERSION}"))
            .build()
            .map_err(|error| return internal_error(error.to_string()))
    );
    let mut request = client
        .request(method, format!("{}{route}", cli.api_url()))
        .header("accept", "application/json");
    if let ApiRequestContent::Authenticated { body, token } = content {
        request = request.bearer_auth(token);
        if let Some(request_body) = body {
            request = request.json(&request_body);
        }
    }

    let response = result_or_return!(request.send().map_err(|error| {
        return agent_error_with_context(
            "api_unreachable",
            format!("Could not reach Tovuk API: {error}"),
            AgentErrorContext::new(
                Some(STATUS_INSTRUCTION.to_owned()),
                Some(STATUS_DOCS_URL.to_owned()),
                None,
            ),
            cli.output_format(),
        );
    }));
    let status = response.status();
    let text = result_or_return!(ResponseText::try_from(response)).0;
    if status.is_success() {
        return JsonPayload::try_from((text.as_str(), JsonPolicy::Strict))
            .map(|payload| return payload.0);
    }

    let data = result_or_return!(JsonPayload::try_from((text.as_str(), JsonPolicy::Lenient)));
    let payload = AgentErrorPayload::from((&data.0, status.as_u16()));
    return Err(CliError::new(
        payload,
        cli.output_format(),
        if status.is_server_error() { 0b10 } else { 0b1 },
    ));
}

/// Creates an error for a response exceeding the public 100 `MiB` JSON ceiling.
fn oversized_response_error() -> CliError {
    return internal_error("Tovuk API response exceeded the 100 MiB JSON response limit.");
}

/// Creates the fallback error payload for an HTTP status.
fn status_payload(status_code: u16) -> AgentErrorPayload {
    return AgentErrorPayload::new(
        "api_error".to_owned(),
        format!("Tovuk API returned HTTP {status_code}."),
        AgentErrorContext::new(Some(STATUS_INSTRUCTION.to_owned()), None, None),
    );
}
