#[cfg(test)]
#[path = "http_tests.rs"]
/// HTTP transport tests.
mod tests;

#[path = "http/transport.rs"]
/// Strict public API transport construction.
mod transport;

#[path = "http/url_policy.rs"]
/// Public API URL and redirect admission policy.
mod url_policy;

use super::super::{
    args::CliOptions,
    constants::VERSION,
    errors::{
        AgentErrorContext, AgentErrorPayload, CliError, OutputFormat, Result,
        agent_error_with_context, internal_error,
    },
};
use core::{error::Error as CoreError, result::Result as CoreResult, time::Duration};
use http_body_util::{BodyExt as _, Full, LengthLimitError, Limited};
use hyper::{
    Method, Request, StatusCode, Uri,
    body::{Body as _, Bytes, Frame},
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
};
use hyper_util::client::legacy::Error as HyperClientError;
use serde_json::{Value, from_str, to_vec};
use std::io::Read;
use tokio::time::timeout;
use tower_http::follow_redirect::{
    FollowRedirect,
    policy::{FilterCredentials, Limited as RedirectLimit, PolicyExt as _, clone_body_fn},
};
use tower_service::Service as _;

use transport::{
    ApiClient, ClientConfiguration, RuntimeConfiguration, TransportClient, TransportRuntime,
};

use url_policy::{SafeRedirect, ValidatedUri};

/// Maximum total time allowed for one public API exchange.
const API_REQUEST_TIMEOUT: Duration = Duration::from_secs(0b1_1110);
/// Public JSON response body limit of 100 mebibytes.
const JSON_RESPONSE_LIMIT: ResponseBodyLimit = ResponseBodyLimit {
    maximum: MAX_JSON_RESPONSE_BYTES,
    read_ceiling: MAX_JSON_RESPONSE_READ_BYTES,
};
/// Maximum number of redirect responses followed for one API exchange.
const MAX_API_REDIRECTS: usize = 0b1010;
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
/// Maximum time allowed to establish the underlying TCP connection.
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(0b1010);

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

/// Concrete request body sent by the public API client.
type ApiBody = Full<Bytes>;
#[derive(Debug)]
/// Fully materialized request input used to build a Hyper request.
struct ApiRequest {
    /// Optional bearer authorization header value.
    authorization: Option<String>,
    /// Serialized request body.
    body: ApiRequestBody,
    /// HTTP request method.
    method: Method,
    /// Validated absolute request URI.
    uri: Uri,
}

impl TryFrom<(Method, Uri, ApiRequestContent)> for ApiRequest {
    type Error = CliError;

    fn try_from(value: (Method, Uri, ApiRequestContent)) -> Result<Self> {
        let (method, uri, content) = value;
        let (authorization, body) = match content {
            ApiRequestContent::Anonymous => (None, ApiRequestBody::Empty),
            ApiRequestContent::Authenticated { body, token } => {
                let request_body = match body {
                    Some(payload) => ApiRequestBody::Json(result_or_return!(
                        to_vec(&payload).map_err(|error| return internal_error(error.to_string()))
                    )),
                    None => ApiRequestBody::Empty,
                };
                (Some(format!("Bearer {token}")), request_body)
            }
        };
        return Ok(Self {
            authorization,
            body,
            method,
            uri,
        });
    }
}

#[derive(Debug)]
/// Serialized body attached to a public API request.
enum ApiRequestBody {
    /// Request without a body.
    Empty,
    /// JSON request bytes.
    Json(Vec<u8>),
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
/// Buffered status and body returned by one transport exchange.
struct ApiResponse {
    /// Validated response body text.
    body: ResponseText,
    /// HTTP response status.
    status: StatusCode,
}

/// Executes one asynchronous public API transport exchange.
trait ExecuteExchange {
    /// Sends the request and buffers its bounded response.
    ///
    /// # Errors
    ///
    /// Returns an error for timeout, connection, TLS, HTTP, body, or decoding failures.
    async fn execute(self) -> Result<ApiResponse>;
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

/// Result produced while reading one size-limited body frame.
type LimitedFrameResult = CoreResult<Frame<Bytes>, Box<dyn CoreError + Send + Sync>>;

impl TryFrom<ApiRequest> for Request<ApiBody> {
    type Error = CliError;

    #[inline]
    fn try_from(value: ApiRequest) -> Result<Self> {
        let mut builder = Request::builder()
            .method(value.method)
            .uri(value.uri)
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, format!("tovuk-cli/{VERSION}"));
        if let Some(authorization) = value.authorization {
            builder = builder.header(AUTHORIZATION, authorization);
        }
        let body = match value.body {
            ApiRequestBody::Empty => Vec::new(),
            ApiRequestBody::Json(json) => {
                builder = builder.header(CONTENT_TYPE, "application/json");
                json
            }
        };
        return builder
            .body(Full::new(Bytes::from(body)))
            .map_err(|error| return internal_error(error.to_string()));
    }
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
/// Data bytes extracted from one HTTP body frame.
struct ResponseData(Bytes);

impl TryFrom<LimitedFrameResult> for ResponseData {
    type Error = CliError;

