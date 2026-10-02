# stucco-tower Thin Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build roadmap step 3: a `stucco-tower` crate that serves `Bundle` assets and full pages through Tower, negotiates full-page vs fragment responses, provides the standard middleware stack, and ships an axum example app covered by browser tests.

**Architecture:** `stucco-tower` depends on `stucco-core`, `tower`, `http`, `http-body`, `http-body-util` and `bytes`. Its default features `axum` and `tower-http` add axum `IntoResponse`/`FromRequestParts` impls, an assets router and the standard layer stack, so new projects are ready to go; both can be turned off for other hosts. `AssetService` is a plain `tower::Service`. Responses carry `Vary: Stucco-Request` so caches keep pages and fragments apart. An `examples/hello` workspace crate runs under Playwright.

**Tech Stack:** Rust 2024 (MSRV 1.85); tower 0.5.3, http 1.5, http-body 1.1, http-body-util 0.1.5, bytes 1.12, axum 0.8.9, tower-http 0.7.1 (features `trace`, `request-id`, `limit`, `timeout`), tokio 1.53, tracing 0.1 (versions verified 2026-10-03; all MSRVs ≤ 1.80).

**Spec:** `docs/superpowers/specs/2026-10-01-stucco-integration-design.md` §2–4, §6 (layers 1–2), §7 step 3, §8; library spec `2026-10-01-stucco-design.md` §4.7 (Bundle) and §4.8 (headers).

## Global Constraints

- Edition 2024, `rust-version = "1.85"`; no post-1.85 syntax.
- `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`; every public item documented with a compiling example where it is a type or function users call.
- `stucco-tower` features: `default = ["axum", "tower-http"]`. With `--no-default-features` it depends only on `stucco-core`, `tower`, `http`, `http-body`, `http-body-util`, `bytes`.
- The component crates (`stucco`, `stucco-core`, `stucco-ui`, `stucco-theme`) gain no HTTP or async dependency.
- Header names come from `stucco_core::behavior` (`HEADER_REQUEST`, `HEADER_TARGET`); the fragment request value is `fragment`, compared case-insensitively.
- Every HTML response sets `Content-Type: text/html; charset=utf-8`, `Vary: Stucco-Request`, `X-Content-Type-Options: nosniff`. Every asset response sets `X-Content-Type-Options: nosniff`.
- Commits: plain messages, no AI attribution.
- Gate after every task: `cargo +stable fmt --all --check`, `cargo +stable clippy --workspace --all-targets -- -D warnings`, `cargo +stable test --workspace`, `cargo +stable hack check -p stucco-tower --each-feature --no-dev-deps`; plus `npx playwright test` from Task 5 on.

## Review Focus

1. **Conditional requests** — `If-None-Match` with the exact ETag, a list (`"x", "<etag>"`), a weak tag (`W/"<hash>"`) or `*` yields 304 with the ETag and caching headers and an empty body; a non-matching tag yields 200. Test in Task 1.
2. **Odd asset paths** — query strings are ignored; percent-encoded names (`/_stucco/stucco%2E<h>.css`), `..` segments and unknown names under the prefix yield 404 (never decoded, never passed to the fallback); paths outside the prefix go to the fallback. Test in Task 1.
3. **Fallback readiness** — `AssetService::fallback(inner)` reports `Pending` while `inner` is not ready and never calls `inner` for asset paths. Test in Task 1.
4. **Malformed negotiation headers** — `Stucco-Request: FRAGMENT` counts as a fragment request; a missing, empty, non-UTF-8 or invalid `Stucco-Target` (`"a b"`, `"x\"y"`, `"1abc"`) yields `RequestKind::Full`. Test in Task 3.
5. **Status and headers survive conversion** — a 422 or 409 `PageResponse`/`FragmentResponse` keeps its status through `into_response()` and axum `IntoResponse`, with all three required headers. Test in Task 2.

---

### Task 1: Crate scaffold and `AssetService`

**Files:**
- Create: `crates/stucco-tower/{Cargo.toml, src/lib.rs, src/assets.rs}`
- Modify: `Cargo.toml` (workspace deps: `tower`, `http`, `http-body`, `http-body-util`, `bytes`, `axum = { version = "0.8.9", default-features = false }`, `tower-http = "0.7.1"`, `tokio = "1.53"`, `tracing = "0.1"`); `crates/stucco-core/src/bundle.rs` (add `pub fn url_prefix(&self) -> &str`)

