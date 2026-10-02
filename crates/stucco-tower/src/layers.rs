//! The standard middleware stack (integration spec §6, layers 1–2).

use std::time::Duration;

use http::StatusCode;
use tower::ServiceBuilder;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

/// Limits for [`with_standard_layers`].
#[derive(Clone, Debug)]
pub struct LayerConfig {
    /// Largest accepted request body, in bytes (default 1 MiB); larger ones
    /// get `413 Payload Too Large`.
    pub body_limit: usize,
    /// Time allowed to produce a response (default 30 s); slower ones get
    /// `408 Request Timeout`.
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

/// Wraps `router` in the standard stack, outermost first: an
/// `x-request-id` (generated if absent and echoed on the response),
/// tracing, the body limit and the timeout.
///
/// ```
/// use stucco_tower::{LayerConfig, with_standard_layers};
/// let app = with_standard_layers(axum::Router::new(), &LayerConfig::default());
/// # let _ = app;
/// ```
pub fn with_standard_layers(router: axum::Router, config: &LayerConfig) -> axum::Router {
    router.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(PropagateRequestIdLayer::x_request_id())
            .layer(TraceLayer::new_for_http())
            .layer(RequestBodyLimitLayer::new(config.body_limit))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                config.timeout,
            )),
    )
}
