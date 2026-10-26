#![cfg(feature = "ssr")]

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use leptos::config::LeptosOptions;
use orders_fullstack::{
    attachments::{self, Attachment, AttachmentError, MAX_ATTACHMENT_BYTES},
    auth::{Principal, PrincipalKey},
    store::OrderStore,
};
use std::collections::BTreeMap;
use stucco_tower::SecretKey;
use tower::ServiceExt;

const ORIGIN: &str = "https://orders.example.com";

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
async fn seeded(store: &OrderStore) -> Principal {
    let alice = store
        .insert_user("team-a", "alice", "Alice", "unused-test-hash")
        .await
        .unwrap();
    store.seed_orders(&alice).await.unwrap();
    alice
}

#[tokio::test]
async fn bytes_survive_reopen_and_names_are_safe() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("orders.db");
    let store = open(&path).await;
    let alice = seeded(&store).await;
    let bytes = vec![0, 1, 255, 13, 10, 0];
    let saved = attachments::put(
        &store,
        &alice,
        1,
        "C:\\fakepath\\résumé.txt".into(),
        bytes.clone(),
    )
    .await
    .unwrap();
    assert_eq!(saved.file_name, "résumé.txt");
    assert_eq!(saved.size, 6);
    store.close().await;
    drop(store);
    let store = open(&path).await;
    assert_eq!(
        attachments::get(&store, &alice, 1, saved.id).await.unwrap(),
        (saved.clone(), bytes)
    );
    assert_eq!(
        attachments::list(&store, &alice, 1).await.unwrap(),
        vec![saved.clone()]
    );
    attachments::remove(&store, &alice, 1, saved.id)
        .await
        .unwrap();
    assert!(
        attachments::list(&store, &alice, 1)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        attachments::get(&store, &alice, 1, saved.id)
            .await
            .unwrap_err(),
        AttachmentError::NotFound
    );
}

#[tokio::test]
async fn validation_and_concurrent_limit_leave_no_partial_records() {
    let directory = tempfile::tempdir().unwrap();
    let store = open(&directory.path().join("orders.db")).await;
    let alice = seeded(&store).await;
    for (name, bytes, error) in [
        ("empty.txt", vec![], AttachmentError::Invalid),
        ("bad\rname", vec![1], AttachmentError::Invalid),
        ("..", vec![1], AttachmentError::Invalid),
        (
            "too-big",
            vec![0; MAX_ATTACHMENT_BYTES + 1],
            AttachmentError::TooLarge,
        ),
    ] {
        assert_eq!(
            attachments::put(&store, &alice, 1, name.into(), bytes)
                .await
                .unwrap_err(),
            error
        );
    }
    assert!(
        attachments::list(&store, &alice, 1)
            .await
            .unwrap()
            .is_empty()
    );
    let exact = attachments::put(
        &store,
        &alice,
        1,
        "exact.bin".into(),
        vec![127; MAX_ATTACHMENT_BYTES],
    )
    .await
    .unwrap();
    assert_eq!(exact.size, MAX_ATTACHMENT_BYTES as u64);
    let mut joins = Vec::new();
    for index in 0..8 {
        let (store, alice) = (store.clone(), alice.clone());
        joins.push(tokio::spawn(async move {
            attachments::put(&store, &alice, 1, format!("{index}.txt"), vec![index]).await
        }));
    }
    let mut success = 0;
    for join in joins {
        match join.await.unwrap() {
            Ok(_) => success += 1,
            Err(error) => assert_eq!(error, AttachmentError::TooMany),
        }
    }
    assert_eq!(success, 2);
    assert_eq!(attachments::list(&store, &alice, 1).await.unwrap().len(), 3);
}

