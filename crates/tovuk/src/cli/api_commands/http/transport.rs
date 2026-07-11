//! Hyper and Rustls transport construction for public API requests.

use super::{ApiBody, CliError, Result, TCP_CONNECT_TIMEOUT, internal_error};

use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};

use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};

use rustls::crypto::aws_lc_rs::default_provider;

use tokio::runtime::{Builder as RuntimeBuilder, Runtime as TokioRuntime};

/// HTTPS-or-validated-loopback client backed by Rustls and Hyper.
pub(super) type ApiClient = Client<HttpsConnector<HttpConnector>, ApiBody>;

#[derive(Clone, Copy, Debug)]
/// Marker selecting the strict public TLS client configuration.
pub(super) struct ClientConfiguration;

#[derive(Clone, Copy, Debug)]
/// Marker selecting the current-thread transport runtime configuration.
pub(super) struct RuntimeConfiguration;

/// Strict Hyper client wrapper.
pub(super) struct TransportClient(ApiClient);

impl TransportClient {
    /// Consume the wrapper and return its configured Hyper client.
    #[inline]
    pub(super) fn into_inner(self) -> ApiClient {
        return self.0;
    }
}

impl TryFrom<ClientConfiguration> for TransportClient {
    type Error = CliError;

    fn try_from(_value: ClientConfiguration) -> Result<Self> {
        let provider = default_provider();
        let tls_builder = result_or_return!(
            HttpsConnectorBuilder::new()
                .with_provider_and_native_roots(provider)
                .map_err(|error| return internal_error(error.to_string()))
        );
        let mut http_connector = HttpConnector::new();
        http_connector.enforce_http(false);
        http_connector.set_connect_timeout(Some(TCP_CONNECT_TIMEOUT));
        http_connector.set_nodelay(true);
        let connector = tls_builder
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);
        let client = Client::builder(TokioExecutor::new()).build(connector);
        return Ok(Self(client));
    }
}

/// Current-thread Tokio runtime wrapper.
pub(super) struct TransportRuntime(TokioRuntime);

impl TransportRuntime {
    /// Consume the wrapper and return its current-thread runtime.
    #[inline]
    pub(super) fn into_inner(self) -> TokioRuntime {
        return self.0;
    }
}

impl TryFrom<RuntimeConfiguration> for TransportRuntime {
    type Error = CliError;

    fn try_from(_value: RuntimeConfiguration) -> Result<Self> {
        return RuntimeBuilder::new_current_thread()
            .enable_all()
            .build()
            .map(Self)
            .map_err(|error| return internal_error(error.to_string()));
    }
}
