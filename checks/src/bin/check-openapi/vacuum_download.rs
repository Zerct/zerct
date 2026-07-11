//! Bounded Vacuum release download transport.

use core::time::Duration;

use tovuk_public_checks::{check_support::CheckResult, http_transport::Client};

/// Maximum duration allowed to establish a Vacuum release connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(0x0a);

/// Largest accepted compressed Vacuum release archive.
const MAXIMUM_VACUUM_ARCHIVE_BYTES: usize = 0x0400_0000;

/// Safe redirect ceiling for `GitHub` release asset downloads.
const REDIRECT_LIMIT: u8 = 0x05;

/// Total duration allowed for one Vacuum release request and its redirects.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(0x01);

/// Public identifier sent with Vacuum release requests.
const USER_AGENT: &str = "Tovuk public OpenAPI check (https://github.com/tovuk/tovuk)";

/// Compile-time references preserve the named download boundaries.
const _: [usize; 0x02] = [size_of_val(&download_asset), size_of_val(&download_client)];

/// Download a Vacuum release archive.
///
/// # Errors
///
/// Returns an error when the request fails, its status is unsuccessful, or its
/// response body cannot be read.
pub(super) fn download_asset(url: &str) -> CheckResult<Vec<u8>> {
    let client = check_try!(download_client());
    let response = check_try!(
        client
            .get(url, &[], MAXIMUM_VACUUM_ARCHIVE_BYTES)
            .map_err(|error| return format!("download {url}: {error}"))
    );
    let status = response.status();
    if !status.is_success() {
        return Err(format!("download {url} failed with status {status}"));
    }
    return Ok(response.body().to_vec());
}

/// Build the bounded Rustls client used for Vacuum downloads.
///
/// # Errors
///
/// Returns an error when the client cannot be constructed.
fn download_client() -> CheckResult<Client> {
    return Client::build(CONNECT_TIMEOUT, REQUEST_TIMEOUT, REDIRECT_LIMIT, USER_AGENT)
        .map_err(|error| return format!("build Vacuum download client: {error}"));
}
