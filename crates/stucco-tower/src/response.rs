//! HTML responses for full pages and fragments.

use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION, VARY, X_CONTENT_TYPE_OPTIONS};
use http::{HeaderValue, Response, StatusCode};
use http_body_util::Full;
use stucco_core::{Bundle, Href, RenderedFragment};

/// A full HTML page response.
///
/// ```
/// use stucco_tower::PageResponse;
/// let res = PageResponse::new("<!doctype html>…".into()).into_response();
/// assert_eq!(res.headers()["vary"], "Stucco-Request, Stucco-Target");
/// ```
#[derive(Clone, Debug)]
pub struct PageResponse {
    status: StatusCode,
    html: String,
}

impl PageResponse {
    /// A 200 page.
    pub fn new(html: String) -> Self {
        PageResponse {
            status: StatusCode::OK,
            html,
        }
    }

    /// Sets the status (e.g. 422 for a form with errors).
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// The HTTP response.
    pub fn into_response(self) -> Response<Full<Bytes>> {
        html_response(self.status, self.html, false)
    }
}

/// A fragment response for an enhanced request: the fragment's HTML,
/// announcing the modules it needs, never cached.
#[derive(Clone, Debug)]
pub struct FragmentResponse {
    status: StatusCode,
    html: String,
}

impl FragmentResponse {
    /// A 200 response for `fragment`, with module URLs from `bundle`.
    pub fn new(fragment: &RenderedFragment, bundle: &Bundle) -> Self {
        FragmentResponse {
            status: StatusCode::OK,
            html: fragment.to_response_html(bundle),
        }
    }

    /// Sets the status (422 and 409 are swapped in; others show a failure).
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// The HTTP response.
    pub fn into_response(self) -> Response<Full<Bytes>> {
        html_response(self.status, self.html, true)
    }
}

/// A `303 See Other` redirect: the response to a successful form submission,
/// so reloading the next page does not submit the form again.
///
/// Never build the location from request input (a `next` query parameter,
/// say) without checking it is one of your own paths: [`Href`] blocks
/// dangerous schemes, not other sites.
///
/// ```
/// use stucco_tower::SeeOther;
/// let res = SeeOther::new("/orders/42?saved=1").into_response();
/// assert_eq!(res.status(), 303);
/// assert_eq!(res.headers()["location"], "/orders/42?saved=1");
/// ```
#[derive(Clone, Debug)]
pub struct SeeOther {
    location: Href,
}

impl SeeOther {
    /// A redirect to `location`. Characters not allowed in a header are
    /// percent-encoded. A URL [`Href`] rejects panics in debug builds and
    /// redirects to `/` in release.
    pub fn new(location: impl Into<Href>) -> Self {
        SeeOther {
            location: location.into(),
        }
    }

    /// The HTTP response.
    pub fn into_response(self) -> Response<Full<Bytes>> {
        debug_assert!(self.location.is_valid(), "SeeOther: unsafe redirect target");
        let location = if self.location.is_valid() {
            self.location.as_str()
        } else {
            "/"
        };
        Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, header_safe(location))
            .header(X_CONTENT_TYPE_OPTIONS, "nosniff")
            .body(Full::new(Bytes::new()))
            .expect("valid response")
    }
}

/// `url` as a header value, percent-encoding bytes outside visible ASCII.
fn header_safe(url: &str) -> HeaderValue {
    let mut out = String::with_capacity(url.len());
    for b in url.bytes() {
        if (0x21..=0x7e).contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    HeaderValue::from_str(&out).expect("visible ASCII is a valid header value")
}

/// The request headers a page or fragment response depends on.
const VARY_HEADERS: &str = "Stucco-Request, Stucco-Target";

/// An HTML response with the headers every stucco page and fragment carries.
fn html_response(status: StatusCode, html: String, fragment: bool) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        // Negotiation reads both headers (`RequestKind::from_headers`), so a
        // cache must key on both.
        .header(VARY, VARY_HEADERS)
        .header(X_CONTENT_TYPE_OPTIONS, "nosniff");
    if fragment {
        builder = builder.header(CACHE_CONTROL, "no-store");
    }
    builder
        .body(Full::new(Bytes::from(html)))
        .expect("valid response")
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for PageResponse {
    fn into_response(self) -> axum::response::Response {
        PageResponse::into_response(self).map(axum::body::Body::new)
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for SeeOther {
    fn into_response(self) -> axum::response::Response {
        SeeOther::into_response(self).map(axum::body::Body::new)
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponse for FragmentResponse {
    fn into_response(self) -> axum::response::Response {
        FragmentResponse::into_response(self).map(axum::body::Body::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::behavior;
    use stucco_theme::Preset;

    #[test]
    fn page_responses_are_html_and_vary_on_the_request_kind() {
        let res = PageResponse::new("<p>x</p>".into()).into_response();
        assert_eq!(res.status(), 200);
        assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
        assert_eq!(res.headers()["vary"], "Stucco-Request, Stucco-Target");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
        assert!(!res.headers().contains_key("cache-control"));
    }

    #[test]
    fn responses_vary_on_every_header_negotiation_reads() {
        let vary: Vec<&str> = VARY_HEADERS.split(", ").collect();
        assert_eq!(vary, [behavior::HEADER_REQUEST, behavior::HEADER_TARGET]);
    }

    #[test]
    fn statuses_survive_conversion() {
        let page = PageResponse::new(String::new())
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .into_response();
        assert_eq!(page.status(), 422);
        let bundle = Bundle::new(Preset::Slate);
        let frag = stucco_core::render_fragment("x", &"hi");
        let res = FragmentResponse::new(&frag, &bundle)
            .status(StatusCode::CONFLICT)
            .into_response();
        assert_eq!(res.status(), 409);
        assert_eq!(res.headers()["cache-control"], "no-store");
        assert_eq!(res.headers()["vary"], "Stucco-Request, Stucco-Target");
        assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
    }

    #[test]
    fn redirects_encode_unsafe_header_bytes() {
        let res = SeeOther::new("/orders?q=caf\u{e9} au lait").into_response();
        assert_eq!(res.status(), 303);
        assert_eq!(res.headers()["location"], "/orders?q=caf%C3%A9%20au%20lait");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "unsafe redirect target")]
    fn unsafe_redirects_panic_in_debug() {
        let _ = SeeOther::new("javascript:alert(1)").into_response();
    }

    #[cfg(feature = "axum")]
    #[test]
    fn axum_conversion_keeps_status_and_headers() {
        let res = axum::response::IntoResponse::into_response(
            PageResponse::new("x".into()).status(StatusCode::CONFLICT),
        );
        assert_eq!(res.status(), 409);
        assert_eq!(res.headers()["vary"], "Stucco-Request, Stucco-Target");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    }
}