    fn try_from(value: LimitedFrameResult) -> Result<Self> {
        return match value {
            Ok(frame) => Ok(Self(frame.into_data().unwrap_or_default())),
            Err(error) if error.downcast_ref::<LengthLimitError>().is_some() => {
                Err(oversized_response_error())
            }
            Err(error) => Err(internal_error(error.to_string())),
        };
    }
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

/// Exchange whose complete execution is bounded by the public request deadline.
struct TimedExchange {
    /// Hyper client used for the exchange.
    client: ApiClient,
    /// Output format used by stable transport errors.
    output_format: OutputFormat,
    /// Fully built HTTP request.
    request: Request<ApiBody>,
}

impl ExecuteExchange for TimedExchange {
    async fn execute(self) -> Result<ApiResponse> {
        let exchange = timeout(
            API_REQUEST_TIMEOUT,
            UntimedExchange {
                client: self.client,
                output_format: self.output_format,
                request: self.request,
            }
            .execute(),
        )
        .await;
        return match exchange {
            Ok(response_result) => response_result,
            Err(_elapsed) => Err(api_unreachable_error(
                "Tovuk API request exceeded the 30 second timeout.",
                self.output_format,
            )),
        };
    }
}

/// Exchange executed after the public request deadline has been installed.
struct UntimedExchange {
    /// Hyper client used for the exchange.
    client: ApiClient,
    /// Output format used by stable transport errors.
    output_format: OutputFormat,
    /// Fully built HTTP request.
    request: Request<ApiBody>,
}

impl ExecuteExchange for UntimedExchange {
    async fn execute(self) -> Result<ApiResponse> {
        let safe_redirect = SafeRedirect::from_body(self.request.body());
        let redirect_policy = RedirectLimit::new(MAX_API_REDIRECTS)
            .and::<_, ApiBody, HyperClientError>(FilterCredentials::default())
            .and::<_, ApiBody, HyperClientError>(safe_redirect)
            .and::<_, ApiBody, HyperClientError>(clone_body_fn(|body: &ApiBody| {
                return Some(body.clone());
            }));
        let mut client =
            FollowRedirect::with_policy(self.client, redirect_policy).preserve_extensions(false);
        let response = result_or_return!(client.call(self.request).await.map_err(|error| {
            return api_unreachable_error(
                format!("Could not reach Tovuk API: {error}"),
                self.output_format,
            );
        }));
        let status = response.status();
        if response
            .body()
            .size_hint()
            .upper()
            .is_some_and(|length| return length > MAX_JSON_RESPONSE_BYTES_U64)
        {
            return Err(oversized_response_error());
        }
        let mut bytes = Vec::new();
        let mut response_body = Limited::new(response.into_body(), JSON_RESPONSE_LIMIT.maximum);
        while let Some(frame_result) = response_body.frame().await {
            let ResponseData(data) = result_or_return!(ResponseData::try_from(frame_result));
            bytes.extend_from_slice(&data);
        }
        let body = result_or_return!(
            String::from_utf8(bytes)
                .map(ResponseText)
                .map_err(|error| return internal_error(error.to_string()))
        );
        return Ok(ApiResponse { body, status });
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
    let uri = result_or_return!(ValidatedUri::try_from((cli.api_url(), route))).into_inner();
    let request_input = result_or_return!(ApiRequest::try_from((method, uri, content)));
    let request = result_or_return!(Request::<ApiBody>::try_from(request_input));
    let client = result_or_return!(TransportClient::try_from(ClientConfiguration)).into_inner();
    let runtime = result_or_return!(TransportRuntime::try_from(RuntimeConfiguration)).into_inner();
    let response = result_or_return!(
        runtime.block_on(
            TimedExchange {
                client,
                output_format: cli.output_format(),
                request,
            }
            .execute()
        )
    );
    let status = response.status;
    let text = response.body.0;
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

/// Creates the stable transport failure used by public automation clients.
fn api_unreachable_error(message: impl Into<String>, output_format: OutputFormat) -> CliError {
    return agent_error_with_context(
        "api_unreachable",
        message,
        AgentErrorContext::new(
            Some(STATUS_INSTRUCTION.to_owned()),
            Some(STATUS_DOCS_URL.to_owned()),
            None,
        ),
        output_format,
    );
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
