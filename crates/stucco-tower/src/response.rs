//! HTML responses for full pages and fragments.

use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, VARY, X_CONTENT_TYPE_OPTIONS};
use http::{Response, StatusCode};
use http_body_util::Full;
use stucco_core::{Bundle, RenderedFragment, behavior};

/// A full HTML page response.
///
/// ```
/// use stucco_tower::PageResponse;
/// let res = PageResponse::new("<!doctype html>…".into()).into_response();
/// assert_eq!(res.headers()["vary"], "Stucco-Request");
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

/// An HTML response with the headers every stucco page and fragment carries.
fn html_response(status: StatusCode, html: String, fragment: bool) -> Response<Full<Bytes>> {
    let mut builder = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .header(VARY, behavior::HEADER_REQUEST)
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
impl axum::response::IntoResponse for FragmentResponse {
    fn into_response(self) -> axum::response::Response {
        FragmentResponse::into_response(self).map(axum::body::Body::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_theme::Preset;

    #[test]
    fn page_responses_are_html_and_vary_on_the_request_kind() {
        let res = PageResponse::new("<p>x</p>".into()).into_response();
        assert_eq!(res.status(), 200);
        assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
        assert_eq!(res.headers()["vary"], "Stucco-Request");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
        assert!(!res.headers().contains_key("cache-control"));
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
        assert_eq!(res.headers()["vary"], "Stucco-Request");
        assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
    }

    #[cfg(feature = "axum")]
    #[test]
    fn axum_conversion_keeps_status_and_headers() {
        let res = axum::response::IntoResponse::into_response(
            PageResponse::new("x".into()).status(StatusCode::CONFLICT),
        );
        assert_eq!(res.status(), 409);
        assert_eq!(res.headers()["vary"], "Stucco-Request");
        assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    }
}
