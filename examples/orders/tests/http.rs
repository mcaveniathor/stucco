use axum::body::Body;
use http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Reply {
    fn location(&self) -> &str {
        self.headers["location"].to_str().unwrap()
    }

    /// The flash cookie this reply sets, as a `Cookie` header value.
    fn flash(&self) -> String {
        let cookie = self.headers["set-cookie"].to_str().unwrap();
        cookie.split(';').next().unwrap().to_owned()
    }
}

async fn send(app: &axum::Router, request: Request<Body>) -> Reply {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    Reply {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).unwrap(),
    }
}

async fn get(app: &axum::Router, uri: &str) -> Reply {
    get_with_cookie(app, uri, None).await
}

async fn get_with_cookie(app: &axum::Router, uri: &str, cookie: Option<&str>) -> Reply {
    let mut request = Request::builder().uri(uri);
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    send(app, request.body(Body::empty()).unwrap()).await
}

/// A same-origin form post, as a browser sends it.
async fn post(app: &axum::Router, uri: &str, body: &str) -> Reply {
    send(
        app,
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/x-www-form-urlencoded")
            .header("sec-fetch-site", "same-origin")
            .body(Body::from(body.to_owned()))
            .unwrap(),
    )
    .await
}

async fn html(app: &axum::Router, uri: &str) -> String {
    let reply = get(app, uri).await;
    assert_eq!(reply.status, StatusCode::OK, "{uri}: {}", reply.body);
    reply.body
}

fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let app = orders::app(&dir.path().join("orders.redb")).unwrap();
    (dir, app)
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

/// The form fields of a valid order.
const VALID: &str = "customer=Margaret+Hamilton&status=paid&total=1%2C234.5&created=2026-09-30&note=Leave+at+door&priority=on";

