#![cfg(feature = "ssr")]

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use leptos::{config::LeptosOptions, server_fn::ServerFn};
use orders_fullstack::sessions::Sessions;
use orders_fullstack::{
    api::{CreateOrder, DeleteOrder, ListOrders, LoadDelete, LoadOrder, SaveOrder},
    auth::{Principal, PrincipalKey},
    store::OrderStore,
};
use std::{collections::BTreeMap, sync::Arc};
use stucco_tower::SecretKey;
use tower::ServiceExt;
use tower_sessions::SessionStore;

const ORIGIN: &str = "https://orders.example.com";

/// Requests traverse the public router and its real CSRF and rendering layers.
struct Client {
    app: Router,
    cookies: BTreeMap<String, String>,
    expected: PrincipalKey,
    store: OrderStore,
    directory: Arc<tempfile::TempDir>,
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    html: String,
}

impl Client {
    async fn open(path: &std::path::Path) -> OrderStore {
        #[cfg(feature = "sqlite")]
        {
            OrderStore::open_sqlite(path).await.unwrap()
        }
        #[cfg(not(feature = "sqlite"))]
        {
            OrderStore::open(path).await.unwrap()
        }
    }
    async fn router(store: OrderStore) -> Router {
        let options = LeptosOptions::builder()
            .output_name("orders-fullstack")
            .site_root("target/orders-fullstack-site")
            .build();
        orders_fullstack::server::app(options, SecretKey::new([17; 32]), ORIGIN, store, 3600)
            .await
            .unwrap()
    }

    async fn anonymous() -> Self {
        let directory = Arc::new(tempfile::tempdir().unwrap());
        let store = Self::open(&directory.path().join("orders.testdb")).await;
        let alice = orders_fullstack::auth::add_user(
            &store,
            "team-a",
            "alice",
            "Alice",
            "test-password-123".into(),
        )
        .await
        .unwrap();
        store.seed_orders(&alice).await.unwrap();
        Self {
            app: Self::router(store.clone()).await,
            cookies: BTreeMap::new(),
            expected: alice.key,
            store,
            directory,
        }
    }

    async fn new() -> Self {
        let mut client = Self::anonymous().await;
        let reply = client.login("alice", "test-password-123", "/orders").await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER);
        client
    }

    fn peer(&self) -> Self {
        Self {
            app: self.app.clone(),
            cookies: BTreeMap::new(),
            expected: self.expected,
            store: self.store.clone(),
            directory: self.directory.clone(),
        }
    }

    async fn login(&mut self, username: &str, password: &str, return_to: &str) -> Reply {
        let page = self.get("/login").await;
        let csrf = input(&page.html, "_csrf");
        let reply = self
            .post_to(
                "/login",
                &[
                    ("username", username),
                    ("password", password),
                    ("_csrf", &csrf),
                    ("return_to", return_to),
                ],
                ORIGIN,
            )
            .await;
        if reply.status == StatusCode::SEE_OTHER {
            self.expected = orders_fullstack::auth::principal_for_username(&self.store, username)
                .await
                .unwrap()
                .key;
        }
        reply
    }

    async fn add_bob(&self) -> Principal {
        orders_fullstack::auth::add_user(
            &self.store,
            "team-b",
            "bob",
            "Bob",
            "other-password-456".into(),
        )
        .await
        .unwrap()
    }

    async fn expire_sessions(&self) {
        let sessions = Sessions::new(&self.store).await.unwrap();
        let id = self.cookies["__Host-orders-session"].parse().unwrap();
        let mut record = sessions.load(&id).await.unwrap().unwrap();
        record.expiry_date = time::OffsetDateTime::UNIX_EPOCH;
        sessions.save(&record).await.unwrap();
    }

    async fn send(&mut self, mut request: Request<Body>) -> Reply {
        if !self.cookies.is_empty() {
            let cookie = self
                .cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("; ");
            request
                .headers_mut()
                .insert(header::COOKIE, cookie.parse().unwrap());
        }
        let response = self.app.clone().oneshot(request).await.unwrap();
        for cookie in response.headers().get_all(header::SET_COOKIE) {
            let cookie = cookie.to_str().unwrap();
            let (name, value) = cookie.split(';').next().unwrap().split_once('=').unwrap();
            if value.is_empty() || cookie.contains("Max-Age=0") {
                self.cookies.remove(name);
            } else {
                self.cookies.insert(name.to_owned(), value.to_owned());
            }
        }
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            headers,
            html: String::from_utf8(bytes.to_vec()).unwrap(),
        }
    }

    async fn get(&mut self, path: &str) -> Reply {
        self.send(
            Request::get(path)
                .header(header::ACCEPT, "text/html")
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn get_api(&mut self, path: &str, parameters: &[(&str, &str)]) -> Reply {
        let user = self.expected.user_id.to_string();
        let tenant = self.expected.tenant_id.to_string();
        let generation = self.expected.generation.to_string();
        let mut parameters = parameters.to_vec();
        if !parameters
            .iter()
            .any(|(name, _)| *name == "expected_user_id")
        {
            parameters.push(("expected_user_id", &user));
            parameters.push(("expected_tenant_id", &tenant));
        }
        if !parameters
            .iter()
            .any(|(name, _)| *name == "expected_generation")
        {
            parameters.push(("expected_generation", &generation));
        }
        let query = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(parameters)
            .finish();
        self.send(
            Request::get(format!("{path}?{query}"))
                .header(header::ACCEPT, "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn post(&mut self, fields: &[(&str, &str)], origin: &str) -> Reply {
        self.post_to("/orders/1/edit", fields, origin).await
    }

    async fn post_to(&mut self, path: &str, fields: &[(&str, &str)], origin: &str) -> Reply {
        self.post_as(path, fields, origin, "text/html").await
    }

    async fn post_api(&mut self, path: &str, fields: &[(&str, &str)], origin: &str) -> Reply {
        self.post_as(path, fields, origin, "application/json").await
    }

    async fn post_as(
        &mut self,
        path: &str,
        fields: &[(&str, &str)],
        origin: &str,
        accept: &str,
    ) -> Reply {
        let user = self.expected.user_id.to_string();
        let tenant = self.expected.tenant_id.to_string();
        let generation = self.expected.generation.to_string();
        let mut fields = fields.to_vec();
        if (path.starts_with("/orders")
            || [SaveOrder::PATH, CreateOrder::PATH, DeleteOrder::PATH].contains(&path))
            && !fields.iter().any(|(name, _)| *name == "expected_user_id")
        {
            fields.push(("expected_user_id", &user));
            fields.push(("expected_tenant_id", &tenant));
        }
        if (path.starts_with("/orders")
            || [SaveOrder::PATH, CreateOrder::PATH, DeleteOrder::PATH].contains(&path))
            && !fields
                .iter()
                .any(|(name, _)| *name == "expected_generation")
        {
            fields.push(("expected_generation", &generation));
        }
        let encoded = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields)
            .finish();
        self.send(
            Request::post(path)
                .header(header::ACCEPT, accept)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .header(header::ORIGIN, origin)
                .body(Body::from(encoded))
                .unwrap(),
        )
        .await
    }
}

fn fields<'a>(
    csrf: &'a str,
    customer: &'a str,
    quantity_one: &'a str,
    quantity_two: &'a str,
    revision: &'a str,
) -> Vec<(&'a str, &'a str)> {
    vec![
        ("_csrf", csrf),
        ("customer", customer),
        ("quantity_one", quantity_one),
        ("quantity_two", quantity_two),
        ("expected_revision", revision),
    ]
}

/// Read rendered native form values without depending on attribute order.
fn input(html: &str, name: &str) -> String {
    let name = format!("name=\"{name}\"");
    let tag = html
        .split("<input")
        .skip(1)
        .map(|tail| tail.split('>').next().unwrap())
        .find(|tag| tag.contains(&name))
        .unwrap_or_else(|| panic!("missing input {name}"));
    let value = tag
        .split_once("value=\"")
        .expect("native input needs a value attribute")
        .1
        .split('"')
        .next()
        .unwrap();
    value
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[tokio::test]
async fn get_renders_the_complete_native_editor_and_issues_csrf_protection() {
    let mut client = Client::new().await;
    client.cookies.remove("__Host-stucco-csrf");
    let reply = client.get("/orders/1/edit").await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert_eq!(reply.headers[header::CACHE_CONTROL], "private, no-store");
    let cookie = reply.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.starts_with("__Host-stucco-csrf="));
    for attribute in ["Path=/", "HttpOnly", "SameSite=Lax", "Secure"] {
        assert!(cookie.contains(attribute), "missing {attribute}");
    }
    for content in [
        "<!DOCTYPE html>",
        "Notebook",
        "Desk organizer",
        "$59.00",
        "action=\"/orders/1/edit\"",
        "method=\"post\"",
        "Save order",
    ] {
        assert!(
            reply.html.contains(content),
            "missing SSR content {content}"
        );
    }
    assert_eq!(input(&reply.html, "customer"), "Ada Lovelace");
    assert_eq!(input(&reply.html, "quantity_one"), "2");
    assert_eq!(input(&reply.html, "quantity_two"), "1");
    assert_eq!(input(&reply.html, "expected_revision"), "1");
    assert!(!input(&reply.html, "_csrf").is_empty());
}

#[tokio::test]
async fn missing_wrong_and_cross_origin_tokens_cannot_mutate_the_order() {
    let mut client = Client::new().await;
    let page = client.get("/orders/1/edit").await;
    let csrf = input(&page.html, "_csrf");
    let valid = fields(&csrf, "Rejected edit", "3", "2", "1");
    let without_token: Vec<_> = valid
        .iter()
        .copied()
        .filter(|(name, _)| *name != "_csrf")
        .collect();
    assert_eq!(
        client.post(&without_token, ORIGIN).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&fields("wrong", "Rejected edit", "3", "2", "1"), ORIGIN)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&valid, "https://elsewhere.example.com")
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    client.cookies.remove("__Host-stucco-csrf");
    assert_eq!(
        client.post(&valid, ORIGIN).await.status,
        StatusCode::FORBIDDEN
    );

    let unchanged = client.get("/orders/1/edit").await;
    assert_eq!(input(&unchanged.html, "customer"), "Ada Lovelace");
    assert_eq!(input(&unchanged.html, "quantity_one"), "2");
    assert_eq!(input(&unchanged.html, "expected_revision"), "1");
}

