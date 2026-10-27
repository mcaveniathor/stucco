#![cfg(feature = "ssr")]

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
};
use http_body_util::BodyExt;
use leptos::config::LeptosOptions;
use orders_fullstack::{exports::Exports, server, store::OrderStore};
use stucco_tower::SecretKey;
use tower::ServiceExt;

struct Application {
    router: Router,
    store: OrderStore,
    exports: Exports,
    _directory: tempfile::TempDir,
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Application {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let site = directory.path().join("site");
        let pkg = site.join("pkg");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("orders-fullstack.js"),
            "export const fixture = true;",
        )
        .unwrap();
        std::fs::write(pkg.join("orders-fullstack.wasm"), b"\0asm\x01\0\0\0").unwrap();
        #[cfg(feature = "sqlite")]
        let store = OrderStore::open_sqlite(&directory.path().join("orders.db"))
            .await
            .unwrap();
        #[cfg(not(feature = "sqlite"))]
        let store = OrderStore::open(&directory.path().join("orders.db"))
            .await
            .unwrap();
        let options = LeptosOptions::builder()
            .output_name("orders-fullstack")
            .site_root(site.to_string_lossy().into_owned())
            .build();
        let (router, exports) = server::app_with_exports(
            options,
            SecretKey::new([23; 32]),
            "https://orders.example.com",
            store.clone(),
            3600,
        )
        .await
        .unwrap();
        Self {
            router,
            store,
            exports,
            _directory: directory,
        }
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.router.clone().oneshot(request).await.unwrap();
        Reply {
            status: response.status(),
            headers: response.headers().clone(),
            body: response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        }
    }

    async fn get(&self, path: &str) -> Reply {
        self.send(Request::get(path).body(Body::empty()).unwrap())
            .await
    }

    async fn shutdown(self) {
        self.exports.shutdown().await;
        drop(self.router);
        self.store.close().await;
    }
}

fn security_headers(reply: &Reply) {
    assert_eq!(reply.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(reply.headers[header::REFERRER_POLICY], "same-origin");
    let csp = reply.headers[header::CONTENT_SECURITY_POLICY]
        .to_str()
        .unwrap();
    for directive in [
        "style-src 'self'",
        "object-src 'none'",
        "base-uri 'none'",
        "frame-src 'none'",
        "frame-ancestors 'none'",
        "form-action 'self'",
    ] {
        assert!(csp.contains(directive), "missing {directive}: {csp}");
    }
    assert!(!csp.contains("'unsafe-inline'"));
    assert!(!csp.contains("'unsafe-eval'"));
}

#[tokio::test]
async fn ssr_nonce_is_fresh_and_matches_every_bootstrap_and_resource_script() {
    let app = Application::new().await;
    let mut previous_nonce = String::new();
    for _ in 0..2 {
        let reply = app.get("/login").await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.headers[header::CACHE_CONTROL], "private, no-store");
        security_headers(&reply);
        let csp = reply.headers[header::CONTENT_SECURITY_POLICY]
            .to_str()
            .unwrap();
        let nonce = csp
            .split_once("'nonce-")
            .unwrap()
            .1
            .split('\'')
            .next()
            .unwrap();
        assert_eq!(nonce.len(), 22);
        assert_ne!(nonce, previous_nonce);
        assert!(csp.contains(&format!(
            "script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'"
        )));
        let body = String::from_utf8(reply.body).unwrap();
        let scripts: Vec<_> = body.split("<script").skip(1).collect();
        assert!(
            scripts.len() >= 2,
            "expected hydration and resource scripts"
        );
        for script in scripts {
            let tag = script.split('>').next().unwrap();
            assert!(
                tag.contains(&format!("nonce=\"{nonce}\"")),
                "script missing nonce: {tag}"
            );
        }
        assert!(!body.contains(" style="));
        assert!(!body.contains("<style"));
        assert!(body.contains("/theme.css"));
        assert!(body.contains("/pkg/orders-fullstack.js"));
        previous_nonce = nonce.to_owned();
    }
    app.shutdown().await;
}

