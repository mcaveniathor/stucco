#![cfg(feature = "ssr")]
use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use leptos::config::LeptosOptions;
use orders_fullstack::{
    auth::{Principal, PrincipalKey},
    exports::new_token,
    store::OrderStore,
};
use std::{collections::BTreeMap, time::Duration};
use stucco_tower::SecretKey;
use tower::ServiceExt;

const ORIGIN: &str = "https://orders.example.com";
struct Client {
    app: Router,
    cookies: BTreeMap<String, String>,
    expected: PrincipalKey,
    csrf: String,
}
struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}
struct Fixture {
    _directory: tempfile::TempDir,
    store: OrderStore,
    alice: Principal,
    client: Client,
}
impl Fixture {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("orders.db");
        #[cfg(feature = "sqlite")]
        let store = OrderStore::open_sqlite(&path).await.unwrap();
        #[cfg(not(feature = "sqlite"))]
        let store = OrderStore::open(&path).await.unwrap();
        let alice = orders_fullstack::auth::add_user(
            &store,
            "tenant-a",
            "alice",
            "Alice",
            "password-for-alice".into(),
        )
        .await
        .unwrap();
        store.seed_orders(&alice).await.unwrap();
        let options = LeptosOptions::builder()
            .output_name("orders-fullstack")
            .site_root("target/orders-fullstack-site")
            .build();
        let app = orders_fullstack::server::app(
            options,
            SecretKey::new([17; 32]),
            ORIGIN,
            store.clone(),
            3600,
        )
        .await
        .unwrap();
        let client = Client {
            app,
            cookies: BTreeMap::new(),
            expected: alice.key,
            csrf: String::new(),
        };
        Self {
            _directory: directory,
            store,
            alice,
            client,
        }
    }
}
impl Client {
    async fn send(&mut self, method: &str, path: &str, fields: &[(&str, String)]) -> Reply {
        let cookie = self
            .cookies
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("; ");
        let body = form_urlencoded::Serializer::new(String::new())
            .extend_pairs(fields.iter().map(|(key, value)| (*key, value.as_str())))
            .finish();
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(header::COOKIE, cookie)
            .header(header::ORIGIN, ORIGIN)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        for cookie in response.headers().get_all(header::SET_COOKIE) {
            let text = cookie.to_str().unwrap();
            let (key, value) = text.split(';').next().unwrap().split_once('=').unwrap();
            if value.is_empty() || text.contains("Max-Age=0") {
                self.cookies.remove(key);
            } else {
                self.cookies.insert(key.to_owned(), value.to_owned());
            }
        }
        let status = response.status();
        let headers = response.headers().clone();
        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        Reply {
            status,
            headers,
            body,
        }
    }
    async fn login(&mut self, username: &str, password: &str) {
        let page = self.send("GET", "/login", &[]).await;
        let csrf = input(&page.body, "_csrf");
        let response = self
            .send(
                "POST",
                "/login",
                &[
                    ("username", username.into()),
                    ("password", password.into()),
                    ("_csrf", csrf),
                    ("return_to", "/orders".into()),
                ],
            )
            .await;
        assert_eq!(response.status, StatusCode::SEE_OTHER);
        let page = self.send("GET", "/orders", &[]).await;
        self.csrf = input(&page.body, "_csrf");
    }
    async fn post(&mut self, path: &str, mut fields: Vec<(&str, String)>) -> Reply {
        fields.extend([
            ("expected_user_id", self.expected.user_id.to_string()),
            ("expected_tenant_id", self.expected.tenant_id.to_string()),
            ("expected_generation", self.expected.generation.to_string()),
            ("_csrf", self.csrf.clone()),
        ]);
        self.send("POST", path, &fields).await
    }
    async fn download(&mut self, token: &str) -> Reply {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let reply = self
                    .send("GET", &format!("/exports/{token}/download"), &[])
                    .await;
                if reply.status != StatusCode::NOT_FOUND {
                    break reply;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("worker output available")
    }
}
fn input(html: &str, name: &str) -> String {
    let marker = format!("name=\"{name}\"");
    let element = html
        .split(&marker)
        .nth(1)
        .unwrap_or_else(|| panic!("missing {name}: {html}"));
    element
        .split("value=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .into()
}

#[tokio::test]
async fn native_export_uses_real_csrf_auth_redirect_and_private_csv_download() {
    let mut f = Fixture::new().await;
    f.client.login("alice", "password-for-alice").await;
    let token = new_token().unwrap();
    let first = f
        .client
        .post("/exports", vec![("token", token.clone())])
        .await;
    assert_eq!(first.status, StatusCode::SEE_OTHER, "{}", first.body);
    assert_eq!(first.headers[header::LOCATION], format!("/exports/{token}"));
    let repeated = f
        .client
        .post("/exports", vec![("token", token.clone())])
        .await;
    assert_eq!(repeated.status, StatusCode::SEE_OTHER);
    assert_eq!(
        repeated.headers[header::LOCATION],
        first.headers[header::LOCATION]
    );
    let csv = f.client.download(&token).await;
    assert_eq!(csv.status, StatusCode::OK);
    assert_eq!(csv.headers[header::CONTENT_TYPE], "text/csv; charset=utf-8");
    assert_eq!(csv.headers[header::CACHE_CONTROL], "private, no-store");
    assert_eq!(csv.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert!(
        csv.headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment;")
    );
    assert_eq!(csv.body.lines().count(), 13);
    let cancel = f
        .client
        .post(&format!("/exports/{token}/cancel"), vec![])
        .await;
    assert_eq!(cancel.status, StatusCode::SEE_OTHER);
    assert_eq!(
        f.client
            .send("GET", &format!("/exports/{token}/download"), &[])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let retry = f
        .client
        .post(
            &format!("/exports/{token}/retry"),
            vec![("expected_attempt", "1".into())],
        )
        .await;
    assert_eq!(retry.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn native_export_rejects_missing_csrf_stale_identity_and_expired_session() {
    let mut f = Fixture::new().await;
    f.client.login("alice", "password-for-alice").await;
    let token = new_token().unwrap();
    let missing = f
        .client
        .send("POST", "/exports", &[("token", token.clone())])
        .await;
    assert_eq!(missing.status, StatusCode::FORBIDDEN);
    f.client.expected.generation = f.client.expected.generation.wrapping_add(1);
    let stale = f
        .client
        .post("/exports", vec![("token", token.clone())])
        .await;
    assert_eq!(stale.status, StatusCode::FORBIDDEN);
    assert!(stale.body.contains("Sign in to continue"));
    f.client.expected = f.alice.key;
    f.client.cookies.remove("__Host-orders-session");
    let expired = f
        .client
        .post("/exports", vec![("token", token.clone())])
        .await;
    assert_eq!(expired.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        f.client
            .send("GET", &format!("/exports/{token}/download"), &[])
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn another_account_cannot_download_or_cancel_an_export_in_the_same_tenant() {
    let mut f = Fixture::new().await;
    let teammate = orders_fullstack::auth::add_user(
        &f.store,
        "tenant-a",
        "other",
        "Other",
        "password-for-other".into(),
    )
    .await
    .unwrap();
    f.client.login("alice", "password-for-alice").await;
    let token = new_token().unwrap();
    f.client
        .post("/exports", vec![("token", token.clone())])
        .await;
    f.client.download(&token).await;
    let mut peer = Client {
        app: f.client.app.clone(),
        cookies: BTreeMap::new(),
        expected: teammate.key,
        csrf: String::new(),
    };
    peer.login("other", "password-for-other").await;
    assert_eq!(
        peer.send("GET", &format!("/exports/{token}/download"), &[])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        peer.post(&format!("/exports/{token}/cancel"), vec![])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(f.client.download(&token).await.status, StatusCode::OK);
}
