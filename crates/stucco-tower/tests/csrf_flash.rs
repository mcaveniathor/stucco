#![cfg(all(feature = "csrf", feature = "axum"))]
use axum::{
    Router,
    body::Body,
    response::Response,
    routing::{any, get, post},
};
use bytes::Bytes;
use http::{Request, StatusCode, header::SET_COOKIE};
use http_body_util::{BodyExt, Full};
use stucco_tower::{
    CsrfLayer, CsrfToken, Flash, FlashLayer, FlashLevel, RequestContext, SecretKey, SeeOther,
    Submission,
};
use tower::{Layer, ServiceExt};

fn key() -> SecretKey {
    SecretKey::new([42; 32])
}
fn csrf() -> CsrfLayer {
    CsrfLayer::new(key(), "https://example.com").unwrap()
}
async fn token(cx: RequestContext) -> String {
    cx.get::<CsrfToken>().unwrap().as_str().to_owned()
}
async fn text(response: Response) -> String {
    String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap()
}
async fn issue(app: &Router) -> (String, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let cookie = response.headers()[SET_COOKIE].to_str().unwrap().to_owned();
    assert!(cookie.contains("; Path=/; HttpOnly; SameSite=Lax; Secure"));
    assert!(cookie.starts_with("__Host-stucco-csrf="));
    (
        cookie.split(';').next().unwrap().to_owned(),
        text(response).await,
    )
}
fn request(method: &str, cookie: &str) -> http::request::Builder {
    Request::builder()
        .method(method)
        .uri("/write")
        .header("cookie", cookie)
        .header(
            "content-type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
}
fn app() -> Router {
    Router::new()
        .route("/token", get(token))
        .route(
            "/write",
            any(|cx: RequestContext, body: Bytes| async move {
                assert!(cx.get::<CsrfToken>().is_some());
                ([(SET_COOKIE, "app=kept")], body)
            }),
        )
        .layer(csrf())
}

#[tokio::test]
async fn form_tokens_round_trip_and_preserve_the_exact_body_and_other_cookies() {
    let app = app();
    let (cookie, token) = issue(&app).await;
    let body = format!("x=one&x=two&encoded=%26%3D+%C3%A9&_csrf={token}");
    let response = app
        .clone()
        .oneshot(
            request("POST", &cookie)
                .header("origin", "https://EXAMPLE.com:443")
                .header("sec-fetch-site", "same-origin")
                .body(Body::from(body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[SET_COOKIE], "app=kept");
    assert_eq!(text(response).await, body);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/token")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(!response.headers().contains_key(SET_COOKIE));
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    assert_eq!(text(response).await, token);
}

#[tokio::test]
async fn missing_wrong_duplicate_and_cross_client_tokens_are_forbidden() {
    let app = app();
    let (cookie, token) = issue(&app).await;
    let (other_cookie, other_token) = issue(&app).await;
    assert_ne!(cookie, other_cookie);
    for (cookie, body) in [
        ("", format!("_csrf={token}")),
        (cookie.as_str(), "x=1".into()),
        (cookie.as_str(), "_csrf=wrong".into()),
        (cookie.as_str(), format!("_csrf={other_token}")),
        (cookie.as_str(), format!("_csrf={token}&_csrf={token}")),
        ("__Host-stucco-csrf=bad", format!("_csrf={token}")),
    ] {
        let response = app
            .clone()
            .oneshot(request("POST", cookie).body(Body::from(body)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(!response.headers().contains_key(SET_COOKIE));
    }
    let response = app
        .oneshot(
            request("POST", &cookie)
                .header("cookie", &other_cookie)
                .body(Body::from(format!("_csrf={token}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn all_unsafe_methods_require_tokens_and_header_bodies_are_not_consumed() {
    let app = app();
    let (cookie, token) = issue(&app).await;
    for method in [
        "POST", "PUT", "PATCH", "DELETE", "PROPFIND", "TRACE", "CONNECT",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/write")
                    .header("cookie", &cookie)
                    .header("stucco-csrf", &token)
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"kept":true}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{method}");
        assert_eq!(
            text(response).await,
            if method == "CONNECT" {
                ""
            } else {
                r#"{"kept":true}"#
            },
            "{method}"
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/write")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{method}");
    }
    for method in ["GET", "HEAD", "OPTIONS"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/write")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn untrusted_origins_fetch_metadata_and_ambiguous_headers_are_forbidden() {
    let app = app();
    let (cookie, token) = issue(&app).await;
    for (header, value) in [
        ("origin", "https://evil.example"),
        ("origin", "https://example.com.evil.test"),
        ("origin", "http://example.com"),
        ("origin", "https://example.com:8443"),
        ("origin", "null"),
        ("origin", "https://example.com https://evil.test"),
        ("sec-fetch-site", "cross-site"),
        ("sec-fetch-site", "same-site"),
        ("sec-fetch-site", "bogus"),
    ] {
        let response = app
            .clone()
            .oneshot(
                request("POST", &cookie)
                    .header(header, value)
                    .body(Body::from(format!("_csrf={token}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            StatusCode::FORBIDDEN,
            "{header}: {value}"
        );
    }
    for header in ["origin", "sec-fetch-site", "stucco-csrf"] {
        let value = match header {
            "origin" => "https://example.com",
            "sec-fetch-site" => "same-origin",
            _ => &token,
        };
        let response = app
            .clone()
            .oneshot(
                request("POST", &cookie)
                    .header(header, value)
                    .header(header, value)
                    .body(Body::from(format!("_csrf={token}")))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
    // A present but bad header must not fall back to a valid form token.
    let response = app
        .oneshot(
            request("POST", &cookie)
                .header("stucco-csrf", "wrong")
                .body(Body::from(format!("_csrf={token}")))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bounded_form_buffering_and_submission_work_together() {
    let app = Router::new()
        .route("/token", get(token))
        .route(
            "/write",
            post(|Submission(form): Submission| async move {
                assert_eq!(form.value("_csrf"), None);
                form.values("x").join(",")
            }),
        )
        .layer(csrf().body_limit(100));
    let (cookie, token) = issue(&app).await;
    let body = format!("x=one&x=two&_csrf={token}");
    let response = app
        .clone()
        .oneshot(request("POST", &cookie).body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(text(response).await, "one,two");
    // No Content-Length: the layer must count actual bytes.
    let body = format!("_csrf={token}&x={}", "x".repeat(100));
    let response = app
        .oneshot(request("POST", &cookie).body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

fn flash_app() -> Router {
    Router::new()
        .route(
            "/save",
            post(|| async {
                SeeOther::new("/done").flash(Flash::new(FlashLevel::Success, "Saved café <order>"))
            }),
        )
        .route(
            "/done",
            get(|cx: RequestContext| async move {
                cx.get::<Flash>()
                    .map(|f| format!("{:?}: {}", f.level, f.message))
                    .unwrap_or_default()
            }),
        )
        .layer(FlashLayer::new(key()))
}

#[tokio::test]
async fn redirect_flash_is_read_once_and_cleared_without_losing_other_cookies() {
    let app = flash_app();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/save")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()["location"], "/done");
    let cookie = response.headers()[SET_COOKIE].to_str().unwrap().to_owned();
    assert!(cookie.contains("; Max-Age=300"));
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/done")
                .header("cookie", cookie.split(';').next().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response.headers()[SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("; Max-Age=0")
    );
    assert_eq!(text(response).await, "Success: Saved café <order>");
    // Model the browser applying the deletion before the next request.
    let response = app
        .oneshot(Request::builder().uri("/done").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert!(!response.headers().contains_key(SET_COOKIE));
    assert_eq!(text(response).await, "");
}

#[tokio::test]
async fn malformed_flash_is_cleared_and_oversized_outgoing_messages_fail_explicitly() {
    let response = flash_app()
        .oneshot(
            Request::builder()
                .uri("/done")
                .header("cookie", "__Host-stucco-flash=bad")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(text(response).await, "");
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                SeeOther::new("/done").flash(Flash::new(FlashLevel::Info, "x".repeat(1025)))
            }),
        )
        .layer(FlashLayer::new(key()));
    let response = app.oneshot(Request::new(Body::empty())).await.unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn both_layers_compose_and_preserve_cookie_headers() {
    let app = flash_app().route("/token", get(token)).layer(csrf());
    let (cookie, token) = issue(&app).await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/save")
                .method("POST")
                .header("cookie", cookie)
                .header("stucco-csrf", token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(
        response.headers()[SET_COOKIE]
            .to_str()
            .unwrap()
            .starts_with("__Host-stucco-flash=")
    );
}

#[tokio::test]
async fn csrf_layer_can_wrap_a_plain_tower_service() {
    let service = tower::service_fn(
        |req: Request<http_body_util::Either<Full<Bytes>, Full<Bytes>>>| async move {
            assert!(req.extensions().get::<CsrfToken>().is_some());
            Ok::<_, std::convert::Infallible>(http::Response::new(Full::new(Bytes::from_static(
                b"ok",
            ))))
        },
    );
    let response = csrf()
        .layer(service)
        .oneshot(Request::new(Full::new(Bytes::new())))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn public_origin_must_be_an_origin() {
    for origin in [
        "",
        "example.com",
        "ftp://example.com",
        "https://user@example.com",
        "https://example.com/path",
        "https://example.com?q=1",
        "https://example.com#fragment",
        "https://example.com:",
        "https://example.com:foo",
        "https://example.com:99999",
    ] {
        assert!(CsrfLayer::new(key(), origin).is_err(), "{origin}");
    }
    assert!(CsrfLayer::new(key(), "http://localhost:3000").is_ok());
}

#[tokio::test]
async fn issuing_both_cookies_appends_to_existing_set_cookie_headers() {
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                let mut response = SeeOther::new("/done")
                    .flash(Flash::new(FlashLevel::Info, "Hello"))
                    .into_http();
                response
                    .headers_mut()
                    .append(SET_COOKIE, "app=existing".parse().unwrap());
                response
            }),
        )
        .layer(FlashLayer::new(key()))
        .layer(csrf());
    let response = app.oneshot(Request::new(Body::empty())).await.unwrap();
    let cookies: Vec<_> = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect();
    assert_eq!(cookies.len(), 3);
    assert!(cookies.contains(&"app=existing"));
    assert!(
        cookies
            .iter()
            .any(|v| v.starts_with("__Host-stucco-flash="))
    );
    assert!(cookies.iter().any(|v| v.starts_with("__Host-stucco-csrf=")));
}

#[tokio::test]
async fn local_http_uses_separate_unprefixed_cookies() {
    let app = Router::new()
        .route(
            "/",
            get(|| async { SeeOther::new("/done").flash(Flash::new(FlashLevel::Info, "Hello")) }),
        )
        .layer(FlashLayer::new(key()).secure(false))
        .layer(csrf().secure(false));
    let response = app.oneshot(Request::new(Body::empty())).await.unwrap();
    let cookies: Vec<_> = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect();
    assert_eq!(cookies.len(), 2);
    assert!(
        cookies
            .iter()
            .all(|v| v.starts_with("stucco-") && !v.contains("Secure"))
    );
}

// A request body with no known length; exercises streaming limits and read errors.
struct Chunks(std::collections::VecDeque<Result<http_body::Frame<Bytes>, std::io::Error>>);
impl http_body::Body for Chunks {
    type Data = Bytes;
    type Error = std::io::Error;
    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Bytes>, Self::Error>>> {
        std::task::Poll::Ready(self.0.pop_front())
    }
}

// A named fn fixes the output type to `()`: an `async { panic!() }` closure's
// output is `!` under edition 2024, which is not a response.
async fn unreachable_handler() {
    panic!("a rejected body must not reach the handler");
}

#[tokio::test]
async fn chunked_forms_obey_limits_and_body_errors_do_not_reach_handlers() {
    let app = Router::new()
        .route("/token", get(token))
        .route("/write", post(unreachable_handler))
        .layer(csrf().body_limit(64));
    let (cookie, token) = issue(&app).await;
    let chunks = Chunks(
        [
            Ok(http_body::Frame::data(Bytes::from(format!(
                "_csrf={token}"
            )))),
            Ok(http_body::Frame::data(Bytes::from_static(
                b"&x=abcdefghijklmnopqrstuvwxyz",
            ))),
        ]
        .into(),
    );
    let response = app
        .clone()
        .oneshot(request("POST", &cookie).body(Body::new(chunks)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let chunks = Chunks([Err(std::io::Error::other("broken request stream"))].into());
    let response = app
        .oneshot(request("POST", &cookie).body(Body::new(chunks)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[cfg(feature = "tower-http")]
#[tokio::test]
async fn outer_body_limit_errors_remain_413() {
    let app = stucco_tower::with_standard_layers(
        app(),
        &stucco_tower::LayerConfig {
            body_limit: 64,
            ..Default::default()
        },
    );
    let (cookie, token) = issue(&app).await;
    let body = format!("_csrf={token}&name={}", "x".repeat(64));
    let response = app
        .oneshot(request("POST", &cookie).body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