#[tokio::test]
async fn tenant_disabled_identity_and_restored_generation_are_checked() {
    let directory = tempfile::tempdir().unwrap();
    let store = open(&directory.path().join("orders.db")).await;
    let alice = seeded(&store).await;
    let bob = store
        .insert_user("team-b", "bob", "Bob", "unused-hash")
        .await
        .unwrap();
    let file = attachments::put(&store, &alice, 1, "private.txt".into(), b"private".to_vec())
        .await
        .unwrap();
    assert_eq!(
        attachments::list(&store, &bob, 1).await.unwrap_err(),
        AttachmentError::NotFound
    );
    assert_eq!(
        attachments::get(&store, &bob, 1, file.id)
            .await
            .unwrap_err(),
        AttachmentError::NotFound
    );
    assert_eq!(
        attachments::put(&store, &bob, 1, "other.txt".into(), vec![1])
            .await
            .unwrap_err(),
        AttachmentError::NotFound
    );
    assert_eq!(
        attachments::remove(&store, &bob, 1, file.id)
            .await
            .unwrap_err(),
        AttachmentError::NotFound
    );
    store.set_user_enabled("alice", false).await.unwrap();
    assert_eq!(
        attachments::get(&store, &alice, 1, file.id)
            .await
            .unwrap_err(),
        AttachmentError::IdentityChanged
    );
    store.set_user_enabled("alice", true).await.unwrap();
    store.rotate_generation().await.unwrap();
    assert_eq!(
        attachments::get(&store, &alice, 1, file.id)
            .await
            .unwrap_err(),
        AttachmentError::IdentityChanged
    );
}

async fn validate_backup(path: &std::path::Path) {
    #[cfg(feature = "sqlite")]
    OrderStore::validate_sqlite_backup(path).await.unwrap();
    #[cfg(not(feature = "sqlite"))]
    OrderStore::validate_backup(path).await.unwrap();
}

#[tokio::test]
async fn snapshot_includes_bytes_and_order_delete_cascades_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let store = open(&directory.path().join("orders.db")).await;
    let alice = seeded(&store).await;
    let file = attachments::put(
        &store,
        &alice,
        1,
        "backup.txt".into(),
        b"snapshot bytes".to_vec(),
    )
    .await
    .unwrap();
    let snapshot = directory.path().join("backup.db");
    store.backup_to(&snapshot).await.unwrap();
    validate_backup(&snapshot).await;
    let copy = open(&snapshot).await;
    assert_eq!(
        attachments::get(&copy, &alice, 1, file.id).await.unwrap().1,
        b"snapshot bytes"
    );
    copy.close().await;
    drop(copy);
    let revision = store
        .load_by_id(&alice, 1)
        .await
        .unwrap()
        .unwrap()
        .revision
        .to_string();
    let (deleted, uploaded) = tokio::join!(
        store.delete(&alice, 1, revision),
        attachments::put(&store, &alice, 1, "racing.txt".into(), vec![1]),
    );
    deleted.unwrap();
    assert!(uploaded.is_ok() || uploaded == Err(AttachmentError::NotFound));
    assert_eq!(
        attachments::get(&store, &alice, 1, file.id)
            .await
            .unwrap_err(),
        AttachmentError::NotFound
    );
    let after = directory.path().join("deleted.db");
    store.backup_to(&after).await.unwrap();
    // Snapshot validation rejects any attachment whose order no longer exists.
    validate_backup(&after).await;
}