#[tokio::test]
async fn storage_navigation_and_native_get_controls() {
    let (_dir, app) = app();
    let first = html(&app, "/orders").await;
    assert_eq!(first.matches("<tbody>").count(), 1);
    // A checkbox, five data columns and the actions column per row.
    assert_eq!(first.matches("<td").count(), 25 * 7);
    let second = html(&app, &next(&first, "Next")).await;
    assert!(second.contains(r#"<tr data-row-id="26">"#));
    let back = html(&app, &next(&second, "Previous")).await;
    assert!(back.contains(r#"<tr data-row-id="1">"#));
    let filtered = html(&app, "/orders?q=Ada&f.status=paid&sort=customer&dir=desc").await;
    assert!(filtered.contains("Ada"));
    assert!(filtered.contains(r#"data-value="paid""#));
    assert!(!filtered.contains(r#"data-value="pending""#));
    assert!(filtered.contains("aria-sort=\"descending\""));
    assert!(filtered.contains("Remove filter: Status is paid"));
    assert!(filtered.contains("Remove filter: Search “Ada”"));
    let counted = html(&app, "/orders?mode=pages&per=10&page=2").await;
    assert!(counted.contains("Showing 11–20 of 67"));
    assert!(counted.contains(r#"aria-label="Page 2" aria-current="page""#));
    assert!(counted.contains("mode=pages"));
    let malformed = html(
        &app,
        "/orders?sort=evil&after=garbage&per=0&f.status=unknown&q=%ZZ",
    )
    .await;
    assert!(!malformed.contains("500"));
    let empty = html(&app, "/orders?q=NoSuchCustomer").await;
    assert!(empty.contains("No matching orders"), "{empty}");
    assert!(empty.contains("Clear search and filters"));
    assert!(!empty.contains("<tbody>"));
}

#[tokio::test]
async fn records_link_back_to_the_same_list() {
    let (_dir, app) = app();
    let list = html(&app, "/orders?q=Grace&sort=customer&dir=asc&per=10").await;
    let detail = next(&list, "2");
    assert!(detail.starts_with("/orders/2?back="), "{detail}");
    let page = html(&app, &detail).await;
    assert!(page.contains("<h1"));
    assert!(page.contains(r#"<span aria-current="page">Order 2</span>"#));
    let back = next(&page, "Back to orders");
    assert_eq!(back, "/orders?sort=customer&dir=asc&q=Grace&per=10");
    // The edit link and its cancel link carry the same list along.
    let edit = html(&app, &next(&page, "Edit")).await;
    assert_eq!(next(&edit, "Cancel"), detail);
}

#[tokio::test]
async fn a_valid_order_is_created_and_confirmed_after_a_redirect() {
    let (_dir, app) = app();
    let form = html(&app, "/orders/new").await;
    assert!(form.contains(r#"<form class="st-form" method="post" action="/orders">"#));
    assert!(form.contains(r#"name="created""#));
    let reply = post(&app, "/orders", VALID).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.location(), "/orders/68");
    let flash = reply.flash();
    let page = get_with_cookie(&app, "/orders/68", Some(&flash)).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains("Order 68 was created."));
    assert!(page.body.contains(r#"role="status""#));
    assert!(page.body.contains("Margaret Hamilton"));
    assert!(page.body.contains("$1234.50"));
    assert!(page.body.contains("Leave at door"));
    assert!(page.body.contains("created the order"));
    // The page that shows the message deletes it.
    let cleared = page.headers["set-cookie"].to_str().unwrap();
    assert!(cleared.starts_with("stucco-flash=;") && cleared.contains("Max-Age=0"));
    let again = get(&app, "/orders/68").await;
    assert!(!again.body.contains("was created"));
}

#[tokio::test]
async fn an_invalid_order_keeps_what_was_typed_and_links_each_error() {
    let (_dir, app) = app();
    let reply = post(
        &app,
        "/orders",
        "customer=+&status=refunded&total=12.x&created=2026-02-30&note=%3Cb%3Ehi%3C%2Fb%3E",
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let body = &reply.body;
    assert!(body.contains(r#"class="st-error-summary""#));
    for (id, message) in [
        ("order-customer", "Enter the customer’s name"),
        ("order-status", "Choose pending, paid or shipped"),
        ("order-total", "Enter the total as an amount, like 12.50"),
        ("order-created", "Enter a real date, like 2026-09-04"),
    ] {
        assert!(
            body.contains(&format!(r##"<a href="#{id}">{message}</a>"##)),
            "{message}: {body}"
        );
        assert!(body.contains(&format!(r#"id="{id}""#)));
    }
    // The invalid raw input is shown back, escaped.
    assert!(body.contains(r#"value="12.x""#));
    assert!(body.contains(r#"value="2026-02-30""#));
    assert!(body.contains("&lt;b&gt;hi&lt;/b&gt;</textarea>"));
    assert!(body.contains(r#"aria-invalid="true""#));
    // Unchecked boxes send nothing, which is "no", not an error.
    assert!(!body.contains("priority-error"));
    // Nothing was stored.
    assert_eq!(get(&app, "/orders/68").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn repeated_fields_are_refused() {
    let (_dir, app) = app();
    let reply = post(&app, "/orders", &format!("{VALID}&customer=Mallory")).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.body.contains("Enter one customer name"));
}

#[tokio::test]
async fn edits_save_record_history_and_catch_conflicts() {
    let (_dir, app) = app();
    let edit = html(&app, "/orders/5/edit").await;
    assert!(edit.contains(r#"<input type="hidden" name="version" value="1">"#));
    assert!(edit.contains(r#"value="Ada Lovelace""#));
    let saved = post(&app, "/orders/5", &format!("{VALID}&version=1")).await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER);
    assert_eq!(saved.location(), "/orders/5");
    let page = get_with_cookie(&app, "/orders/5", Some(&saved.flash())).await;
    assert!(page.body.contains("Order 5 was saved."));
    assert!(page.body.contains("edited the order"));
    assert!(
        page.body
            .contains("Changed customer, total, date, priority, note")
    );
    // A second tab still holds version 1.
    let stale = post(
        &app,
        "/orders/5",
        "customer=Old+tab&status=pending&total=1&created=2026-09-01&version=1",
    )
    .await;
    assert_eq!(stale.status, StatusCode::CONFLICT);
    assert!(stale.body.contains("Someone else changed this order"));
    assert!(
        stale.body.contains(r#"value="Old tab""#),
        "the user's input is kept"
    );
    assert!(stale.body.contains(r#"name="version" value="2""#));
    // Saving again from that page is a deliberate overwrite.
    let overwrite = post(
        &app,
        "/orders/5",
        "customer=Old+tab&status=pending&total=1&created=2026-09-01&version=2",
    )
    .await;
    assert_eq!(overwrite.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn archiving_hides_an_order_and_restoring_brings_it_back() {
    let (_dir, app) = app();
    let confirm = html(&app, "/orders/3/archive").await;
    assert!(confirm.contains("Archive order 3?"));
    assert!(confirm.contains("Order 3 for Alan Turing"));
    assert!(confirm.contains(r#"action="/orders/3/archive""#));
    let archived = post(&app, "/orders/3/archive", "back=%2Forders%3Fper%3D10").await;
    assert_eq!(archived.location(), "/orders/3?back=%2Forders%3Fper%3D10");
    let page = get_with_cookie(&app, "/orders/3", Some(&archived.flash())).await;
    assert!(page.body.contains("Order 3 was archived."));
    assert!(page.body.contains("This order is archived"));
    assert!(page.body.contains(">Archived</span>"));
    assert!(!page.body.contains(">Edit</a>"));
    let list = html(&app, "/orders").await;
    assert!(!list.contains(r#"data-row-id="3""#));
    let filtered = html(&app, "/orders?f.status=archived").await;
    assert!(filtered.contains(r#"data-row-id="3""#));
    // Archived orders are read-only, whichever way the edit arrives.
    let edit = get(&app, "/orders/3/edit").await;
    assert_eq!(edit.status, StatusCode::SEE_OTHER);
    let forced = post(&app, "/orders/3", &format!("{VALID}&version=2")).await;
    assert_eq!(forced.status, StatusCode::SEE_OTHER);
    assert!(forced.flash().contains("Restore"));
    let restored = post(&app, "/orders/3/restore", "").await;
    assert!(restored.flash().contains("restored"));
    let page = html(&app, "/orders/3").await;
    assert!(
        page.contains(">Shipped</span>"),
        "the old status comes back"
    );
    assert!(page.contains("restored the order") && page.contains("archived the order"));
}

#[tokio::test]
async fn deleting_asks_first_and_returns_to_the_list() {
    let (_dir, app) = app();
    let confirm = html(&app, "/orders/7/delete?back=%2Forders%3Fq%3DAda").await;
    assert!(confirm.contains("Delete order 7 permanently?"));
    assert!(confirm.contains(r#"data-variant="danger""#));
    assert!(confirm.contains("archive it instead"));
    let deleted = post(&app, "/orders/7/delete", "back=%2Forders%3Fq%3DAda").await;
    assert_eq!(deleted.location(), "/orders?q=Ada");
    let list = get_with_cookie(&app, "/orders?q=Ada", Some(&deleted.flash())).await;
    assert!(list.body.contains("Order 7 was deleted."));
    assert_eq!(get(&app, "/orders/7").await.status, StatusCode::NOT_FOUND);
    // A second tab deleting again: not an error.
    let again = post(&app, "/orders/7/delete", "back=%2Forders%2F7").await;
    assert_eq!(
        again.location(),
        "/orders",
        "never back to the deleted order"
    );
    assert!(again.flash().contains("already"));
}

#[tokio::test]
async fn bulk_actions_name_their_scope_and_recheck_each_order() {
    let (_dir, app) = app();
    let list = html(&app, "/orders").await;
    assert!(
        list.contains(
            r#"<form class="st-form orders-bulk" method="get" action="/orders/bulk" id="bulk">"#
        ) || list.contains(r#"id="bulk""#)
    );
    assert!(list.contains(r#"form="bulk" name="id" value="1" aria-label="Select order 1""#));
    let confirm = html(
        &app,
        "/orders/bulk?action=delete&id=1&id=2&id=999&id=2&back=%2Forders",
    )
    .await;
    assert!(
        confirm.contains("Delete 2 orders permanently?"),
        "{confirm}"
    );
    assert!(confirm.contains("1 of the selected orders no longer exists"));
    assert!(confirm.contains(
        r#"<input type="hidden" name="id" value="1"><input type="hidden" name="id" value="2">"#
    ));
    let done = post(
        &app,
        "/orders/bulk",
        "action=archive&id=1&id=2&back=%2Forders",
    )
    .await;
    assert_eq!(done.location(), "/orders");
    assert!(
        done.flash().contains("Archived+2+orders"),
        "{}",
        done.flash()
    );
    let partly = post(&app, "/orders/bulk", "action=archive&id=1&id=4").await;
    assert!(partly.flash().starts_with("stucco-flash=warning."));
    let none = get(&app, "/orders/bulk?action=archive&back=%2Forders").await;
    assert_eq!(none.status, StatusCode::SEE_OTHER);
    assert!(none.flash().contains("Select+the+orders"));
}

#[tokio::test]
async fn changes_need_a_same_origin_post() {
    let (_dir, app) = app();
    // GET never changes anything: it shows a confirmation, or isn't allowed.
    assert_eq!(
        get(&app, "/orders/1/restore").await.status,
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(get(&app, "/orders/1/delete").await.status, StatusCode::OK);
    assert_eq!(get(&app, "/orders/1").await.status, StatusCode::OK);
    let cross = |site: Option<&str>, origin: Option<&str>| {
        let mut request = Request::builder()
            .method("POST")
            .uri("/orders/1/delete")
            .header("host", "orders.example")
            .header("content-type", "application/x-www-form-urlencoded");
        if let Some(site) = site {
            request = request.header("sec-fetch-site", site);
        }
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        request.body(Body::from("back=%2Forders")).unwrap()
    };
    for (site, origin) in [
        (Some("cross-site"), None),
        (Some("same-site"), Some("https://orders.example")),
        (None, Some("https://evil.example")),
        (None, Some("null")),
    ] {
        let reply = send(&app, cross(site, origin)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{site:?} {origin:?}");
    }
    assert_eq!(
        get(&app, "/orders/1").await.status,
        StatusCode::OK,
        "still there"
    );
    let same = send(&app, cross(None, Some("https://orders.example"))).await;
    assert_eq!(same.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn return_locations_are_checked_and_missing_orders_are_404() {
    let (_dir, app) = app();
    let created = post(
        &app,
        "/orders",
        &format!("{VALID}&back=https%3A%2F%2Fevil.example"),
    )
    .await;
    assert_eq!(created.location(), "/orders/68");
    let page = html(&app, "/orders/1?back=%2F%2Fevil.example").await;
    assert!(!page.contains("evil.example"));
    for uri in [
        "/orders/999",
        "/orders/abc",
        "/orders/999/edit",
        "/orders/999/delete",
    ] {
        let reply = get(&app, uri).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{uri}");
        assert!(reply.body.contains("Back to orders"));
    }
    let missing = post(&app, "/orders/999", VALID).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}
