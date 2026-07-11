//! Bounded synchronous HTTP over Hyper and Rustls with Ring.

#[path = "http_transport/config.rs"]
mod config;

#[path = "http_transport/redirect.rs"]
mod redirect;

#[path = "http_transport/response.rs"]
mod response;

#[cfg(test)]
#[path = "http_transport_tests/verification.rs"]
mod tests;

use core::time::Duration;

use http::{HeaderMap, HeaderValue, Request, StatusCode};

use http_body_util::Empty;

use hyper::{
    Response as HyperResponse,
    body::{Bytes, Incoming},
};

use hyper_rustls::HttpsConnector;

use hyper_util::client::legacy::{Client as HyperClient, connect::HttpConnector};

use alloc::collections::BTreeSet;

use tokio::{runtime::Runtime, time::timeout};

use url::Url;

use config::{
    build_headers, build_loopback_client, build_request, build_runtime, build_secure_client,
    parse_user_agent, validate_configuration,
};

use redirect::{
    advance_redirect, next_redirect, parse_request_url, record_visit, redirect_headers,
};

use response::collect_response;

/// Compile-time references preserve the named transport boundaries.
const _: [usize; 0x03] = [
    size_of_val(&Client::request),
    size_of_val(&Client::request_with_timeout),
    size_of_val(&Client::send),
];

/// Synchronous bounded HTTP client shared by repository checks.
#[derive(Debug)]
pub struct Client {
    /// Plaintext client reached only after loopback URL validation.
    loopback: LoopbackClient,
    /// Maximum redirects followed by one request.
    redirect_limit: u8,
    /// Total deadline for a request and all of its redirects.
    request_timeout: Duration,
    /// Current-thread asynchronous executor hidden behind the synchronous API.
    runtime: Runtime,
    /// Certificate- and hostname-verifying HTTPS client.
    secure: SecureClient,
    /// Public identifier sent with every request.
    user_agent: HeaderValue,
}

impl Client {
    /// Build one certificate-verifying client with bounded connections and requests.
    ///
    /// # Errors
    ///
    /// Returns an error when a timeout is zero, the redirect limit is excessive,
    /// the user agent is invalid, or a runtime or TLS configuration cannot be built.
    #[inline]
    pub fn build(
        connect_timeout: Duration,
        request_timeout: Duration,
        redirect_limit: u8,
        user_agent: &str,
    ) -> TransportResult<Self> {
        check_try!(validate_configuration(
            connect_timeout,
            request_timeout,
            redirect_limit,
            user_agent,
        ));
        let runtime = check_try!(build_runtime());
        let secure = check_try!(build_secure_client(connect_timeout));
        return Ok(Self {
            loopback: build_loopback_client(connect_timeout),
            redirect_limit,
            request_timeout,
            runtime,
            secure,
            user_agent: check_try!(parse_user_agent(user_agent)),
        });
    }

    /// Fetch one response body through the configured deadline and byte ceiling.
    ///
    /// Plaintext HTTP is accepted only for literal loopback addresses and
    /// `localhost`. HTTPS uses native roots, Ring, and Rustls hostname
    /// verification.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid or unsafe URL, invalid headers, a timeout,
    /// redirect policy failure, transport failure, or oversized response.
    #[inline]
    pub fn get<'headers>(
        &self,
        url: &str,
        headers: &'headers RequestHeaders<'headers>,
        maximum_body_bytes: usize,
    ) -> TransportResult<Response> {
        let initial_url = check_try!(parse_request_url(url));
        let initial_headers = check_try!(build_headers(headers, &self.user_agent));
        let operation =
            Self::request_with_timeout(self, initial_url, initial_headers, maximum_body_bytes);
        return self.runtime.block_on(operation);
    }

    /// Follow safe redirects and return one bounded terminal response.
    ///
    /// # Errors
    ///
    /// Returns an error for a redirect failure, transport failure, or oversized body.
    async fn request(
        &self,
        initial_url: Url,
        initial_headers: HeaderMap,
        maximum_body_bytes: usize,
    ) -> TransportResult<Response> {
        let mut current_url = initial_url.clone();
        let mut followed_redirects: u8 = 0x00;
        let mut visited_urls = BTreeSet::new();
        loop {
            check_try!(record_visit(&mut visited_urls, &current_url));
            let headers = redirect_headers(&initial_url, &current_url, &initial_headers);
            let response = check_try!(Self::send(self, &current_url, headers).await);
            let redirect_url = check_try!(next_redirect(
                &current_url,
                &response,
                followed_redirects,
                self.redirect_limit,
            ));
            match check_try!(advance_redirect(
                &mut current_url,
                &mut followed_redirects,
                redirect_url,
            )) {
                Some(()) => drop(response),
                None => return collect_response(response, maximum_body_bytes).await,
            }
        }
    }

    /// Apply the total deadline to one request and all safe redirects.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or exceeds the configured deadline.
    async fn request_with_timeout(
        &self,
        initial_url: Url,
        initial_headers: HeaderMap,
        maximum_body_bytes: usize,
    ) -> TransportResult<Response> {
        let operation = Self::request(self, initial_url, initial_headers, maximum_body_bytes);
        return match timeout(self.request_timeout, operation).await {
            Ok(result) => result,
            Err(error) => Err(format!(
                "HTTP request exceeded {:?}: {error}",
                self.request_timeout
            )),
        };
    }

    /// Send one GET request with the client appropriate for its validated scheme.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected Hyper client cannot complete the request.
    async fn send(
        &self,
        url: &Url,
        headers: HeaderMap,
    ) -> TransportResult<HyperResponse<Incoming>> {
        let request = check_try!(build_request(url, headers));
        if url.scheme() == "http" {
            return self
                .loopback
                .request(request)
                .await
                .map_err(|error| format!("request {url}: {error}"));
        }
        return self
            .secure
            .request(request)
            .await
            .map_err(|error| format!("request {url}: {error}"));
    }
}

/// Parsed optional response length operation.
type LengthResult = TransportResult<Option<u64>>;

/// Hyper client restricted to loopback plaintext requests.
type LoopbackClient = HyperClient<HttpConnector, RequestBody>;

/// Parsed optional redirect operation.
type RedirectResult = TransportResult<Option<Url>>;

/// Empty request body used for every repository check request.
type RequestBody = Empty<Bytes>;

/// Borrowed HTTP header name and value pairs supplied by one caller.
pub type RequestHeaders<'headers> = [(&'headers str, &'headers str)];

/// Built request operation.
type RequestResult = TransportResult<Request<RequestBody>>;

/// One bounded terminal HTTP response.
#[derive(Debug)]
pub struct Response {
    /// Complete response bytes within the caller's ceiling.
    body: Vec<u8>,
    /// Parsed server-declared response length when present.
    content_length: Option<u64>,
    /// Terminal response status.
    status: StatusCode,
}

impl Response {
    /// Return the complete bounded body.
    #[must_use]
    #[inline]
    pub const fn body(&self) -> &[u8] {
        return self.body.as_slice();
    }

    /// Return the declared response length when supplied by the server.
    #[must_use]
    #[inline]
    pub const fn content_length(&self) -> Option<u64> {
        return self.content_length;
    }

    /// Return the terminal HTTP status.
    #[must_use]
    #[inline]
    pub const fn status(&self) -> StatusCode {
        return self.status;
    }
}

/// Hyper client whose connections are authenticated by Rustls with Ring.
type SecureClient = HyperClient<HttpsConnector<HttpConnector>, RequestBody>;

/// Result returned by shared HTTP transport operations.
pub type TransportResult<Value = ()> = Result<Value, String>;
