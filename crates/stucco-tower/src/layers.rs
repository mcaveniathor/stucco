//! The standard middleware stack (integration spec §6, layers 1–2).

use std::time::Duration;

use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use http::header::X_CONTENT_TYPE_OPTIONS;
use http::{HeaderValue, Request, StatusCode};
use tower::ServiceBuilder;
use tower::util::MapRequestLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Limits for [`with_standard_layers`].
///
/// ```
/// use std::time::Duration;
/// use stucco_tower::LayerConfig;
/// let config = LayerConfig { timeout: Duration::from_secs(10), ..LayerConfig::default() };
/// assert_eq!(config.body_limit, 1024 * 1024);
/// ```
#[derive(Clone, Debug)]
pub struct LayerConfig {
    /// Largest accepted request body, in bytes (default 1 MiB); larger ones
    /// get `413 Payload Too Large`.
    pub body_limit: usize,
    /// Time allowed to produce a response (default 30 s); slower ones get
    /// `503 Service Unavailable` (not 408, which invites clients to retry a
    /// request whose side effects may already have happened).
    pub timeout: Duration,
}

impl Default for LayerConfig {
    fn default() -> Self {
        LayerConfig {
            body_limit: 1024 * 1024,
            timeout: Duration::from_secs(30),
        }
    }
}

const REQUEST_ID: &str = "x-request-id";

/// A request id we are willing to keep from the client: 1–128 characters of
/// `[A-Za-z0-9._-]`. Anything else is replaced with a generated UUID, so the
/// id is safe to show on error pages and in logs.
fn acceptable_request_id(value: &HeaderValue) -> bool {
    let bytes = value.as_bytes();
    (1..=128).contains(&bytes.len())
        && bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// Wraps `router` in the standard stack, outermost first:
///
/// 1. request ids — a well-formed incoming `x-request-id` is kept, anything
///    else replaced by a UUID, and the id is echoed on the response;
/// 2. `X-Content-Type-Options: nosniff` on every response;
/// 3. tracing at INFO, one span per request carrying method, path (not the
///    query) and request id;
/// 4. the body limit (413) and the timeout (503).
///
/// ```
/// use stucco_tower::{LayerConfig, with_standard_layers};
/// let app = with_standard_layers(axum::Router::new(), &LayerConfig::default());
/// # let _ = app;
/// ```
pub fn with_standard_layers(router: axum::Router, config: &LayerConfig) -> axum::Router {
    router.layer(
        ServiceBuilder::new()
            .layer(MapRequestLayer::new(|mut req: Request<Body>| {
                if req
                    .headers()
                    .get(REQUEST_ID)
                    .is_some_and(|v| !acceptable_request_id(v))
                {
                    req.headers_mut().remove(REQUEST_ID);
                }
                req
            }))
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(SetResponseHeaderLayer::if_not_present(
                X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|req: &Request<Body>| {
                        let id = req
                            .headers()
                            .get(REQUEST_ID)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("");
                        tracing::info_span!(
                            "request",
                            method = %req.method(),
                            path = %req.uri().path(),
                            request_id = %id,
                        )
                    })
                    .on_response(DefaultOnResponse::new().level(Level::INFO)),
            )
            .layer(RequestBodyLimitLayer::new(config.body_limit))
            .layer(DefaultBodyLimit::max(config.body_limit))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::SERVICE_UNAVAILABLE,
                config.timeout,
            )),
    )
}
