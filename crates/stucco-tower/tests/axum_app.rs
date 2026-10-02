#![cfg(all(feature = "axum", feature = "tower-http"))]

use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::routing::{get, post};
use http::Request;
use stucco_core::Bundle;
use stucco_theme::Preset;
use stucco_tower::{LayerConfig, PageResponse, assets_router, with_standard_layers};
use tower::ServiceExt;

fn app(bundle: Arc<Bundle>, config: &LayerConfig) -> Router {
    let routes = Router::new()
        .route("/", get(|| async { PageResponse::new("ok".into()) }))
        .route(
            "/echo",
            post(|body: String| async move { body.len().to_string() }),
        )
        .route(
            "/slow",
            get(|| async {
                tokio::time::sleep(Duration::from_secs(60)).await;
                "late"
            }),
        );
    with_standard_layers(routes.merge(assets_router(bundle)), config)
}

fn req(method: &str, uri: &str, body: impl Into<Body>) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(body.into())
        .unwrap()
}

#[tokio::test]
async fn responses_carry_a_request_id_and_assets_are_routed() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let res = app(bundle.clone(), &LayerConfig::default())
        .oneshot(req("GET", "/", Body::empty()))
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(res.headers().contains_key("x-request-id"));
    let css = app(bundle.clone(), &LayerConfig::default())
        .oneshot(req("GET", bundle.stylesheet_url(), Body::empty()))
        .await
        .unwrap();
    assert_eq!(css.status(), 200);
    assert_eq!(css.headers()["content-type"], "text/css; charset=utf-8");
}

#[tokio::test]
async fn oversized_bodies_are_413() {
    let config = LayerConfig {
        body_limit: 16,
        ..LayerConfig::default()
    };
    let res = app(Arc::new(Bundle::new(Preset::Slate)), &config)
        .oneshot(req("POST", "/echo", "x".repeat(17)))
        .await
        .unwrap();
    assert_eq!(res.status(), 413);
}

#[tokio::test(start_paused = true)]
async fn slow_handlers_time_out_with_408() {
    let config = LayerConfig {
        timeout: Duration::from_secs(1),
        ..LayerConfig::default()
    };
    let res = app(Arc::new(Bundle::new(Preset::Slate)), &config)
        .oneshot(req("GET", "/slow", Body::empty()))
        .await
        .unwrap();
    assert_eq!(res.status(), 408);
}

#[test]
fn defaults_match_the_spec() {
    let config = LayerConfig::default();
    assert_eq!(config.body_limit, 1024 * 1024);
    assert_eq!(config.timeout, Duration::from_secs(30));
}