struct Client {
    app: Router,
    store: OrderStore,
    principal: Principal,
    cookies: BTreeMap<String, String>,
    _directory: tempfile::TempDir,
}
struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    bytes: Vec<u8>,
}
impl Client {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = open(&directory.path().join("orders.db")).await;
        let principal = orders_fullstack::auth::add_user(
            &store,
            "team-a",
            "alice",
            "Alice",
            "test-password-123".into(),
        )
        .await
        .unwrap();
        store.seed_orders(&principal).await.unwrap();
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
        let mut client = Self {
            app,
            store,
            principal,
            cookies: BTreeMap::new(),
            _directory: directory,
        };
        client.login("alice", "test-password-123").await;
        client
    }
    async fn send(&mut self, mut request: Request<Body>) -> Reply {
        let cookie = self
            .cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("; ");
        request
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().unwrap());
        let response = self.app.clone().oneshot(request).await.unwrap();
        for value in response.headers().get_all(header::SET_COOKIE) {
            let cookie = value.to_str().unwrap();
            let (key, value) = cookie.split(';').next().unwrap().split_once('=').unwrap();
            if value.is_empty() || cookie.contains("Max-Age=0") {
                self.cookies.remove(key);
            } else {
                self.cookies.insert(key.to_owned(), value.to_owned());
            }
        }
        Reply {
            status: response.status(),
            headers: response.headers().clone(),
            bytes: response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        }
    }
    async fn get(&mut self, path: &str) -> Reply {
        self.send(Request::get(path).body(Body::empty()).unwrap())
            .await
    }
    async fn csrf(&mut self) -> String {
        let reply = self.get("/orders/1/edit").await;
        input(&String::from_utf8(reply.bytes).unwrap(), "_csrf")
    }
    async fn login(&mut self, username: &str, password: &str) {
        let reply = self.get("/login").await;
        let csrf = input(&String::from_utf8(reply.bytes).unwrap(), "_csrf");
        let body = form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("_csrf", csrf.as_str()),
                ("username", username),
                ("password", password),
                ("return_to", "/orders"),
            ])
            .finish();
        let reply = self
            .send(
                Request::post("/login")
                    .header(header::ORIGIN, ORIGIN)
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await;
        assert_eq!(reply.status, StatusCode::SEE_OTHER);
    }
    async fn upload(
        &mut self,
        body: Vec<u8>,
        json: bool,
        csrf_header: Option<&str>,
        origin: &str,
    ) -> Reply {
        let mut request = Request::post("/orders/1/attachments")
            .header(header::ORIGIN, origin)
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=stucco-test-boundary",
            )
            .header(
                header::ACCEPT,
                if json {
                    "application/json"
                } else {
                    "text/html"
                },
            );
        if let Some(csrf) = csrf_header {
            request = request.header("Stucco-Csrf", csrf);
        }
        self.send(request.body(Body::from(body)).unwrap()).await
    }
}
fn input(html: &str, name: &str) -> String {
    let name = format!("name=\"{name}\"");
    let tag = html
        .split("<input")
        .skip(1)
        .map(|tail| tail.split('>').next().unwrap())
        .find(|tag| tag.contains(&name))
        .unwrap();
    tag.split_once("value=\"")
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap()
        .replace("&amp;", "&")
}
fn multipart(
    csrf: &str,
    expected: PrincipalKey,
    filename: &str,
    bytes: &[u8],
    extra: Option<(&str, &str)>,
    finish: bool,
) -> Vec<u8> {
    let mut body = Vec::new();
    let mut fields = vec![
        ("_csrf", csrf.to_owned()),
        ("expected_user_id", expected.user_id.to_string()),
        ("expected_tenant_id", expected.tenant_id.to_string()),
        ("expected_generation", expected.generation.to_string()),
    ];
    if let Some((name, value)) = extra {
        fields.push((name, value.to_owned()));
    }
    for (name, value) in fields {
        body.extend_from_slice(format!("--stucco-test-boundary\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
    }
    body.extend_from_slice(format!("--stucco-test-boundary\r\nContent-Disposition: form-data; name=\"attachment\"; filename=\"{filename}\"\r\nContent-Type: text/html\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    if finish {
        body.extend_from_slice(b"\r\n--stucco-test-boundary--\r\n");
    }
    body
}

#[tokio::test]
async fn native_and_enhanced_uploads_use_real_csrf_and_authorized_downloads() {
    let mut client = Client::new().await;
    let csrf = client.csrf().await;
    let bytes = b"<script>untrusted file contents</script>";
    let body = multipart(
        &csrf,
        client.principal.key,
        "report.html",
        bytes,
        None,
        true,
    );
    let native = client.upload(body, false, None, ORIGIN).await;
    assert_eq!(native.status, StatusCode::SEE_OTHER);
    assert_eq!(native.headers[header::LOCATION], "/orders/1");
    let body = multipart(
        &csrf,
        client.principal.key,
        "second.txt",
        b"second",
        None,
        true,
    );
    let enhanced = client.upload(body, true, Some(&csrf), ORIGIN).await;
    assert_eq!(
        enhanced.status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&enhanced.bytes)
    );
    let _: Attachment = serde_json::from_slice(&enhanced.bytes).unwrap();
    let files = attachments::list(&client.store, &client.principal, 1)
        .await
        .unwrap();
    let html = files
        .iter()
        .find(|file| file.file_name == "report.html")
        .unwrap();
    let url = format!("/orders/1/attachments/{}", html.id);
    let download = client.get(&url).await;
    assert_eq!(download.status, StatusCode::OK);
    assert_eq!(download.bytes, bytes);
    assert_eq!(
        download.headers[header::CONTENT_TYPE],
        "application/octet-stream"
    );
    assert_eq!(download.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(download.headers[header::CACHE_CONTROL], "private, no-store");
    assert!(
        download.headers[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("attachment;")
    );
    orders_fullstack::auth::add_user(
        &client.store,
        "team-b",
        "bob",
        "Bob",
        "other-password-456".into(),
    )
    .await
    .unwrap();
    client.login("bob", "other-password-456").await;
    assert_eq!(client.get(&url).await.status, StatusCode::NOT_FOUND);
    client.cookies.clear();
    assert_eq!(client.get(&url).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn csrf_rejections_truncated_bodies_and_invalid_fields_never_commit() {
    let mut client = Client::new().await;
    let csrf = client.csrf().await;
    for (token, extra, finish, origin, expected_status) in [
        ("wrong", None, true, ORIGIN, StatusCode::FORBIDDEN),
        (
            csrf.as_str(),
            None,
            true,
            "https://evil.example",
            StatusCode::FORBIDDEN,
        ),
        (
            csrf.as_str(),
            Some(("unexpected", "field")),
            true,
            ORIGIN,
            StatusCode::BAD_REQUEST,
        ),
        (csrf.as_str(), None, false, ORIGIN, StatusCode::BAD_REQUEST),
    ] {
        let body = multipart(
            token,
            client.principal.key,
            "file.txt",
            b"partial",
            extra,
            finish,
        );
        let reply = client.upload(body, true, None, origin).await;
        assert_eq!(
            reply.status,
            expected_status,
            "{}",
            String::from_utf8_lossy(&reply.bytes)
        );
        assert!(
            attachments::list(&client.store, &client.principal, 1)
                .await
                .unwrap()
                .is_empty()
        );
    }
    // A valid file followed by an invalid field must not have been committed
    // while the multipart parser was still waiting for the closing boundary.
    let mut body = multipart(
        &csrf,
        client.principal.key,
        "first.txt",
        b"complete file",
        None,
        false,
    );
    body.extend_from_slice(b"\r\n--stucco-test-boundary\r\nContent-Disposition: form-data; name=\"unexpected\"\r\n\r\ntrailing field\r\n--stucco-test-boundary--\r\n");
    assert_eq!(
        client.upload(body, true, Some(&csrf), ORIGIN).await.status,
        StatusCode::BAD_REQUEST
    );
    assert!(
        attachments::list(&client.store, &client.principal, 1)
            .await
            .unwrap()
            .is_empty()
    );
    let body = multipart(
        &csrf,
        client.principal.key,
        "huge.bin",
        &vec![0; MAX_ATTACHMENT_BYTES + 1],
        None,
        true,
    );
    assert_eq!(
        client.upload(body, true, Some(&csrf), ORIGIN).await.status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(
        attachments::list(&client.store, &client.principal, 1)
            .await
            .unwrap()
            .is_empty()
    );
    let stale = PrincipalKey {
        generation: client.principal.key.generation ^ 1,
        ..client.principal.key
    };
    let body = multipart(&csrf, stale, "stale.txt", b"old history", None, true);
    assert_eq!(
        client.upload(body, true, Some(&csrf), ORIGIN).await.status,
        StatusCode::FORBIDDEN
    );
    assert!(
        attachments::list(&client.store, &client.principal, 1)
            .await
            .unwrap()
            .is_empty()
    );
    client.cookies.remove("__Host-orders-session");
    let body = multipart(
        &csrf,
        client.principal.key,
        "expired.txt",
        b"must not commit",
        None,
        true,
    );
    assert_eq!(
        client.upload(body, true, Some(&csrf), ORIGIN).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert!(
        attachments::list(&client.store, &client.principal, 1)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn native_delete_requires_csrf() {
    let mut client = Client::new().await;
    let saved = attachments::put(
        &client.store,
        &client.principal,
        1,
        "delete.txt".into(),
        b"remove me".to_vec(),
    )
    .await
    .unwrap();
    let csrf = client.csrf().await;
    let url = format!("/orders/1/attachments/{}/delete", saved.id);
    let body = |csrf: &str, expected: PrincipalKey| {
        form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("_csrf", csrf.to_owned()),
                ("expected_user_id", expected.user_id.to_string()),
                ("expected_tenant_id", expected.tenant_id.to_string()),
                ("expected_generation", expected.generation.to_string()),
            ])
            .finish()
    };
    let reply = client
        .send(
            Request::post(&url)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body("wrong", client.principal.key)))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert!(
        attachments::get(&client.store, &client.principal, 1, saved.id)
            .await
            .is_ok()
    );
    let reply = client
        .send(
            Request::post(&url)
                .header(header::ORIGIN, ORIGIN)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(body(&csrf, client.principal.key)))
                .unwrap(),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert!(
        attachments::list(&client.store, &client.principal, 1)
            .await
            .unwrap()
            .is_empty()
    );
}
