#![cfg(all(feature = "axum", feature = "tower-http"))]

use axum::Router;
use axum::body::Body;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use stucco_core::Validator;
use stucco_tower::{LayerConfig, PageResponse, SeeOther, Submission, with_standard_layers};
use tower::ServiceExt;

async fn create(Submission(form): Submission) -> Response {
    let mut v = Validator::new(form);
    let name = v.text("name").required("Enter a name").get();
    let qty = v
        .text("qty")
        .required("Enter a quantity")
        .parse::<u32>("Enter a whole number")
        .get();
    match v.finish(|| Some((name?, qty?))) {
        Ok((name, qty)) => {
            IntoResponse::into_response(SeeOther::new(format!("/items/{name}?qty={qty}")))
        }
        Err(state) => {
            let errors: Vec<String> = state
                .field_errors()
                .map(|(field, messages)| format!("{field}: {}", messages.join(", ")))
                .collect();
            let kept = state.value("name").unwrap_or_default().to_owned();
            IntoResponse::into_response(
                PageResponse::new(format!("{} | kept={kept}", errors.join("; ")))
                    .status(StatusCode::UNPROCESSABLE_ENTITY),
            )
        }
    }
}

fn app(body_limit: usize) -> Router {
    let config = LayerConfig {
        body_limit,
        ..LayerConfig::default()
    };
    with_standard_layers(Router::new().route("/items", post(create)), &config)
}

async fn submit(content_type: &str, body: &str, body_limit: usize) -> (StatusCode, String, String) {
    let res = app(body_limit)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/items")
                .header("content-type", content_type)
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let location = res
        .headers()
        .get("location")
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default();
    let body = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        location,
        String::from_utf8_lossy(&body).into_owned(),
    )
}

const FORM: &str = "application/x-www-form-urlencoded";

#[tokio::test]
async fn a_valid_submission_redirects_with_303() {
    let (status, location, _) = submit(FORM, "name=bolt&qty=3&_csrf=t", 1024).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location, "/items/bolt?qty=3");
}

#[tokio::test]
async fn an_invalid_submission_keeps_values_and_reports_errors() {
    let (status, _, body) = submit(
        "application/x-www-form-urlencoded; charset=UTF-8",
        "name=+nut+&qty=many",
        1024,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body, "qty: Enter a whole number | kept= nut ");
}

#[tokio::test]
async fn other_content_types_are_415() {
    let (status, _, _) = submit("application/json", r#"{"name":"bolt"}"#, 1024).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let (status, _, _) = submit("multipart/form-data; boundary=x", "", 1024).await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn oversized_submissions_are_413() {
    let body = format!("name={}&qty=1", "x".repeat(64));
    let (status, _, _) = submit(FORM, &body, 16).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}