**Interfaces:**
- Consumes: `stucco_core::{Bundle, AssetFile}`.
- Produces:
  - `pub struct AssetService { bundle: Arc<Bundle> }` (`Clone`); `AssetService::new(bundle: Arc<Bundle>) -> Self`; `impl<B> Service<Request<B>> for AssetService` with `Response = Response<Full<Bytes>>`, `Error = Infallible`, `Future = std::future::Ready<…>`, always ready.
  - Behaviour: method GET or HEAD else 405 with `Allow: GET, HEAD`; `bundle.get(path_and_query)` (path only; no percent-decoding) → 200 with `Content-Type`, `ETag`, `Cache-Control: public, max-age=31536000, immutable`, `Content-Length`, `X-Content-Type-Options: nosniff`; HEAD: same headers, empty body; unknown → 404 (`text/plain`, body `not found`). `If-None-Match` (comma-separated list; `W/` prefix ignored; `*` matches) → 304 with `ETag` + `Cache-Control`, empty body.
  - `AssetService::fallback<S>(self, inner: S) -> WithFallback<S>`; `WithFallback<S>: Service<Request<B>>` where `S: Service<Request<B>, Response = Response<ResBody>>`, `ResBody: http_body::Body<Data = Bytes>`; `Response = Response<http_body_util::Either<Full<Bytes>, ResBody>>`, `Error = S::Error`; `poll_ready` delegates to `inner`; paths starting with `bundle.url_prefix()` are answered by the asset logic (including 404/405), all others go to `inner`.
- Dev-deps: `tokio` (`macros`, `rt`), `tower` (`util`).

- [ ] **Step 1: Write the failing tests** (`assets.rs` tests module; helper `async fn get(svc, method, uri, headers) -> Response`):

```rust
#[tokio::test]
async fn serves_hashed_assets_with_immutable_caching() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let res = AssetService::new(bundle.clone()).oneshot(req("GET", &format!("{url}?v=1"))).await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "text/css; charset=utf-8");
    assert_eq!(res.headers()["cache-control"], "public, max-age=31536000, immutable");
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    assert_eq!(body(res).await, bundle.css().as_bytes());
}

#[tokio::test]
async fn conditional_requests_return_304() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let etag = bundle.get(&url).unwrap().etag;
    for inm in [etag.clone(), format!("\"x\", {etag}"), format!("W/{etag}"), "*".to_owned()] {
        let res = AssetService::new(bundle.clone()).oneshot(req_with("GET", &url, "if-none-match", &inm)).await.unwrap();
        assert_eq!(res.status(), 304, "{inm}");
        assert_eq!(res.headers()["etag"], etag.as_str());
        assert!(body(res).await.is_empty());
    }
    let res = AssetService::new(bundle).oneshot(req_with("GET", &url, "if-none-match", "\"nope\"")).await.unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn head_has_headers_but_no_body_and_other_methods_are_405() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let head = AssetService::new(bundle.clone()).oneshot(req("HEAD", &url)).await.unwrap();
    assert_eq!(head.status(), 200);
    assert_eq!(head.headers()["content-length"], bundle.css().len().to_string().as_str());
    assert!(body(head).await.is_empty());
    let post = AssetService::new(bundle).oneshot(req("POST", &url)).await.unwrap();
    assert_eq!((post.status().as_u16(), post.headers()["allow"].to_str().unwrap()), (405, "GET, HEAD"));
}

#[tokio::test]
async fn odd_paths_under_the_prefix_are_404_and_never_fall_back() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let name = bundle.stylesheet_url().trim_start_matches("/_stucco/").to_owned();
    let encoded = name.replacen('.', "%2E", 1);
    for path in [format!("/_stucco/{encoded}"), format!("/_stucco/../{name}"), "/_stucco/nope.css".into()] {
        let res = AssetService::new(bundle.clone()).fallback(Teapot).oneshot(req("GET", &path)).await.unwrap();
        assert_eq!(res.status(), 404, "{path}");
    }
    let res = AssetService::new(bundle).fallback(Teapot).oneshot(req("GET", "/orders")).await.unwrap();
    assert_eq!(res.status(), 418, "outside the prefix goes to the fallback");
}

#[tokio::test]
async fn fallback_readiness_is_delegated() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let mut svc = AssetService::new(bundle).fallback(NeverReady);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(svc.poll_ready(&mut cx).is_pending());
}
```