#[tokio::test]
async fn private_policy_covers_readiness_redirects_csrf_and_framework_errors() {
    let app = Application::new().await;
    let requests = [
        (Request::get("/ready"), StatusCode::OK),
        (Request::get("/orders"), StatusCode::SEE_OTHER),
        (Request::post("/login"), StatusCode::FORBIDDEN),
        (Request::put("/theme.css"), StatusCode::METHOD_NOT_ALLOWED),
        (Request::get("/pkg/missing.js"), StatusCode::NOT_FOUND),
    ];
    for (request, expected) in requests {
        let reply = app.send(request.body(Body::empty()).unwrap()).await;
        assert_eq!(reply.status, expected);
        assert_eq!(reply.headers[header::CACHE_CONTROL], "private, no-store");
        security_headers(&reply);
        assert!(
            reply.headers[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .starts_with("default-src 'none'")
        );
    }
    app.store.close().await;
    let failed = app.get("/ready").await;
    assert_eq!(failed.status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(failed.headers[header::CACHE_CONTROL], "private, no-store");
    security_headers(&failed);
    app.shutdown().await;
}

#[tokio::test]
async fn public_assets_revalidate_without_session_cookies_and_keep_their_mime_types() {
    let app = Application::new().await;
    for (path, mime) in [
        ("/theme.css", "text/css; charset=utf-8"),
        ("/pkg/orders-fullstack.js", "text/javascript"),
        ("/pkg/orders-fullstack.wasm", "application/wasm"),
    ] {
        let reply = app.get(path).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.headers[header::CONTENT_TYPE], mime);
        assert_eq!(reply.headers[header::CACHE_CONTROL], "no-cache");
        assert!(!reply.headers.contains_key(header::SET_COOKIE));
        assert!(!reply.body.is_empty());
        security_headers(&reply);
        if path.starts_with("/pkg/") {
            let request = Request::get(path)
                .header(
                    header::IF_MODIFIED_SINCE,
                    reply.headers[header::LAST_MODIFIED].clone(),
                )
                .body(Body::empty())
                .unwrap();
            let unchanged = app.send(request).await;
            assert_eq!(unchanged.status, StatusCode::NOT_MODIFIED);
            assert_eq!(unchanged.headers[header::CACHE_CONTROL], "no-cache");
            assert!(unchanged.body.is_empty());
            security_headers(&unchanged);
        }
    }
    app.shutdown().await;
}

#[test]
fn runtime_overrides_cannot_silently_disconnect_fixed_routes_from_the_client() {
    let options = LeptosOptions::builder()
        .output_name("orders-fullstack")
        .build();
    server::validate_runtime_options(&options).unwrap();
    let mut invalid = options.clone();
    invalid.output_name = "different-build".into();
    assert!(
        server::validate_runtime_options(&invalid)
            .unwrap_err()
            .to_string()
            .contains("LEPTOS_OUTPUT_NAME")
    );
    let mut invalid = options.clone();
    invalid.site_pkg_dir = "prefix/pkg".into();
    assert!(
        server::validate_runtime_options(&invalid)
            .unwrap_err()
            .to_string()
            .contains("LEPTOS_SITE_PKG_DIR")
    );
    let mut invalid = options.clone();
    invalid.hash_files = true;
    assert!(
        server::validate_runtime_options(&invalid)
            .unwrap_err()
            .to_string()
            .contains("LEPTOS_HASH_FILES")
    );
    let mut invalid = options.clone();
    invalid.server_fn_prefix = Some("/prefix/api".into());
    assert!(server::validate_runtime_options(&invalid).is_err());
    let mut invalid = options.clone();
    invalid.disable_server_fn_hash = true;
    assert!(server::validate_runtime_options(&invalid).is_err());
    let mut invalid = options;
    invalid.server_fn_mod_path = true;
    assert!(server::validate_runtime_options(&invalid).is_err());
}
