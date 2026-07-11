/// Public billing checkout route.
pub(super) const BILLING_CHECKOUT_ROUTE: &str = "/v1/billing/checkout";
/// Default public API base URL.
pub(super) const DEFAULT_API_URL: &str = "https://api.tovuk.com";
/// Credential-store account name.
pub(super) const SESSION_ACCOUNT: &str = "session-token";
/// Legacy session directory name.
pub(super) const SESSION_DIR: &str = ".tovuk";
/// Session token file name.
pub(super) const SESSION_FILE: &str = "session-token";
/// Human-readable credential-store label.
pub(super) const SESSION_LABEL: &str = "Tovuk session";
/// Credential-store service identifier.
pub(super) const SESSION_SERVICE: &str = "com.tovuk.cli";
/// Published CLI version.
pub(super) const VERSION: &str = env!("CARGO_PKG_VERSION");
