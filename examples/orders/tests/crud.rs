use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use std::collections::BTreeMap;
use stucco::server::SecretKey;
use tower::ServiceExt;

/// A cookie-keeping client over the router.
struct Client {
    app: Router,
    cookies: BTreeMap<String, String>,
    _dir: tempfile::TempDir,
}

struct Reply {
    status: StatusCode,
    location: Option<String>,
    body: String,
}

impl Client {
    fn new() -> Client {
        let dir = tempfile::tempdir().unwrap();
        Client::at(dir)
    }

    fn at(dir: tempfile::TempDir) -> Client {
        let app = orders::app(
            &dir.path().join("orders.redb"),
            SecretKey::new([7; 32]),
            "http://localhost",
        )
        .unwrap();
        Client {
            app,
            cookies: BTreeMap::new(),
            _dir: dir,
        }
    }

    async fn send(&mut self, request: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(request).await.unwrap();
        for value in response.headers().get_all(header::SET_COOKIE) {
            let value = value.to_str().unwrap();
            let (pair, attrs) = value.split_once(';').unwrap_or((value, ""));
            let (name, val) = pair.split_once('=').unwrap();
            if val.is_empty() || attrs.to_ascii_lowercase().contains("max-age=0") {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.into(), val.into());
            }
        }
        let status = response.status();
        let location = response
            .headers()
            .get(header::LOCATION)
            .map(|v| v.to_str().unwrap().to_owned());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            location,
            body: String::from_utf8(body.to_vec()).unwrap(),
        }
    }

    fn cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    async fn get(&mut self, uri: &str) -> Reply {
        let request = Request::get(uri)
            .header(header::COOKIE, self.cookie_header())
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    async fn post(&mut self, uri: &str, fields: &[(&str, &str)]) -> Reply {
        let body = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields)
            .finish();
        let request = Request::post(uri)
            .header(header::COOKIE, self.cookie_header())
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();
        self.send(request).await
    }
}

/// The value of the first `<input … name="{name}" value="…">` in `html`.
fn input(html: &str, name: &str) -> String {
    let at = html
        .find(&format!("name=\"{name}\""))
        .unwrap_or_else(|| panic!("no {name} input"));
    let start = html[..at].rfind('<').unwrap();
    let tag = &html[start..start + html[start..].find('>').unwrap()];
    let value = tag.split("value=\"").nth(1).expect("value attribute");
    value[..value.find('"').unwrap()].replace("&amp;", "&")
}

fn order<'a>(csrf: &'a str, customer: &'a str, total: &'a str) -> Vec<(&'a str, &'a str)> {
    vec![
        ("_csrf", csrf),
        ("customer", customer),
        ("status", "paid"),
        ("total", total),
        ("created", "2026-10-01"),
    ]
}

#[tokio::test]
async fn valid_create_redirects_and_flashes() {
    let mut c = Client::new();
    let form = c.get("/orders/new").await;
    assert_eq!(form.status, 200);
    let csrf = input(&form.body, "_csrf");
    let reply = c
        .post("/orders", &order(&csrf, "Mary Jackson", "12.50"))
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.location.as_deref(), Some("/orders"));
    assert!(c.get("/orders").await.body.contains("Order 68 created."));
    assert!(!c.get("/orders").await.body.contains("Order 68 created."));
    let detail = c.get("/orders/68").await;
    assert_eq!(detail.status, 200);
    assert!(detail.body.contains("Mary Jackson"));
    assert!(detail.body.contains("$12.50"));
}

#[tokio::test]
async fn invalid_create_is_422_with_values_and_links() {
    let mut c = Client::new();
    let csrf = input(&c.get("/orders/new").await.body, "_csrf");
    let reply = c.post("/orders", &order(&csrf, "", "abc")).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.body.contains("value=\"abc\""));
    assert!(reply.body.contains("href=\"#order-customer\""));
    assert!(reply.body.contains("href=\"#order-total\""));
    assert!(reply.body.contains("Enter a customer name"));
    assert!(reply.body.contains("Enter an amount like 12.50"));
}

