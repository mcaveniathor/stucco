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

#[tokio::test]
async fn configured_body_limit_can_exceed_axums_default() {
    let config = LayerConfig {
        body_limit: 4 * 1024 * 1024,
        ..LayerConfig::default()
    };
    let router = app(Arc::new(Bundle::new(Preset::Slate)), &config);
    for size in [3 * 1024 * 1024, config.body_limit] {
        let res = router
            .clone()
            .oneshot(req("POST", "/echo", "x".repeat(size)))
            .await
            .unwrap();
        assert_eq!(res.status(), 200, "{size} bytes should be accepted");
        let body = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.as_ref(), size.to_string().as_bytes());
    }
    let res = router
        .oneshot(req("POST", "/echo", "x".repeat(config.body_limit + 1)))
        .await
        .unwrap();
    assert_eq!(res.status(), 413);
}

#[tokio::test(start_paused = true)]
async fn slow_handlers_time_out_with_503() {
    // 408 would invite clients and proxies to retry a possibly-committed
    // mutation; 503 says the server, not the client, ran out of time.
    let config = LayerConfig {
        timeout: Duration::from_secs(1),
        ..LayerConfig::default()
    };
    let res = app(Arc::new(Bundle::new(Preset::Slate)), &config)
        .oneshot(req("GET", "/slow", Body::empty()))
        .await
        .unwrap();
    assert_eq!(res.status(), 503);
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
}

#[tokio::test]
async fn every_response_is_nosniff() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let config = LayerConfig {
        body_limit: 4,
        ..LayerConfig::default()
    };
    for (method, uri, body) in [
        ("GET", "/no-such-route", String::new()),
        ("POST", "/echo", "too long".to_owned()),
        ("DELETE", "/", String::new()),
    ] {
        let res = app(bundle.clone(), &config)
            .oneshot(req(method, uri, body))
            .await
            .unwrap();
        assert_eq!(
            res.headers()["x-content-type-options"],
            "nosniff",
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn well_formed_request_ids_are_kept_and_others_replaced() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let with_id = |id: &str| {
        Request::builder()
            .uri("/")
            .header("x-request-id", id)
            .body(Body::empty())
            .unwrap()
    };
    let kept = app(bundle.clone(), &LayerConfig::default())
        .oneshot(with_id("abc-123.X_y"))
        .await
        .unwrap();
    assert_eq!(kept.headers()["x-request-id"], "abc-123.X_y");
    let replaced = app(bundle, &LayerConfig::default())
        .oneshot(with_id("evil<script>"))
        .await
        .unwrap();
    let id = replaced.headers()["x-request-id"].to_str().unwrap();
    assert!(!id.contains('<') && !id.is_empty(), "{id}");
}

#[test]
fn defaults_match_the_spec() {
    let config = LayerConfig::default();
    assert_eq!(config.body_limit, 1024 * 1024);
    assert_eq!(config.timeout, Duration::from_secs(30));
}
