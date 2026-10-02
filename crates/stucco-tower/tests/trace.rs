#![cfg(all(feature = "axum", feature = "tower-http"))]
//! Tracing output, in its own test binary: `tracing` caches callsite interest
//! process-wide, so sharing a process with subscriber-less tests is flaky.

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::routing::get;
use http::Request;
use stucco_core::Bundle;
use stucco_theme::Preset;
use stucco_tower::{LayerConfig, PageResponse, with_standard_layers};
use tower::ServiceExt;

fn app(_bundle: Arc<Bundle>, config: &LayerConfig) -> Router {
    let routes = Router::new().route("/", get(|| async { PageResponse::new("ok".into()) }));
    with_standard_layers(routes, config)
}

#[tokio::test]
async fn the_trace_span_carries_the_request_id_at_info_level() {
    use std::sync::{Arc as StdArc, Mutex};
    #[derive(Clone, Default)]
    struct Buf(StdArc<Mutex<Vec<u8>>>);
    impl std::io::Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Buf::default();
    let writer = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let res = app(
        Arc::new(Bundle::new(Preset::Slate)),
        &LayerConfig::default(),
    )
    .oneshot(
        Request::builder()
            .uri("/?secret=1")
            .header("x-request-id", "trace-me-42")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(res.status(), 200);
    let log = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    assert!(log.contains("trace-me-42"), "{log}");
    assert!(
        !log.contains("secret"),
        "query strings stay out of logs: {log}"
    );
}