#[tokio::test]
async fn missing_or_wrong_csrf_is_403() {
    let mut c = Client::new();
    c.get("/orders/new").await;
    let mut fields = order("", "Ada", "1");
    fields.remove(0);
    assert_eq!(
        c.post("/orders", &fields).await.status,
        StatusCode::FORBIDDEN
    );
    let reply = c.post("/orders", &order("nope", "Ada", "1")).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rerendered_form_can_be_resubmitted() {
    let mut c = Client::new();
    let csrf = input(&c.get("/orders/new").await.body, "_csrf");
    let failed = c.post("/orders", &order(&csrf, "", "1")).await;
    assert_eq!(failed.status, StatusCode::UNPROCESSABLE_ENTITY);
    let again = input(&failed.body, "_csrf");
    let reply = c.post("/orders", &order(&again, "Ada", "1")).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn total_parsing() {
    let mut c = Client::new();
    let csrf = input(&c.get("/orders/new").await.body, "_csrf");
    for (total, shown) in [("12", "$12.00"), ("12.5", "$12.50"), ("12.50", "$12.50")] {
        let reply = c.post("/orders", &order(&csrf, "Ada", total)).await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{total}");
        let id = c.get("/orders").await.body;
        let id = id
            .split("Order ")
            .nth(1)
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .to_owned();
        assert!(
            c.get(&format!("/orders/{id}")).await.body.contains(shown),
            "{total}"
        );
    }
    for total in ["12.555", "-1", "1e3", ".5", "12."] {
        let reply = c.post("/orders", &order(&csrf, "Ada", total)).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{total}");
    }
}

#[tokio::test]
async fn stale_edit_is_409_and_keeps_values() {
    let mut c = Client::new();
    let edit = c.get("/orders/1/edit").await;
    assert_eq!(edit.status, 200);
    let csrf = input(&edit.body, "_csrf");
    let first = input(&edit.body, "revision");
    let mut fields = order(&csrf, "Ada Byron", "10");
    fields.push(("revision", &first));
    assert_eq!(
        c.post("/orders/1", &fields).await.status,
        StatusCode::SEE_OTHER
    );
    assert!(c.get("/orders").await.body.contains("Order 1 saved."));
    let mut stale = order(&csrf, "Zed", "10");
    stale.push(("revision", &first));
    let reply = c.post("/orders/1", &stale).await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert!(reply.body.contains("Zed"));
    assert!(reply.body.contains("This record has changed"));
    assert_eq!(input(&reply.body, "revision"), first);
    assert!(c.get("/orders/1").await.body.contains("Ada Byron"));
}

#[tokio::test]
async fn malformed_revision_is_conflict() {
    let mut c = Client::new();
    let csrf = input(&c.get("/orders/1/edit").await.body, "_csrf");
    for revision in ["", "%%%", "AAAA"] {
        let mut fields = order(&csrf, "Ada", "10");
        fields.push(("revision", revision));
        let reply = c.post("/orders/1", &fields).await;
        assert_eq!(reply.status, StatusCode::CONFLICT, "{revision:?}");
        let reply = c
            .post(
                "/orders/1/delete",
                &[("_csrf", &csrf), ("revision", revision)],
            )
            .await;
        assert_eq!(reply.status, StatusCode::CONFLICT, "{revision:?}");
    }
}

#[tokio::test]
async fn delete_flow() {
    let mut c = Client::new();
    let confirm = c.get("/orders/2/delete").await;
    assert_eq!(confirm.status, 200);
    let csrf = input(&confirm.body, "_csrf");
    let revision = input(&confirm.body, "revision");
    let fields = [("_csrf", csrf.as_str()), ("revision", revision.as_str())];
    let reply = c.post("/orders/2/delete", &fields).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert!(c.get("/orders").await.body.contains("Order 2 deleted."));
    assert_eq!(c.get("/orders/2").await.status, StatusCode::NOT_FOUND);
    let again = c.post("/orders/2/delete", &fields).await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn deleting_the_newest_order_frees_its_id_without_losing_edits() {
    let mut c = Client::new();
    let edit = c.get("/orders/67/edit").await;
    let csrf = input(&edit.body, "_csrf");
    let old = input(&edit.body, "revision");
    let fields = [("_csrf", csrf.as_str()), ("revision", old.as_str())];
    c.post("/orders/67/delete", &fields).await;
    c.post("/orders", &order(&csrf, "New 67", "1")).await;
    let mut stale = order(&csrf, "Overwrite", "1");
    stale.push(("revision", &old));
    assert_eq!(
        c.post("/orders/67", &stale).await.status,
        StatusCode::CONFLICT
    );
    assert!(c.get("/orders/67").await.body.contains("New 67"));
}

#[tokio::test]
async fn stale_delete_is_409() {
    let mut c = Client::new();
    let confirm = c.get("/orders/3/delete").await;
    let csrf = input(&confirm.body, "_csrf");
    let revision = input(&confirm.body, "revision");
    let edit = c.get("/orders/3/edit").await;
    let mut fields = order(&csrf, "Changed", "3");
    let current = input(&edit.body, "revision");
    fields.push(("revision", &current));
    assert_eq!(
        c.post("/orders/3", &fields).await.status,
        StatusCode::SEE_OTHER
    );
    let reply = c
        .post(
            "/orders/3/delete",
            &[("_csrf", &csrf), ("revision", &revision)],
        )
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert!(reply.body.contains("This record has changed"));
    assert_eq!(c.get("/orders/3").await.status, StatusCode::OK);
}

#[tokio::test]
async fn bad_ids_are_404() {
    let mut c = Client::new();
    for uri in [
        "/orders/abc",
        "/orders/0/edit",
        "/orders/999",
        "/orders/-1/delete",
    ] {
        let reply = c.get(uri).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{uri}");
        assert!(
            reply.body.contains("<!doctype html>") || reply.body.contains("<!DOCTYPE html>"),
            "{uri}"
        );
    }
}

#[tokio::test]
async fn stored_hostile_customer_is_escaped() {
    let mut c = Client::new();
    let csrf = input(&c.get("/orders/new").await.body, "_csrf");
    let hostile = "<script>alert(1)</script>";
    c.post("/orders", &order(&csrf, hostile, "1")).await;
    for uri in [
        "/orders?q=alert",
        "/orders/68",
        "/orders/68/edit",
        "/orders/68/delete",
    ] {
        let body = c.get(uri).await.body;
        assert!(
            body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
            "{uri}"
        );
        assert!(!body.contains(hostile), "{uri}");
    }
}

#[tokio::test]
async fn conflict_pages_focus_one_element() {
    let mut c = Client::new();
    let edit = c.get("/orders/4/edit").await;
    let csrf = input(&edit.body, "_csrf");
    let mut fields = order(&csrf, "Ada", "1");
    fields.push(("revision", "AAAAAAAAAAAAAAAAAAAAAA"));
    let reply = c.post("/orders/4", &fields).await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.body.matches("autofocus").count(), 1);
    let fields = [
        ("_csrf", csrf.as_str()),
        ("revision", "AAAAAAAAAAAAAAAAAAAAAA"),
    ];
    let reply = c.post("/orders/4/delete", &fields).await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.body.matches("autofocus").count(), 1);
}
