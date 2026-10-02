//! Telling full-page requests from fragment (enhanced) requests, and
//! answering both from one operation.

use bytes::Bytes;
use http::{HeaderMap, Response};
use http_body_util::Full;

use stucco_core::behavior;

use crate::{FragmentResponse, PageResponse};

/// What kind of response a request wants.
///
/// ```
/// use stucco_tower::RequestKind;
/// assert_eq!(RequestKind::from_headers(&http::HeaderMap::new()), RequestKind::Full);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestKind {
    /// A normal navigation or form submission: answer with a full page.
    Full,
    /// An enhanced request: answer with the fragment for element `target`.
    Fragment {
        /// The id of the element being replaced.
        target: String,
    },
}

impl RequestKind {
    /// Reads `Stucco-Request: fragment` (any case) and a valid
    /// `Stucco-Target` id; anything missing or malformed means `Full`.
    pub fn from_headers(headers: &HeaderMap) -> RequestKind {
        let is_fragment = headers
            .get(behavior::HEADER_REQUEST)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("fragment"));
        let target = headers
            .get(behavior::HEADER_TARGET)
            .and_then(|v| v.to_str().ok())
            .filter(|t| valid_target(t));
        match (is_fragment, target) {
            (true, Some(target)) => RequestKind::Fragment {
                target: target.to_owned(),
            },
            _ => RequestKind::Full,
        }
    }

    /// [`RequestKind::from_headers`] on a request's parts.
    pub fn from_parts(parts: &http::request::Parts) -> RequestKind {
        RequestKind::from_headers(&parts.headers)
    }
}

/// Runs exactly one of `full` or `fragment`, chosen by `kind`, so a handler
/// performs its operation once and only picks the output format here.
pub fn respond(
    kind: &RequestKind,
    full: impl FnOnce() -> PageResponse,
    fragment: impl FnOnce(&str) -> FragmentResponse,
) -> Response<Full<Bytes>> {
    match kind {
        RequestKind::Full => full().into_response(),
        RequestKind::Fragment { target } => fragment(target).into_response(),
    }
}

/// An element id: an ASCII letter, then letters, digits, `-` or `_`.
fn valid_target(target: &str) -> bool {
    let mut chars = target.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(feature = "axum")]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for RequestKind {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(RequestKind::from_parts(parts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use stucco_core::Bundle;
    use stucco_theme::Preset;

    fn headers(pairs: &[(&'static str, &[u8])]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, HeaderValue::from_bytes(value).unwrap());
        }
        map
    }

    #[test]
    fn fragment_requests_need_a_valid_target() {
        assert_eq!(
            RequestKind::from_headers(&headers(&[
                ("stucco-request", b"FRAGMENT"),
                ("stucco-target", b"orders")
            ])),
            RequestKind::Fragment {
                target: "orders".into()
            }
        );
        for target in [&b""[..], b"a b", b"x\"y", b"1abc", b"\xff"] {
            assert_eq!(
                RequestKind::from_headers(&headers(&[
                    ("stucco-request", b"fragment"),
                    ("stucco-target", target)
                ])),
                RequestKind::Full,
                "{target:?}"
            );
        }
        assert_eq!(
            RequestKind::from_headers(&headers(&[("stucco-request", b"fragment")])),
            RequestKind::Full
        );
        assert_eq!(
            RequestKind::from_headers(&headers(&[
                ("stucco-request", b"full"),
                ("stucco-target", b"x")
            ])),
            RequestKind::Full
        );
        assert_eq!(
            RequestKind::from_headers(&HeaderMap::new()),
            RequestKind::Full
        );
    }

    #[test]
    fn respond_runs_only_the_matching_branch() {
        let bundle = Bundle::new(Preset::Slate);
        let res = respond(
            &RequestKind::Full,
            || PageResponse::new("page".into()),
            |_| unreachable!(),
        );
        assert_eq!(res.status(), 200);
        assert!(!res.headers().contains_key("cache-control"));
        let kind = RequestKind::Fragment { target: "t".into() };
        let res = respond(
            &kind,
            || unreachable!(),
            |t| FragmentResponse::new(&stucco_core::render_fragment("t", &t.to_owned()), &bundle),
        );
        assert_eq!(res.headers()["cache-control"], "no-store");
    }
}