(`Teapot` = a `tower::service_fn` answering 418; `NeverReady` = a test `Service` whose `poll_ready` returns `Pending`.)

- [ ] **Step 2: Run** `cargo +stable test -p stucco-tower` — expected FAIL.
- [ ] **Step 3: Implement** (and `Bundle::url_prefix` in core with a one-line test in `bundle.rs`: `assert_eq!(Bundle::new(Preset::Slate).prefix("/assets").url_prefix(), "/assets/")`).
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add stucco-tower with AssetService"`.

---

### Task 2: Page and fragment responses

**Files:**
- Create: `crates/stucco-tower/src/response.rs`

**Interfaces:**
- Consumes: `stucco_core::{RenderedFragment, Bundle}`.
- Produces:
  - `pub struct PageResponse { status: StatusCode, html: String }`; `PageResponse::new(html: String) -> Self` (200); `.status(StatusCode) -> Self`; `fn into_response(self) -> Response<Full<Bytes>>`.
  - `pub struct FragmentResponse { status, html }`; `FragmentResponse::new(fragment: &RenderedFragment, bundle: &Bundle) -> Self` (html = `fragment.to_response_html(bundle)`); `.status(StatusCode)`; `into_response()`.
  - Both set `Content-Type: text/html; charset=utf-8`, `Vary: Stucco-Request`, `X-Content-Type-Options: nosniff`, `Cache-Control: no-store` only for fragments.
  - Feature `axum`: `impl axum::response::IntoResponse` for both (delegating to `into_response`, mapping the body with `axum::body::Body::new`).

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn page_responses_are_html_and_vary_on_the_request_kind() {
    let res = PageResponse::new("<p>x</p>".into()).into_response();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "text/html; charset=utf-8");
    assert_eq!(res.headers()["vary"], "Stucco-Request");
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
}

#[test]
fn statuses_survive_conversion() {
    assert_eq!(PageResponse::new(String::new()).status(StatusCode::UNPROCESSABLE_ENTITY).into_response().status(), 422);
    let bundle = Bundle::new(Preset::Slate);
    let frag = stucco_core::render_fragment("x", &"hi");
    let res = FragmentResponse::new(&frag, &bundle).status(StatusCode::CONFLICT).into_response();
    assert_eq!((res.status().as_u16(), res.headers()["cache-control"].to_str().unwrap()), (409, "no-store"));
}

#[cfg(feature = "axum")]
#[tokio::test]
async fn axum_conversion_keeps_status_and_headers() {
    use axum::response::IntoResponse;
    let res = PageResponse::new("x".into()).status(StatusCode::CONFLICT).into_response_axum();
    assert_eq!(res.status(), 409);
    assert_eq!(res.headers()["vary"], "Stucco-Request");
}
```

(`into_response_axum` is a test-local helper calling `IntoResponse::into_response`, avoiding the name clash with the inherent method.)

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS). Commit `"Add page and fragment responses"`.

---

### Task 3: Request kind and negotiation

**Files:**
- Create: `crates/stucco-tower/src/negotiate.rs`

**Interfaces:**
- Produces:
  - `#[derive(Clone, Debug, PartialEq, Eq)] pub enum RequestKind { Full, Fragment { target: String } }`; `RequestKind::from_headers(&http::HeaderMap) -> RequestKind` (fragment when `Stucco-Request` equals `fragment` case-insensitively **and** `Stucco-Target` is valid UTF-8 matching `[A-Za-z][A-Za-z0-9_-]*`; otherwise `Full`); `RequestKind::from_parts(&http::request::Parts)`.
  - `pub fn respond(kind: &RequestKind, full: impl FnOnce() -> PageResponse, fragment: impl FnOnce(&str) -> FragmentResponse) -> Response<Full<Bytes>>` — runs exactly one closure.
  - Feature `axum`: `impl<S: Send + Sync> FromRequestParts<S> for RequestKind` with `Rejection = Infallible`.

- [ ] **Step 1: Write the failing tests:**

