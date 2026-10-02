use http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn html(app: axum::Router, uri: &str) -> String {
    let response = app
        .oneshot(
            Request::builder()
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
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
fn next(html: &str, name: &str) -> String {
    let start = html.find(&format!(">{name}</a>")).unwrap();
    let prefix = &html[..start];
    let href = prefix.rfind("href=\"").unwrap() + 6;
    prefix[href..]
        .split('"')
        .next()
        .unwrap()
        .replace("&amp;", "&")
}
#[tokio::test]
async fn storage_navigation_and_native_get_controls() {
    let dir = tempfile::tempdir().unwrap();
    let app = orders::app(&dir.path().join("orders.redb")).unwrap();
    let first = html(app.clone(), "/orders").await;
    assert_eq!(first.matches("<tbody>").count(), 1);
    assert_eq!(first.matches("<td").count(), 25 * 5);
    let second = html(app.clone(), &next(&first, "Next")).await;
    assert!(second.contains(">26</td>"));
    let back = html(app.clone(), &next(&second, "Previous")).await;
    assert!(back.contains(">1</td>"));
    let filtered = html(
        app.clone(),
        "/orders?q=Ada&f.status=paid&sort=customer&dir=desc",
    )
    .await;
    assert!(filtered.contains("Ada"));
    assert!(filtered.contains(r#"data-value="paid""#));
    assert!(!filtered.contains(r#"data-value="pending""#));
    assert!(filtered.contains("aria-sort=\"descending\""));
    let counted = html(app.clone(), "/orders?mode=pages&per=10&page=2").await;
    assert!(counted.contains("Showing 11–20 of 67"));
    assert!(counted.contains(r#"aria-label="Page 2" aria-current="page""#));
    assert!(counted.contains("mode=pages"));
    let malformed = html(
        app.clone(),
        "/orders?sort=evil&after=garbage&per=0&f.status=unknown&q=%ZZ",
    )
    .await;
    assert!(!malformed.contains("500"));
    let empty = html(app, "/orders?q=NoSuchCustomer").await;
    assert!(empty.contains("No results"));
    assert!(!empty.contains("<tbody>"));
}