#[tokio::test]
async fn valid_native_save_redirects_and_the_next_get_reads_the_saved_order() {
    let mut client = Client::new().await;
    let page = client.get("/orders/1/edit").await;
    let csrf = input(&page.html, "_csrf");
    let reply = client
        .post(&fields(&csrf, "Grace Hopper", "3", "2", "1"), ORIGIN)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.headers[header::LOCATION], "/orders/1");
    assert_eq!(reply.headers[header::CACHE_CONTROL], "private, no-store");

    let saved = client.get("/orders/1/edit").await;
    assert_eq!(saved.status, StatusCode::OK);
    assert_eq!(input(&saved.html, "customer"), "Grace Hopper");
    assert_eq!(input(&saved.html, "quantity_one"), "3");
    assert_eq!(input(&saved.html, "quantity_two"), "2");
    assert_eq!(input(&saved.html, "expected_revision"), "2");
    assert!(saved.html.contains("$106.00"));
}

#[tokio::test]
async fn invalid_native_save_returns_422_with_exact_text_and_original_revision() {
    let mut client = Client::new().await;
    let page = client.get("/orders/1/edit").await;
    let csrf = input(&page.html, "_csrf");
    let reply = client
        .post(
            &fields(&csrf, "  Unsent customer  ", "several", "0", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(input(&reply.html, "customer"), "  Unsent customer  ");
    assert_eq!(input(&reply.html, "quantity_one"), "several");
    assert_eq!(input(&reply.html, "quantity_two"), "0");
    assert_eq!(input(&reply.html, "expected_revision"), "1");
    assert_eq!(input(&reply.html, "_csrf"), csrf);
    assert!(reply.html.contains("Enter a whole quantity from 1 to 999"));
    assert!(reply.html.contains("aria-invalid=\"true\""));

    let unchanged = client.get("/orders/1/edit").await;
    assert_eq!(input(&unchanged.html, "customer"), "Ada Lovelace");
    assert_eq!(input(&unchanged.html, "quantity_one"), "2");
    assert_eq!(input(&unchanged.html, "expected_revision"), "1");
}

#[tokio::test]
async fn stale_native_draft_returns_409_and_cannot_adopt_a_new_revision_silently() {
    let mut client = Client::new().await;
    let first = client.get("/orders/1/edit").await;
    let second = client.get("/orders/1/edit").await;
    let first_revision = input(&first.html, "expected_revision");
    let second_revision = input(&second.html, "expected_revision");
    let csrf = input(&first.html, "_csrf");
    assert_eq!(first_revision, second_revision);
    assert_eq!(
        client
            .post(
                &fields(&csrf, "First save", "3", "1", &first_revision),
                ORIGIN
            )
            .await
            .status,
        StatusCode::SEE_OTHER
    );

    let stale = fields(&csrf, "My unsent edit", "7", "4", &second_revision);
    let conflict = client.post(&stale, ORIGIN).await;
    assert_eq!(conflict.status, StatusCode::CONFLICT);
    assert_eq!(input(&conflict.html, "customer"), "My unsent edit");
    assert_eq!(input(&conflict.html, "quantity_one"), "7");
    assert_eq!(input(&conflict.html, "quantity_two"), "4");
    assert_eq!(input(&conflict.html, "expected_revision"), second_revision);
    assert!(conflict.html.contains("changed to revision 2"));
    assert_eq!(
        client.post(&stale, ORIGIN).await.status,
        StatusCode::CONFLICT
    );

    let saved = client.get("/orders/1/edit").await;
    assert_eq!(input(&saved.html, "customer"), "First save");
    assert_eq!(input(&saved.html, "quantity_one"), "3");
    assert_eq!(input(&saved.html, "expected_revision"), "2");
}

#[tokio::test]
async fn hostile_submitted_text_is_escaped_in_form_attributes_and_hydration_state() {
    let mut client = Client::new().await;
    let page = client.get("/orders/1/edit").await;
    let csrf = input(&page.html, "_csrf");
    let hostile = "</script><script>window.__stucco_xss=1</script>\"&";
    let reply = client
        .post(&fields(&csrf, hostile, "invalid", "1", "1"), ORIGIN)
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(input(&reply.html, "customer"), hostile);
    assert!(!reply.html.contains(hostile));
    assert!(
        !reply
            .html
            .contains("<script>window.__stucco_xss=1</script>")
    );
    assert!(reply.html.contains("&lt;/script&gt;"));

    let serialized = reply
        .html
        .split("<script")
        .skip(1)
        .filter_map(|tail| tail.split_once('>').map(|(_, body)| body))
        .map(|body| body.split("</script>").next().unwrap())
        .find(|body| body.contains("window.__stucco_xss=1"))
        .expect("the retained draft must also appear in serialized hydration state");
    assert!(serialized.contains("\\u003c/script>"));
    assert!(!serialized.contains("<script>"));

    let unchanged = client.get("/orders/1/edit").await;
    assert_eq!(input(&unchanged.html, "customer"), "Ada Lovelace");
    assert_eq!(input(&unchanged.html, "expected_revision"), "1");
}

#[tokio::test]
async fn unknown_routes_are_not_rendered_as_the_order_editor() {
    let mut client = Client::new().await;
    let reply = client.get("/not-an-order").await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(!reply.html.contains("name=\"customer\""));
}

fn order_links(html: &str) -> Vec<u64> {
    html.split("href=\"")
        .skip(1)
        .filter_map(|tail| tail.split('"').next())
        .filter_map(|path| path.strip_prefix("/orders/"))
        .filter_map(|id| id.parse().ok())
        .collect()
}

fn assert_missing_order(reply: &Reply) {
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(reply.html.contains("Order not found"));
    for absent in [
        "action=\"/orders",
        "name=\"expected_revision\"",
        "name=\"customer\"",
        "Delete order #",
        "unit_price_cents",
        "$47.00",
    ] {
        assert!(
            !reply.html.contains(absent),
            "missing order response contains {absent}"
        );
    }
}

#[tokio::test]
async fn collection_search_and_pagination_work_as_native_get_requests() {
    let mut client = Client::new().await;
    let home = client.get("/").await;
    assert!(home.status.is_redirection());
    assert_eq!(home.headers[header::LOCATION], "/orders");

    let first = client.get("/orders").await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(order_links(&first.html), [12, 11, 10, 9, 8]);
    let second = client.get("/orders?page=2").await;
    assert_eq!(second.status, StatusCode::OK);
    assert_eq!(order_links(&second.html), [7, 6, 5, 4, 3]);
    let last = client.get("/orders?page=4294967295").await;
    assert_eq!(last.status, StatusCode::OK);
    assert_eq!(order_links(&last.html), [2, 1]);
    let found = client.get("/orders?q=hOpPeR&page=2").await;
    assert_eq!(found.status, StatusCode::OK);
    assert_eq!(order_links(&found.html), [2]);
    assert_eq!(input(&found.html, "q"), "hOpPeR");
    let empty = client.get("/orders?q=no-such-customer").await;
    assert_eq!(empty.status, StatusCode::OK);
    assert!(order_links(&empty.html).is_empty());
    assert_eq!(input(&empty.html, "q"), "no-such-customer");
}

#[tokio::test]
async fn native_create_detail_edit_and_confirmed_delete_form_one_complete_workflow() {
    let mut client = Client::new().await;
    let new = client.get("/orders/new").await;
    assert_eq!(new.status, StatusCode::OK);
    assert_eq!(input(&new.html, "expected_revision"), "1");
    assert_eq!(input(&new.html, "quantity_one"), "1");
    let csrf = input(&new.html, "_csrf");
    let invalid = client
        .post_to(
            "/orders/new",
            &fields(&csrf, "Native new order", "1.5", "2", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(invalid.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(input(&invalid.html, "customer"), "Native new order");
    assert_eq!(input(&invalid.html, "quantity_one"), "1.5");
    assert_eq!(input(&invalid.html, "expected_revision"), "1");

    let created = client
        .post_to(
            "/orders/new",
            &fields(&csrf, "Native new order", "3", "2", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    assert_eq!(created.headers[header::LOCATION], "/orders/13");
    let detail = client.get("/orders/13").await;
    assert_eq!(detail.status, StatusCode::OK);
    assert!(detail.html.contains("Native new order"));
    assert!(detail.html.contains("$106.00"));
    assert!(detail.html.contains("href=\"/orders/13/edit\""));
    let deletion_before_edit = client.get("/orders/13/delete").await;
    assert_eq!(deletion_before_edit.status, StatusCode::OK);
    let old_revision = input(&deletion_before_edit.html, "expected_revision");

    let edit = client.get("/orders/13/edit").await;
    assert_eq!(input(&edit.html, "customer"), "Native new order");
    let saved = client
        .post_to(
            "/orders/13/edit",
            &fields(&csrf, "Native revised order", "4", "2", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER);
    assert_eq!(saved.headers[header::LOCATION], "/orders/13");
    let detail = client.get("/orders/13").await;
    assert_eq!(detail.status, StatusCode::OK);
    assert!(detail.html.contains("Native revised order"));
    assert!(detail.html.contains("$118.00"));

    let stale_delete = [
        ("_csrf", csrf.as_str()),
        ("expected_revision", old_revision.as_str()),
    ];
    let conflict = client
        .post_to("/orders/13/delete", &stale_delete, ORIGIN)
        .await;
    assert_eq!(conflict.status, StatusCode::CONFLICT);
    assert_eq!(input(&conflict.html, "expected_revision"), old_revision);
    assert!(conflict.html.contains("changed to revision 2"));
    assert_eq!(
        client
            .post_to("/orders/13/delete", &stale_delete, ORIGIN)
            .await
            .status,
        StatusCode::CONFLICT
    );
    assert_eq!(client.get("/orders/13").await.status, StatusCode::OK);

    let confirmation = client.get("/orders/13/delete").await;
    let revision = input(&confirmation.html, "expected_revision");
    assert_eq!(revision, "2");
    let deletion = [
        ("_csrf", csrf.as_str()),
        ("expected_revision", revision.as_str()),
    ];
    let deleted = client.post_to("/orders/13/delete", &deletion, ORIGIN).await;
    assert_eq!(deleted.status, StatusCode::SEE_OTHER);
    assert_eq!(deleted.headers[header::LOCATION], "/orders");
    for path in ["/orders/13", "/orders/13/edit", "/orders/13/delete"] {
        assert_eq!(
            client.get(path).await.status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    let missing_delete = client.post_to("/orders/13/delete", &deletion, ORIGIN).await;
    assert_missing_order(&missing_delete);
    let missing_edit = client
        .post_to(
            "/orders/13/edit",
            &fields(&csrf, "Deleted order edit", "1", "1", "2"),
            ORIGIN,
        )
        .await;
    assert_missing_order(&missing_edit);

    let next = client
        .post_to(
            "/orders/new",
            &fields(&csrf, "Next order", "1", "1", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(next.status, StatusCode::SEE_OTHER);
    assert_eq!(next.headers[header::LOCATION], "/orders/14");
}

#[tokio::test]
async fn missing_records_return_404_for_detail_edit_and_delete_pages() {
    let mut client = Client::new().await;
    for path in ["/orders/999", "/orders/999/edit", "/orders/999/delete"] {
        assert_eq!(
            client.get(path).await.status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn missing_record_posts_render_no_order_snapshot_or_mutation_form() {
    let mut client = Client::new().await;
    let editor = client.get("/orders/1/edit").await;
    let csrf = input(&editor.html, "_csrf");
    for quantity in ["1", "invalid"] {
        let reply = client
            .post_to(
                "/orders/999/edit",
                &fields(&csrf, "Missing order", quantity, "1", "1"),
                ORIGIN,
            )
            .await;
        assert_missing_order(&reply);
    }
    for revision in ["1", "invalid"] {
        let reply = client
            .post_to(
                "/orders/999/delete",
                &[("_csrf", csrf.as_str()), ("expected_revision", revision)],
                ORIGIN,
            )
            .await;
        assert_missing_order(&reply);
    }
}

#[tokio::test]
async fn editing_order_zero_cannot_create_an_order() {
    let mut client = Client::new().await;
    let editor = client.get("/orders/new").await;
    let csrf = input(&editor.html, "_csrf");
    assert_eq!(
        client.get("/orders/0/edit").await.status,
        StatusCode::NOT_FOUND
    );
    let rejected = client
        .post_to(
            "/orders/0/edit",
            &fields(&csrf, "Must not create", "1", "1", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(rejected.status, StatusCode::NOT_FOUND);
    assert!(!rejected.html.contains("<form"));
    assert_eq!(client.get("/orders/0").await.status, StatusCode::NOT_FOUND);
    assert_eq!(client.get("/orders/13").await.status, StatusCode::NOT_FOUND);
    let created = client
        .post_to(
            "/orders/new",
            &fields(&csrf, "Valid creation", "1", "1", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(created.status, StatusCode::SEE_OTHER);
    assert_eq!(created.headers[header::LOCATION], "/orders/13");
}

#[tokio::test]
async fn all_typed_mutation_endpoints_enforce_csrf_before_changing_records() {
    let mut client = Client::new().await;
    let page = client.get("/orders/1/edit").await;
    let csrf = input(&page.html, "_csrf");
    let mut save = fields(&csrf, "API rejected edit", "3", "2", "1");
    save.push(("id", "1"));
    let create = fields(&csrf, "API rejected create", "3", "2", "1");
    let delete = vec![
        ("_csrf", csrf.as_str()),
        ("id", "1"),
        ("expected_revision", "1"),
    ];
    for (path, valid) in [
        (SaveOrder::PATH, save),
        (CreateOrder::PATH, create),
        (DeleteOrder::PATH, delete),
    ] {
        let missing: Vec<_> = valid
            .iter()
            .copied()
            .filter(|(name, _)| *name != "_csrf")
            .collect();
        assert_eq!(
            client.post_api(path, &missing, ORIGIN).await.status,
            StatusCode::FORBIDDEN,
            "{path}"
        );
        let wrong: Vec<_> = valid
            .iter()
            .map(|&(name, value)| (name, if name == "_csrf" { "wrong" } else { value }))
            .collect();
        assert_eq!(
            client.post_api(path, &wrong, ORIGIN).await.status,
            StatusCode::FORBIDDEN,
            "{path}"
        );
        assert_eq!(
            client
                .post_api(path, &valid, "https://elsewhere.example.com")
                .await
                .status,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
    let unchanged = client.get("/orders/1/edit").await;
    assert_eq!(input(&unchanged.html, "customer"), "Ada Lovelace");
    assert_eq!(input(&unchanged.html, "expected_revision"), "1");
    assert_eq!(client.get("/orders/13").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn valid_typed_mutations_change_the_same_orders_as_native_forms() {
    let mut client = Client::new().await;
    let page = client.get("/orders/1/edit").await;
    let csrf = input(&page.html, "_csrf");
    let mut save = fields(&csrf, "Typed edit", "3", "2", "1");
    save.push(("id", "1"));
    let saved = client.post_api(SaveOrder::PATH, &save, ORIGIN).await;
    assert_eq!(saved.status, StatusCode::OK);
    assert!(
        saved.headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    let editor = client.get("/orders/1/edit").await;
    assert_eq!(input(&editor.html, "customer"), "Typed edit");
    assert_eq!(input(&editor.html, "expected_revision"), "2");

    let created = client
        .post_api(
            CreateOrder::PATH,
            &fields(&csrf, "Typed new order", "1", "2", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(created.status, StatusCode::OK);
    let editor = client.get("/orders/13/edit").await;
    assert_eq!(editor.status, StatusCode::OK);
    assert_eq!(input(&editor.html, "customer"), "Typed new order");
    assert_eq!(input(&editor.html, "expected_revision"), "1");

    let deleted = client
        .post_api(
            DeleteOrder::PATH,
            &[
                ("_csrf", csrf.as_str()),
                ("id", "1"),
                ("expected_revision", "2"),
            ],
            ORIGIN,
        )
        .await;
    assert_eq!(deleted.status, StatusCode::OK);
    assert_eq!(client.get("/orders/1").await.status, StatusCode::NOT_FOUND);
    assert_eq!(client.get("/orders/13").await.status, StatusCode::OK);
}

#[tokio::test]
async fn anonymous_reads_require_login_and_failed_login_retains_username_and_destination() {
    let mut client = Client::anonymous().await;
    let protected = client.get("/orders?q=Grace&page=2").await;
    assert_eq!(protected.status, StatusCode::SEE_OTHER);
    assert_eq!(
        protected.headers[header::LOCATION],
        "/login?return_to=%2Forders%3Fq%3DGrace%26page%3D2"
    );
    for (path, parameters) in [
        (LoadOrder::PATH, vec![("id", "1")]),
        (LoadDelete::PATH, vec![("id", "1")]),
        (ListOrders::PATH, vec![("query", ""), ("page", "1")]),
    ] {
        let reply = client.get_api(path, &parameters).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.html.contains("Unauthenticated"));
        assert!(!reply.html.contains("Ada Lovelace"));
    }
    let page = client.get("/login?return_to=%2Forders%2F1%2Fedit").await;
    let csrf = input(&page.html, "_csrf");
    let rejected = client
        .post_to(
            "/login",
            &[
                ("username", "missing <user>"),
                ("password", "never-echo-this-password"),
                ("_csrf", &csrf),
                ("return_to", "/orders/1/edit"),
            ],
            ORIGIN,
        )
        .await;
    assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    assert_eq!(input(&rejected.html, "username"), "missing <user>");
    assert_eq!(input(&rejected.html, "return_to"), "/orders/1/edit");
    assert!(!rejected.html.contains("never-echo-this-password"));
    let signed_in = client
        .login("alice", "test-password-123", "https://elsewhere.example/")
        .await;
    assert_eq!(signed_in.status, StatusCode::SEE_OTHER);
    assert_eq!(signed_in.headers[header::LOCATION], "/orders");
    assert_eq!(client.get("/ready").await.status, StatusCode::OK);
}

#[tokio::test]
async fn login_logout_rotate_session_and_csrf_and_invalidate_old_session_ids() {
    let mut client = Client::anonymous().await;
    let login_page = client.get("/login").await;
    let before_login = input(&login_page.html, "_csrf");
    let login = client
        .login("alice", "test-password-123", "/orders/1/edit")
        .await;
    assert_eq!(login.status, StatusCode::SEE_OTHER);
    let set_cookies: Vec<_> = login
        .headers
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|header| header.to_str().unwrap())
        .collect();
    for name in ["__Host-orders-session=", "__Host-stucco-csrf="] {
        let cookie = set_cookies
            .iter()
            .find(|cookie| cookie.starts_with(name))
            .unwrap();
        for attribute in ["Path=/", "HttpOnly", "SameSite=Lax", "Secure"] {
            assert!(cookie.contains(attribute), "missing {attribute} in {name}");
        }
    }
    let editor = client.get("/orders/1/edit").await;
    let after_login = input(&editor.html, "_csrf");
    assert_ne!(after_login, before_login);
    assert_eq!(
        client
            .post(&fields(&before_login, "Old CSRF", "1", "1", "1"), ORIGIN)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .post(&fields(&after_login, "Valid CSRF", "1", "1", "1"), ORIGIN)
            .await
            .status,
        StatusCode::SEE_OTHER
    );

    let mut replay = client.peer();
    replay.cookies = client.cookies.clone();
    let first_session = client.cookies["__Host-orders-session"].clone();
    client.login("alice", "test-password-123", "/orders").await;
    assert_ne!(client.cookies["__Host-orders-session"], first_session);
    assert_eq!(replay.get("/orders").await.status, StatusCode::SEE_OTHER);
    let editor = client.get("/orders/1/edit").await;
    let before_logout = input(&editor.html, "_csrf");
    let mut replay = client.peer();
    replay.cookies = client.cookies.clone();
    let logout = client
        .post_to("/logout", &[("_csrf", &before_logout)], ORIGIN)
        .await;
    assert_eq!(logout.status, StatusCode::SEE_OTHER);
    assert_eq!(logout.headers[header::LOCATION], "/login");
    assert!(!client.cookies.contains_key("__Host-orders-session"));
    let login = client.get("/login").await;
    assert_ne!(input(&login.html, "_csrf"), before_logout);
    assert_eq!(replay.get("/orders").await.status, StatusCode::SEE_OTHER);
    assert_eq!(
        client
            .post_to(
                "/login",
                &[
                    ("username", "alice"),
                    ("password", "test-password-123"),
                    ("_csrf", &before_logout)
                ],
                ORIGIN
            )
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn persisted_sessions_and_order_data_survive_a_router_and_pool_restart() {
    let mut client = Client::new().await;
    let editor = client.get("/orders/1/edit").await;
    let csrf = input(&editor.html, "_csrf");
    assert_eq!(
        client
            .post(
                &fields(&csrf, "Saved before restart", "4", "2", "1"),
                ORIGIN
            )
            .await
            .status,
        StatusCode::SEE_OTHER
    );
    let session = client.cookies["__Host-orders-session"].clone();
    client.app = Router::new();
    client.store.close().await;
    let store = Client::open(&client.directory.path().join("orders.testdb")).await;
    client.app = Client::router(store.clone()).await;
    client.store = store;
    let reloaded = client.get("/orders/1/edit").await;
    assert_eq!(reloaded.status, StatusCode::OK);
    assert_eq!(input(&reloaded.html, "customer"), "Saved before restart");
    assert_eq!(input(&reloaded.html, "expected_revision"), "2");
    assert_eq!(client.cookies["__Host-orders-session"], session);
}

#[tokio::test]
async fn tenant_boundaries_apply_to_native_and_typed_reads_and_all_mutations() {
    let mut alice = Client::new().await;
    let bob_principal = alice.add_bob().await;
    let mut bob = alice.peer();
    assert_eq!(
        bob.login("bob", "other-password-456", "/orders")
            .await
            .status,
        StatusCode::SEE_OTHER
    );
    let new = bob.get("/orders/new").await;
    let csrf = input(&new.html, "_csrf");
    let created = bob
        .post_api(
            CreateOrder::PATH,
            &fields(&csrf, "Bob private customer", "2", "3", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(created.status, StatusCode::OK);
    assert!(created.html.contains("Bob private customer"));
    let bob_order = alice
        .store
        .list(&bob_principal, String::new(), 1)
        .await
        .unwrap()
        .orders
        .remove(0);
    assert_eq!(bob_order.id, 13);
    for path in ["/orders/13", "/orders/13/edit", "/orders/13/delete"] {
        assert_eq!(alice.get(path).await.status, StatusCode::NOT_FOUND);
    }
    for path in [LoadOrder::PATH, LoadDelete::PATH] {
        let hidden = alice.get_api(path, &[("id", "13")]).await;
        assert_eq!(hidden.status, StatusCode::OK);
        assert_eq!(hidden.html, "{\"Ok\":null}");
    }
    let hidden_list = alice
        .get_api(ListOrders::PATH, &[("query", "Bob private"), ("page", "1")])
        .await;
    assert!(hidden_list.html.contains("\"total_count\":0"));
    assert!(!hidden_list.html.contains("Bob private customer"));
    let editor = alice.get("/orders/1/edit").await;
    let csrf = input(&editor.html, "_csrf");
    let mut forged = fields(&csrf, "Cross-tenant overwrite", "1", "1", "1");
    forged.push(("id", "13"));
    let rejected = alice.post_api(SaveOrder::PATH, &forged, ORIGIN).await;
    assert!(rejected.html.contains("NotFound"));
    let rejected = alice
        .post_api(
            DeleteOrder::PATH,
            &[("id", "13"), ("expected_revision", "1"), ("_csrf", &csrf)],
            ORIGIN,
        )
        .await;
    assert!(rejected.html.contains("NotFound"));
    assert_missing_order(&alice.post_to("/orders/13/edit", &forged, ORIGIN).await);
    assert_missing_order(
        &alice
            .post_to(
                "/orders/13/delete",
                &[("expected_revision", "1"), ("_csrf", &csrf)],
                ORIGIN,
            )
            .await,
    );
    assert_eq!(
        alice.store.load_by_id(&bob_principal, 13).await.unwrap(),
        Some(bob_order)
    );
    let alice_principal = orders_fullstack::auth::principal_for_username(&alice.store, "alice")
        .await
        .unwrap();
    assert_eq!(
        alice
            .store
            .list(&alice_principal, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        12
    );
}

#[tokio::test]
async fn a_stale_asserted_identity_cannot_read_or_mutate_under_the_new_account() {
    let mut client = Client::new().await;
    let alice_key = client.expected;
    let bob = client.add_bob().await;
    client.login("bob", "other-password-456", "/orders").await;
    let page = client.get("/orders/new").await;
    let csrf = input(&page.html, "_csrf");
    let user = alice_key.user_id.to_string();
    let tenant = alice_key.tenant_id.to_string();
    let assertions = [
        ("expected_user_id", user.as_str()),
        ("expected_tenant_id", tenant.as_str()),
    ];
    for (path, mut parameters) in [
        (LoadOrder::PATH, vec![("id", "1")]),
        (LoadDelete::PATH, vec![("id", "1")]),
        (ListOrders::PATH, vec![("query", ""), ("page", "1")]),
    ] {
        parameters.extend(assertions);
        let rejected = client.get_api(path, &parameters).await;
        assert!(rejected.html.contains("IdentityChanged"));
        assert!(!rejected.html.contains("Ada Lovelace"));
    }
    let mut stale = fields(&csrf, "Old account draft", "7x", "2", "1");
    stale.extend(assertions);
    for path in [CreateOrder::PATH, SaveOrder::PATH] {
        let mut input = stale.clone();
        input.push(("id", "1"));
        let rejected = client.post_api(path, &input, ORIGIN).await;
        assert!(rejected.html.contains("IdentityChanged"));
        assert!(rejected.html.contains("Old account draft"));
    }
    let mut deletion = vec![
        ("_csrf", csrf.as_str()),
        ("id", "1"),
        ("expected_revision", "1"),
    ];
    deletion.extend(assertions);
    assert!(
        client
            .post_api(DeleteOrder::PATH, &deletion, ORIGIN)
            .await
            .html
            .contains("IdentityChanged")
    );
    for path in ["/orders/new", "/orders/1/edit"] {
        let rejected = client.post_to(path, &stale, ORIGIN).await;
        assert_eq!(rejected.status, StatusCode::FORBIDDEN);
        assert!(rejected.html.contains("Sign in to recover your draft"));
        assert!(!rejected.html.contains("Ada Lovelace"));
        assert!(!rejected.html.contains("$59.00"));
    }
    assert_eq!(
        client
            .post_to("/orders/1/delete", &deletion, ORIGIN)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .store
            .list(&bob, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
}

#[tokio::test]
async fn expired_native_save_recovers_exact_values_and_revision_only_after_matching_login() {
    let mut client = Client::new().await;
    let editor = client.get("/orders/1/edit").await;
    let csrf = input(&editor.html, "_csrf");
    client.expire_sessions().await;
    let raw = fields(&csrf, "  Original <draft>  ", "7x", " 03 ", " 01 ");
    let expired = client.post(&raw, ORIGIN).await;
    assert_eq!(expired.status, StatusCode::UNAUTHORIZED);
    assert!(expired.html.contains("Sign in to recover your draft"));
    assert!(!expired.html.contains("Ada Lovelace"));
    let principal = orders_fullstack::auth::principal_for_username(&client.store, "alice")
        .await
        .unwrap();
    let mut changed = client
        .store
        .load_by_id(&principal, 1)
        .await
        .unwrap()
        .unwrap()
        .draft();
    changed.customer = "Someone saved meanwhile".into();
    client
        .store
        .save_by_id(&principal, 1, changed)
        .await
        .unwrap();
    let login = client
        .login("alice", "test-password-123", "/orders/1/edit")
        .await;
    assert_eq!(login.headers[header::LOCATION], "/orders/1/edit");
    let recovered = client.get("/orders/1/edit").await;
    assert_eq!(input(&recovered.html, "customer"), "  Original <draft>  ");
    assert_eq!(input(&recovered.html, "quantity_one"), "7x");
    assert_eq!(input(&recovered.html, "quantity_two"), " 03 ");
    assert_eq!(input(&recovered.html, "expected_revision"), " 01 ");
    assert_eq!(
        input(&recovered.html, "expected_user_id"),
        principal.key.user_id.to_string()
    );
    let csrf = input(&recovered.html, "_csrf");
    let conflict = client
        .post(&fields(&csrf, "Original draft", "7", "3", " 01 "), ORIGIN)
        .await;
    assert_eq!(conflict.status, StatusCode::CONFLICT);
    assert_eq!(input(&conflict.html, "expected_revision"), " 01 ");
}

#[tokio::test]
async fn signing_into_another_account_discards_the_native_recovery_without_creating_an_order() {
    let mut client = Client::new().await;
    let bob = client.add_bob().await;
    let page = client.get("/orders/new").await;
    let csrf = input(&page.html, "_csrf");
    client.expire_sessions().await;
    let expired = client
        .post_to(
            "/orders/new",
            &fields(&csrf, "Alice private unsent", "9x", "1", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(expired.status, StatusCode::UNAUTHORIZED);
    client
        .login("bob", "other-password-456", "/orders/new")
        .await;
    let new = client.get("/orders/new").await;
    assert_eq!(input(&new.html, "customer"), "");
    assert_eq!(input(&new.html, "quantity_one"), "1");
    assert!(!new.html.contains("Alice private unsent"));
    assert_eq!(
        client
            .store
            .list(&bob, String::new(), 1)
            .await
            .unwrap()
            .total_count,
        0
    );
    client
        .login("alice", "test-password-123", "/orders/new")
        .await;
    assert_eq!(input(&client.get("/orders/new").await.html, "customer"), "");
}

#[tokio::test]
async fn expired_delete_and_typed_mutations_retain_inputs_without_bypassing_authorization() {
    let mut client = Client::new().await;
    let editor = client.get("/orders/1/edit").await;
    let csrf = input(&editor.html, "_csrf");
    client.expire_sessions().await;
    let mut save = fields(&csrf, "Expired typed draft", "3", "2", "1");
    save.push(("id", "1"));
    for path in [SaveOrder::PATH, CreateOrder::PATH] {
        let rejected = client.post_api(path, &save, ORIGIN).await;
        assert_eq!(rejected.status, StatusCode::OK);
        assert!(rejected.html.contains("Unauthenticated"));
        assert!(rejected.html.contains("Expired typed draft"));
    }
    let deletion = [
        ("_csrf", csrf.as_str()),
        ("id", "1"),
        ("expected_revision", " 01 "),
    ];
    let rejected = client.post_api(DeleteOrder::PATH, &deletion, ORIGIN).await;
    assert!(rejected.html.contains("Unauthenticated"));
    assert!(rejected.html.contains(" 01 "));
    let recovery = client.post_to("/orders/1/delete", &deletion, ORIGIN).await;
    assert_eq!(recovery.status, StatusCode::UNAUTHORIZED);
    client
        .login("alice", "test-password-123", "/orders/1/delete")
        .await;
    let recovered = client.get("/orders/1/delete").await;
    assert_eq!(recovered.status, StatusCode::OK);
    assert_eq!(input(&recovered.html, "expected_revision"), " 01 ");
    assert!(client.get("/orders/1").await.html.contains("Ada Lovelace"));
    assert_eq!(client.get("/orders/13").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn explicit_session_revocation_and_disabled_users_cannot_reuse_authenticated_cookies() {
    let mut client = Client::new().await;
    assert!(client.store.revoke_sessions("alice").await.unwrap());
    assert_eq!(client.get("/orders").await.status, StatusCode::SEE_OTHER);
    client.login("alice", "test-password-123", "/orders").await;
    assert_eq!(client.get("/orders").await.status, StatusCode::OK);
    assert!(client.store.set_user_enabled("alice", false).await.unwrap());
    assert_eq!(client.get("/orders").await.status, StatusCode::SEE_OTHER);
    assert_eq!(
        client
            .login("alice", "test-password-123", "/orders")
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn absolute_expiry_survives_session_updates_and_wrong_account_cannot_consume_recovery() {
    let mut alice = Client::new().await;
    alice.add_bob().await;
    let original = alice.expected;
    alice.login("bob", "other-password-456", "/orders").await;
    let sessions = Sessions::new(&alice.store).await.unwrap();
    let id = alice.cookies["__Host-orders-session"].parse().unwrap();
    let before = sessions.load(&id).await.unwrap().unwrap();
    let editor = alice.get("/login").await;
    let csrf = input(&editor.html, "_csrf");
    let mut submission = fields(&csrf, "Retained <raw> draft", "2x", "003", " 01 ");
    let uid = original.user_id.to_string();
    let tid = original.tenant_id.to_string();
    submission.extend([
        ("expected_user_id", uid.as_str()),
        ("expected_tenant_id", tid.as_str()),
    ]);
    let rejected = alice.post(&submission, ORIGIN).await;
    assert_eq!(rejected.status, StatusCode::FORBIDDEN);
    let after = sessions.load(&id).await.unwrap().unwrap();
    assert_eq!(
        before.expiry_date, after.expiry_date,
        "Recovery writes cannot extend login lifetime"
    );
    assert!(after.data.contains_key("native_recovery"));
    assert_eq!(
        alice.get("/orders/1/edit").await.status,
        StatusCode::NOT_FOUND
    );
    assert!(
        sessions
            .load(&id)
            .await
            .unwrap()
            .unwrap()
            .data
            .contains_key("native_recovery")
    );
    alice
        .login("alice", "test-password-123", "/orders/1/edit")
        .await;
    let id = alice.cookies["__Host-orders-session"].parse().unwrap();
    let deadline = sessions.load(&id).await.unwrap().unwrap().expiry_date;
    let recovered = alice.get("/orders/1/edit").await;
    assert_eq!(input(&recovered.html, "customer"), "Retained <raw> draft");
    assert_eq!(input(&recovered.html, "expected_revision"), " 01 ");
    let after = sessions.load(&id).await.unwrap().unwrap();
    assert_eq!(
        after.expiry_date, deadline,
        "Consuming recovery cannot reset absolute expiry"
    );
    assert!(!after.data.contains_key("native_recovery"));
    // Even an incorrectly extended store/cookie expiry cannot bypass auth_deadline.
    let mut record = after;
    record
        .data
        .insert("auth_deadline".into(), serde_json::json!(0));
    sessions.save(&record).await.unwrap();
    assert_eq!(alice.get("/orders").await.status, StatusCode::SEE_OTHER);
    assert!(sessions.load(&id).await.unwrap().is_none());
}

#[tokio::test]
async fn deleted_order_after_native_expiry_keeps_submitted_text_without_a_mutation_form() {
    let mut client = Client::new().await;
    let editor = client.get("/orders/1/edit").await;
    let csrf = input(&editor.html, "_csrf");
    client.expire_sessions().await;
    let rejected = client
        .post(
            &fields(&csrf, "Copy <this> draft", "2x", "003", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(rejected.status, StatusCode::UNAUTHORIZED);
    let principal = orders_fullstack::auth::principal_for_username(&client.store, "alice")
        .await
        .unwrap();
    client
        .store
        .delete(&principal, 1, "1".into())
        .await
        .unwrap();
    client
        .login("alice", "test-password-123", "/orders/1/edit")
        .await;
    let missing = client.get("/orders/1/edit").await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(missing.html.contains("Copy &lt;this&gt; draft"));
    assert!(missing.html.contains("2x"));
    assert!(!missing.html.contains("<form"));
}

#[tokio::test]
async fn oversized_anonymous_recovery_creates_no_session() {
    let mut client = Client::anonymous().await;
    let page = client.get("/login").await;
    let csrf = input(&page.html, "_csrf");
    let reply = client
        .post_to(
            "/orders/new",
            &fields(&csrf, &"x".repeat(1025), "1", "1", "0"),
            ORIGIN,
        )
        .await;
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(!client.cookies.contains_key("__Host-orders-session"));
}

#[tokio::test]
async fn restored_generation_rejects_old_identity_even_when_ids_and_revisions_match() {
    let mut client = Client::new().await;
    let old = client.expected;
    let editor = client.get("/orders/1/edit").await;
    let old_csrf = input(&editor.html, "_csrf");
    // Restore rotates generation offline. Restarting the router picks it up and
    // invalidates the old auth hash before any protected handler executes.
    client.store.rotate_generation().await.unwrap();
    client.app = Client::router(client.store.clone()).await;
    assert_eq!(client.get("/orders").await.status, StatusCode::SEE_OTHER);
    let recovered = client
        .post(
            &fields(&old_csrf, "Stale backup draft", "2", "1", "1"),
            ORIGIN,
        )
        .await;
    assert_eq!(recovered.status, StatusCode::UNAUTHORIZED);
    client
        .login("alice", "test-password-123", "/orders/1/edit")
        .await;
    assert_eq!(client.expected.user_id, old.user_id);
    assert_eq!(client.expected.tenant_id, old.tenant_id);
    assert_ne!(client.expected.generation, old.generation);
    let editor = client.get("/orders/1/edit").await;
    assert_ne!(input(&editor.html, "customer"), "Stale backup draft");
    let csrf = input(&editor.html, "_csrf");
    let old_generation = old.generation.to_string();
    for (path, values) in [
        (ListOrders::PATH, vec![("query", ""), ("page", "1")]),
        (LoadOrder::PATH, vec![("id", "1")]),
        (LoadDelete::PATH, vec![("id", "1")]),
    ] {
        let mut values = values;
        values.push(("expected_generation", &old_generation));
        let reply = client.get_api(path, &values).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.html.contains("IdentityChanged"), "{}", reply.html);
    }
    let mut draft = fields(&csrf, "Wrong generation", "1", "1", "1");
    draft.push(("expected_generation", &old_generation));
    assert_eq!(
        client.post(&draft, ORIGIN).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client.post_to("/orders/new", &draft, ORIGIN).await.status,
        StatusCode::FORBIDDEN
    );
    for path in [SaveOrder::PATH, CreateOrder::PATH, DeleteOrder::PATH] {
        let mut values = draft.clone();
        values.push(("id", "1"));
        let reply = client.post_api(path, &values, ORIGIN).await;
        assert!(reply.html.contains("IdentityChanged"), "{}", reply.html);
    }
    assert_eq!(
        client
            .post_to("/orders/1/delete", &draft, ORIGIN)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let principal = orders_fullstack::auth::principal_for_username(&client.store, "alice")
        .await
        .unwrap();
    let order = client
        .store
        .load_by_id(&principal, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(order.revision, 1);
    assert_ne!(order.customer, "Wrong generation");
}

#[tokio::test]
async fn anonymous_recovery_stores_only_a_bounded_canonical_route() {
    let mut client = Client::anonymous().await;
    let page = client.get("/login").await;
    let csrf = input(&page.html, "_csrf");
    for action in ["edit", "delete"] {
        let path = format!("/orders/{}1/{action}", "0".repeat(8192));
        let reply = client
            .post_to(&path, &fields(&csrf, "Retained", "1", "1", "1"), ORIGIN)
            .await;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
        let sessions = Sessions::new(&client.store).await.unwrap();
        let id = client.cookies["__Host-orders-session"].parse().unwrap();
        let record = sessions.load(&id).await.unwrap().unwrap();
        assert_eq!(
            record.data["native_recovery"]["path"],
            format!("/orders/1/{action}")
        );
        assert!(serde_json::to_vec(&record).unwrap().len() < 1024);
    }
}