```rust
fn headers(pairs: &[(&str, &[u8])]) -> HeaderMap { /* build from raw bytes */ }

#[test]
fn fragment_requests_need_a_valid_target() {
    assert_eq!(RequestKind::from_headers(&headers(&[("stucco-request", b"FRAGMENT"), ("stucco-target", b"orders")])),
        RequestKind::Fragment { target: "orders".into() });
    for target in [&b""[..], b"a b", b"x\"y", b"1abc", b"\xff"] {
        assert_eq!(RequestKind::from_headers(&headers(&[("stucco-request", b"fragment"), ("stucco-target", target)])),
            RequestKind::Full, "{target:?}");
    }
    assert_eq!(RequestKind::from_headers(&headers(&[("stucco-request", b"fragment")])), RequestKind::Full);
    assert_eq!(RequestKind::from_headers(&HeaderMap::new()), RequestKind::Full);
}

#[test]
fn respond_runs_only_the_matching_branch() {
    let bundle = Bundle::new(Preset::Slate);
    let res = respond(&RequestKind::Full, || PageResponse::new("page".into()), |_| unreachable!());
    assert_eq!(res.status(), 200);
    let kind = RequestKind::Fragment { target: "t".into() };
    let res = respond(&kind, || unreachable!(), |t| FragmentResponse::new(&stucco_core::render_fragment("t", &t.to_owned()), &bundle));
    assert_eq!(res.headers()["cache-control"], "no-store");
}
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS). Commit `"Add request kinds and response negotiation"`.

---

### Task 4: Standard layers and the axum assets router

**Files:**
- Create: `crates/stucco-tower/src/axum_support.rs` (feature `axum`), `crates/stucco-tower/src/layers.rs` (features `axum` + `tower-http`)

**Interfaces:**
- Produces:
  - Feature `axum`: `pub fn assets_router(bundle: Arc<Bundle>) -> axum::Router` — routes `{prefix}{*file}` to `AssetService` (for the default prefix: `/_stucco/{*file}`), mergeable into an app router.
  - Feature `axum` + `tower-http`: `#[derive(Clone, Debug)] pub struct LayerConfig { pub body_limit: usize /* default 1 MiB */, pub timeout: Duration /* default 30 s */ }` with `Default`; `pub fn with_standard_layers(router: axum::Router, config: &LayerConfig) -> axum::Router` applying, outermost first: request id (`SetRequestIdLayer::x_request_id(MakeRequestUuid)` + `PropagateRequestIdLayer::x_request_id()`), `TraceLayer::new_for_http()`, `RequestBodyLimitLayer::new(config.body_limit)`, a timeout layer answering `408 Request Timeout` after `config.timeout` (use the tower-http 0.7 constructor that takes a status code; verify its exact name in the 0.7.1 docs and record it in the ledger).
  - `tower-http` features enabled: `trace`, `request-id`, `limit`, `timeout`; plus `tokio` `time`.

- [ ] **Step 1: Write the failing tests** (`tests/axum_app.rs`, `#[cfg(all(feature = "axum", feature = "tower-http"))]`):

