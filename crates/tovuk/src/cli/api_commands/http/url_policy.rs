//! Public API URL and redirect safety policy.

use super::{ApiBody, CliError, Result, internal_error};

use core::{net::IpAddr, result::Result as CoreResult, str::FromStr as _};

use hyper::{Method, Request, StatusCode, Uri, body::Body};

use tower_http::follow_redirect::policy::{
    Action as RedirectAction, Attempt as RedirectAttempt, Policy as RedirectPolicy,
};

/// Compile-time references preserve the redirect transition boundaries.
const _: [usize; 0x05] = [
    size_of_val(&SafeRedirect::from_body::<ApiBody>),
    size_of_val(&redirect_body_presence),
    size_of_val(&redirect_transition_is_allowed),
    size_of_val(&same_origin),
    size_of_val(&uri_is_loopback),
];

#[derive(Clone, Copy, Debug)]
/// Whether a request body remains available for replay.
pub(super) enum BodyPresence {
    /// The redirected request carries no body bytes.
    Empty,
    /// The redirected request carries body bytes.
    Present,
}

impl BodyPresence {
    /// Classify one request body conservatively from its advertised size.
    fn from_body<BodyValue>(body: &BodyValue) -> Self
    where
        BodyValue: Body,
    {
        if body.size_hint().upper() == Some(0x00) {
            return Self::Empty;
        }
        return Self::Present;
    }
}

#[derive(Clone, Copy, Debug)]
/// Marker proving that a host is a literal loopback endpoint.
pub(super) struct LoopbackHost;

impl TryFrom<&str> for LoopbackHost {
    type Error = ();

    fn try_from(value: &str) -> CoreResult<Self, Self::Error> {
        if value.eq_ignore_ascii_case("localhost") {
            return Ok(Self);
        }
        let unbracketed = value
            .strip_prefix('[')
            .and_then(|host| return host.strip_suffix(']'))
            .unwrap_or(value);
        if IpAddr::from_str(unbracketed).is_ok_and(|address| return address.is_loopback()) {
            return Ok(Self);
        }
        return Err(());
    }
}

#[derive(Clone, Copy, Debug)]
/// Redirect policy that preserves the transport scheme and loopback boundary.
pub(super) struct SafeRedirect {
    /// Whether the request presented to the policy carries a replayable body.
    request_body: BodyPresence,
}

impl SafeRedirect {
    /// Initialize redirect state from the first request before middleware execution.
    #[inline]
    pub(super) fn from_body<BodyValue>(body: &BodyValue) -> Self
    where
        BodyValue: Body,
    {
        return Self {
            request_body: BodyPresence::from_body(body),
        };
    }
}

impl<BodyValue, ErrorValue> RedirectPolicy<BodyValue, ErrorValue> for SafeRedirect
where
    BodyValue: Body,
{
    fn clone_body(&self, _body: &BodyValue) -> Option<BodyValue> {
        return None;
    }

    fn on_request(&mut self, request: &mut Request<BodyValue>) {
        self.request_body = BodyPresence::from_body(request.body());
    }

    fn redirect(
        &mut self,
        attempt: &RedirectAttempt<'_>,
    ) -> CoreResult<RedirectAction, ErrorValue> {
        if ValidatedUri::try_from(attempt.location().clone()).is_ok()
            && redirect_transition_is_allowed(
                attempt.previous(),
                attempt.location(),
                attempt.method(),
                redirect_body_presence(
                    attempt.status(),
                    attempt.previous_method(),
                    self.request_body,
                ),
            )
        {
            return Ok(RedirectAction::Follow);
        }
        return Ok(RedirectAction::Stop);
    }
}

#[derive(Debug)]
/// Absolute API URI admitted by the public transport policy.
pub(super) struct ValidatedUri(Uri);

impl ValidatedUri {
    /// Consume the validated wrapper and return its admitted URI.
    #[inline]
    pub(super) fn into_inner(self) -> Uri {
        return self.0;
    }
}

impl TryFrom<(&str, &str)> for ValidatedUri {
    type Error = CliError;

    fn try_from(value: (&str, &str)) -> Result<Self> {
        let (api_url, route) = value;
        let uri = result_or_return!(
            Uri::from_str(&format!("{api_url}{route}"))
                .map_err(|error| return internal_error(format!("Invalid Tovuk API URL: {error}")))
        );
        return Self::try_from(uri);
    }
}

impl TryFrom<Uri> for ValidatedUri {
    type Error = CliError;

    fn try_from(value: Uri) -> Result<Self> {
        let uri = value;
        let has_authority = uri.authority().is_some();
        let secure = has_authority && uri.scheme_str() == Some("https");
        let loopback = has_authority
            && uri.scheme_str() == Some("http")
            && uri
                .host()
                .is_some_and(|host| return LoopbackHost::try_from(host).is_ok());
        if secure || loopback {
            return Ok(Self(uri));
        }
        return Err(internal_error(
            "Tovuk API URLs must use HTTPS; plaintext HTTP is limited to loopback test endpoints.",
        ));
    }
}

/// Return whether Tower retains the request body for this redirect response.
fn redirect_body_presence(
    status: StatusCode,
    previous_method: &Method,
    request_body: BodyPresence,
) -> BodyPresence {
    return match status {
        StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND if previous_method == Method::POST => {
            BodyPresence::Empty
        }
        StatusCode::SEE_OTHER => BodyPresence::Empty,
        _ => request_body,
    };
}

/// Return whether a redirect preserves transport and request trust boundaries.
pub(super) fn redirect_transition_is_allowed(
    previous: &Uri,
    location: &Uri,
    method: &Method,
    body: BodyPresence,
) -> bool {
    if previous.scheme_str() == Some("https") && location.scheme_str() == Some("http") {
        return false;
    }
    if !uri_is_loopback(previous) && uri_is_loopback(location) {
        return false;
    }
    let method_is_safe = method == Method::GET || method == Method::HEAD;
    let body_is_empty = matches!(body, BodyPresence::Empty);
    return same_origin(previous, location) || method_is_safe && body_is_empty;
}

/// Return whether two absolute request URIs share an exact origin.
fn same_origin(previous: &Uri, location: &Uri) -> bool {
    return previous.scheme() == location.scheme() && previous.authority() == location.authority();
}

/// Return whether an absolute URI names an explicit loopback host.
fn uri_is_loopback(uri: &Uri) -> bool {
    return uri
        .host()
        .is_some_and(|host| return LoopbackHost::try_from(host).is_ok());
}