```rust
fn app(bundle: Arc<Bundle>, config: &LayerConfig) -> Router {
    let routes = Router::new()
        .route("/", get(|| async { PageResponse::new("ok".into()) }))
        .route("/echo", post(|body: String| async move { body.len().to_string() }))
        .route("/slow", get(|| async { tokio::time::sleep(Duration::from_secs(60)).await; "late" }));
    with_standard_layers(routes.merge(assets_router(bundle)), config)
}

#[tokio::test]
async fn responses_carry_a_request_id_and_assets_are_routed() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let res = app(bundle.clone(), &LayerConfig::default()).oneshot(req("GET", "/")).await.unwrap();
    assert!(res.headers().contains_key("x-request-id"));
    let css = app(bundle.clone(), &LayerConfig::default()).oneshot(req("GET", bundle.stylesheet_url())).await.unwrap();
    assert_eq!(css.status(), 200);
}

#[tokio::test]
async fn oversized_bodies_are_413() {
    let config = LayerConfig { body_limit: 16, ..LayerConfig::default() };
    let res = app(Arc::new(Bundle::new(Preset::Slate)), &config)
        .oneshot(req_body("POST", "/echo", "x".repeat(17))).await.unwrap();
    assert_eq!(res.status(), 413);
}

#[tokio::test(start_paused = true)]
async fn slow_handlers_time_out_with_408() {
    let config = LayerConfig { timeout: Duration::from_secs(1), ..LayerConfig::default() };
    let res = app(Arc::new(Bundle::new(Preset::Slate)), &config).oneshot(req("GET", "/slow")).await.unwrap();
    assert_eq!(res.status(), 408);
}
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS, including `cargo hack check -p stucco-tower --each-feature --no-dev-deps`). Commit `"Add standard layers and the assets router"`.

---

### Task 5: `examples/hello` and browser checks

**Files:**
- Create: `examples/hello/{Cargo.toml, src/main.rs}`; Modify: workspace `members` (add `"examples/*"`), `browser/playwright.config.ts` (second `webServer`), Create: `browser/tests/hello.spec.ts`; Modify: `browser/tests/a11y.spec.ts` is **not** changed (hello has its own axe test)
- Modify: `README.md` (a "Quick start" with the example's core lines)

**Interfaces:**
- Produces: binary `hello` (`publish = false`) listening on `127.0.0.1:${PORT:-4180}`:
  - `GET /` → full page (`Page::new(&bundle, "Hello — stucco")`, `SkipLink`, `main#main`, `Container`, `Heading`, `Text`, a `Form::get("/")` containing `Field::new("Name", Input::text("q"))` and `Button::new("Greet").submit()`, and a `<div id="greeting">` showing "Hello, {q}!" when `?q=` is present, escaped); fragment requests targeting `greeting` → `FragmentResponse` of just the greeting rendered with namespace `greeting`.
  - Bundle: `Bundle::new(Preset::Slate)`; app = `with_standard_layers(routes.merge(assets_router(bundle)), &LayerConfig::default())`.
- Playwright: second `webServer` entry `cargo run -q -p hello` with `env: { PORT: "4180" }`, `url: "http://localhost:4180/"`; `hello.spec.ts` uses `test.use({ baseURL: "http://localhost:4180" })`.

- [ ] **Step 1: Write the failing browser tests** (`browser/tests/hello.spec.ts`):

```ts
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test.use({ baseURL: "http://localhost:4180" });

test("the page renders with its linked stylesheet and works without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto("/");
  const href = await page.locator('link[rel="stylesheet"]').getAttribute("href");
  expect(href).toMatch(/^\/_stucco\/stucco\.[0-9a-f]{10}\.css$/);
  await page.getByLabel("Name").fill("Ada <3");
  await page.getByRole("button", { name: "Greet" }).click();
  await expect(page.locator("#greeting")).toHaveText("Hello, Ada <3!");
  await context.close();
});

test("assets answer 200, 304, 404 and 405", async ({ request, page }) => {
  await page.goto("/");
  const href = (await page.locator('link[rel="stylesheet"]').getAttribute("href"))!;
  const ok = await request.get(href);
  expect(ok.status()).toBe(200);
  expect(ok.headers()["cache-control"]).toBe("public, max-age=31536000, immutable");
  expect((await request.get(href, { headers: { "if-none-match": ok.headers()["etag"] } })).status()).toBe(304);
  expect((await request.get("/_stucco/nope.css")).status()).toBe(404);
  expect((await request.post(href)).status()).toBe(405);
});

test("fragment requests get only the target, uncached, varying on the request kind", async ({ request }) => {
  const res = await request.get("/?q=Grace", { headers: { "Stucco-Request": "fragment", "Stucco-Target": "greeting" } });
  expect(res.headers()["vary"]).toBe("Stucco-Request");
  expect(res.headers()["cache-control"]).toBe("no-store");
  const html = await res.text();
  expect(html).toContain("Hello, Grace!");
  expect(html).not.toContain("<html");
});

test("the page has no axe violations", async ({ page }) => {
  await page.goto("/?q=Ada");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});
```

- [ ] **Step 2: Run** `cd browser && npx playwright test tests/hello.spec.ts` — expected FAIL (no server).
- [ ] **Step 3: Implement** the example and the Playwright config entry; add the README quick start.
- [ ] **Step 4: Run the full gate** (Rust gate, release tests, `cargo hack`, full `npx playwright test`) — expected PASS. Commit `"Add the hello example and browser checks for stucco-tower"`.
