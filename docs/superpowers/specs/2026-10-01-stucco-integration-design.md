# stucco — Tower integration design

Date: 2026-10-01 · Status: draft for review

Companion to `2026-10-01-stucco-design.md` (the library spec, "LS" below). Scope:
the **thin integration** needed by roadmap steps 3–6 (LS §14), designed so the later
framework can grow from it. Everything for step 8 and beyond is listed as candidate
vocabulary only (§9) and needs its own scope review.

## 1. Scope

In scope:

- Serving `Bundle` assets and full pages through Tower.
- Response negotiation between full pages and fragments on the same endpoints.
- Typed collection queries and a `CollectionSource` contract (step 4).
- Record loading, validation, a typed `Action` and form-state redisplay, CSRF and
  authorization placement (step 5).
- Enhanced-mode responses (step 6).
- A reference application on axum and an embedded key/value store.
- `stucco-redb`, a first-party storage adapter, so new projects are ready to go
  without a database server (§5.5).

Out of scope here: a router, a framework facade, identity implementations (the
integration composes existing crates), uploads, jobs, live transports, idempotency
storage.

**Long-term goal (framework spec):** a batteries-included framework the user can
start every future project from. This integration is its first layer, so defaults
(pipeline, storage, sessions, CSRF) are chosen to work out of the box and stay
replaceable.

## 2. Crate and ecosystem stance

- Crate `stucco-tower`, depending on `stucco-core`, `tower` 0.5, `http` 1,
  `http-body` 1, `bytes`; optional features `axum` (axum 0.8) and `tower-http`
  (0.7). Versions current as of 2026-10-01; re-verify when step 3 starts.
- **Reuse before inventing.** HTTP types come from `http`; middleware is Tower
  `Layer`/`Service`; request extraction and response conversion in this phase use
  axum's `FromRequestParts`/`FromRequest`/`IntoResponse` behind the `axum` feature,
  plus plain functions over `http::request::Parts` for other hosts. stucco defines no
  extraction trait of its own until the framework spec shows a need axum cannot meet.
- Sessions: `tower-sessions`. Tracing, request ids, body limits, timeouts:
  `tower-http`. The application supplies its identity layer.
- `stucco` (the component facade) does not depend on this crate.

## 3. Asset and page serving (step 3)

- `AssetService::new(bundle: Arc<Bundle>)` implements
  `Service<Request<B>, Response = Response<Full<Bytes>>, Error = Infallible>`:
  GET/HEAD only (405 otherwise), `ETag` + `If-None-Match` → 304,
  `Cache-Control: public, max-age=31536000, immutable`, `Content-Type` from the
  bundle, 404 for unknown paths. `.fallback(inner)` forwards non-asset paths to
  `inner`, preserving its readiness (`poll_ready` delegates to `inner`).
- Responses: `PageResponse(String)` and `FragmentResponse { fragment, bundle }`
  convert to `Response` (`text/html; charset=utf-8`, `Vary: Stucco-Request`); with
  `axum`, both implement `IntoResponse`.

## 4. Request kinds and negotiation (steps 3 and 6)

```rust
pub enum RequestKind { Full, Fragment { target: String } }
impl RequestKind { pub fn from_parts(parts: &http::request::Parts) -> RequestKind }
```

Derived from `Stucco-Request: fragment` and `Stucco-Target` (target validated as an
id). Handlers run the **same operation** for both kinds and only choose the output
afterwards:

```rust
respond(kind, || full_page(&data), |target| fragment_for(target, &data))
```

Outcome-to-response mapping is fixed so full and enhanced modes stay equivalent:

| Outcome | Full request | Fragment request |
|---|---|---|
| success, read | 200 page | 200 fragment |
| success, mutation | 303 to the result URL | 204 + `Stucco-Location`, or 200 fragment when the target is updated in place |
| invalid input | 422 page with `FormState` | 422 fragment with `FormState` |
| conflict | 409 page with current record | 409 fragment with current record |
| not found | 404 page | 404 (runtime shows region failure) |
| denied | 403 page | 403 (runtime shows region failure) |
| infrastructure error | 500 page from `ErrorPresenter` | 500 (runtime shows region failure) |

Query and validation state survive both modes: collections re-encode
`CollectionQuery` in every link and form; invalid submissions redisplay `FormState`.

## 5. Data contracts (steps 4–5)

Only contracts exercised by the slices are specified; each is a trait because
several storage backends must implement it. State and configuration are concrete
types from LS §6.

### 5.1 Async shape

Traits use return-position `impl Future<Output = …> + Send` (stable since Rust 1.75,
within MSRV 1.85). They are used generically in this phase; trait objects are not
supported until a boxed adapter is justified. Implementations are `Send + Sync +
'static` and are shared via `Arc`. Cancellation is drop: a source may be dropped
mid-query at any await point and must not leave partial writes (reads only).

### 5.2 RequestContext

```rust
pub struct RequestContext {
    pub request_id: RequestId,
    pub locale: Option<String>,
    extensions: http::Extensions,      // principal and app data, typed access
}
impl RequestContext { pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<&T> }
```

Built once per request from `Parts`; passed by reference to sources and actions so
backend traits are not threaded as generics through UI types.

### 5.3 CollectionSource (step 4)

```rust
pub trait CollectionSource: Send + Sync + 'static {
    type Row: Send;
    fn capabilities(&self) -> Capabilities;
    fn query(&self, q: &CollectionQuery, cx: &RequestContext)
        -> impl Future<Output = Result<CollectionPage<Self::Row>, SourceError>> + Send;
}
```

Designed for key/value stores first:

- `Capabilities` declares which columns sort, which filter, whether search exists,
  and whether a `total` count is available; the DataTable hides everything else and
  `CollectionQuery::parse` drops unsupported parameters.
- Key-ordered stores use `Window::After/Before(Cursor)`; cursors are opaque,
  URL-safe encodings of the last key. Offset windows remain for sources that support
  them cheaply.
- `total: None` renders "Showing 1–25" with next/previous links instead of numbered
  pages.
- `CollectionQuery::apply(&mut Vec<T>)` serves small in-memory or fully scanned
  collections.

### 5.4 RecordSource, Validate, Action (step 5)

```rust
pub trait RecordSource: Send + Sync + 'static {
    type Id; type Record: Send;
    fn get(&self, id: &Self::Id, cx: &RequestContext)
        -> impl Future<Output = Result<Option<Versioned<Self::Record>>, SourceError>> + Send;
}
pub struct Versioned<T> { pub value: T, pub version: Version }   // Version: opaque bytes

pub trait Validate { fn validate(&self) -> Result<(), FormErrors>; }
```

**Actions are Tower services.** An action is
`Service<ActionRequest<I>, Response = Outcome<O>, Error = InfraError>` where
`ActionRequest { input: I, expected_version: Option<Version>, cx: RequestContext }`
and

```rust
pub enum Outcome<O> { Done(O), Invalid(FormErrors), NotFound, Conflict { current: Option<Version> }, Denied }
```

- `action_fn(f)` adapts an `async fn(ActionRequest<I>) -> Result<Outcome<O>, InfraError>`.
  It is always ready and documents that it applies no backpressure; applications add
  `tower::limit::ConcurrencyLimitLayer` or `BufferLayer` where needed. Adapters that
  wrap an inner service delegate `poll_ready` and never report ready on its behalf.
- Services must be `Clone + Send + 'static` with `Send` futures (as axum requires).
- Expected outcomes are values; `InfraError` is for failures of the infrastructure
  and is never shown to users verbatim (§6).
- Mutations are not retried. A retry policy may only wrap actions that declare an
  idempotency contract (later, §9).
- Cancellation: a client disconnect drops the response future. Actions whose effects
  must complete regardless spawn the work and return; the reference example documents
  which choice it makes.

**Conflicts with a k/v store.** Records are stored with a version and updated with
compare-and-swap; a mismatch yields `Outcome::Conflict`, rendered as the current
record next to the submitted values.

### 5.5 `stucco-redb` (steps 4–5)

The default storage adapter, on `redb` (pure Rust, embedded, ACID, stable 4.x;
re-verify the version when step 4 starts). The contracts stay storage-agnostic; other
pure-Rust embedded stores (fjall, sled) can get adapters later.

- `Store::open(path)`; typed tables via a `Table<K, V>` wrapper with
  serialization behind a small `Codec` trait (default implementation: `postcard` +
  `serde`).
- `VersionedTable<K, V>`: `get -> Option<Versioned<V>>`, `insert`,
  `update(key, expected: Version, value) -> Result<Version, Conflict>` in one write
  transaction.
- `scan(range, window: &Window) -> CollectionPage<V>` with key cursors, and
  `IndexTable` for secondary sort orders maintained in the same transaction.
- A `tower_sessions::SessionStore` implementation (sessions table with expiry and a
  `delete_expired` maintenance call).
- A `CollectionSource` helper: implement `capabilities` and a key/index mapping; the
  helper does windowing, cursor encoding and in-scan filtering.

Blocking I/O: redb calls are synchronous; the adapter runs them on
`tokio::task::spawn_blocking` behind an optional `tokio` feature (default on), and
exposes the synchronous API for other runtimes.

## 6. Pipeline and security (steps 3–5)

Layer order, outermost first:

1. Tracing and request id (`tower-http` `TraceLayer`, `SetRequestIdLayer`).
2. Request body limit (`RequestBodyLimitLayer`, default 1 MiB) and timeout.
3. Sessions (`tower-sessions`, cookie store or a k/v-backed store).
4. Identity: application layer inserting a principal into request extensions.
5. CSRF for unsafe methods (POST) on cookie-authenticated requests:
   - accept when `Sec-Fetch-Site` is `same-origin` or `none`;
   - otherwise, when `Sec-Fetch-Site` is absent, accept when `Origin` matches a
     configured origin;
   - otherwise require a synchronizer token stored in the session and submitted via
     `CsrfToken` (form field) or the `Stucco-Csrf` header (runtime);
   - reject with 403 and the ErrorPresenter page.
6. Routing (axum `Router` in the reference app).
7. In the handler: extract and parse input → load the record → **authorize against
   the loaded resource** (`Authorizer`-style check written by the application; not
   generic middleware, since it needs the record) → validate → call the action →
   negotiate the response (§4).

`ErrorPresenter` renders a safe page or fragment for `InfraError`: a generic message
plus the request id; the error itself goes only to tracing. Body and upload limits
stay at the request boundary.

Streaming responses (later) need two policies: a timeout for producing the response
head and a separate idle/lifetime policy for the stream.

## 7. Slices and acceptance tests

Each slice extends one reference application, `examples/orders` (axum +
`stucco-redb`).

| Step | Delivers | Acceptance (Playwright + Rust integration tests) |
|---|---|---|
| 3 | AssetService, PageResponse, the pipeline skeleton | assets 200/304/404/405 with correct headers; page renders with linked CSS; no-JS page works |
| 4 | `stucco-redb` tables, scans, cursors, CollectionSource helper; orders list: AppShell, PageHeader, server DataTable over `CollectionSource`, cursor and numbered pagination | sort/filter/search via GET without JS; query preserved across pages; unsupported parameters ignored; axe clean; keyboard reachable |
| 5 | `stucco-redb` versioned updates and session store; order edit: ValidatedForm, FormState redisplay, ErrorSummary, CSRF, authorization, conflict | invalid submit → 422 with values and focusable summary; password-like fields not redisplayed; cross-site POST rejected; other user's order → 403; concurrent edit → 409 view; no-JS submission works |
| 6 | Enhanced mode on the same endpoints | table updates in place; concurrent fragments load a new behaviour module once; an icon and behaviour absent from the initial page work after insertion; stale responses discarded; focus restored; 422 fragment focuses summary; htmx smoke test |

## 8. Testing approach

- Rust: `tower::ServiceExt::oneshot` tests for AssetService, negotiation and the
  outcome mapping table; CSRF layer tests for each header combination.
- Playwright against the running example in Chromium, Firefox and WebKit, with and
  without JavaScript.

## 9. Candidate vocabulary for later scope (not specified)

To be designed in the framework spec after review, introducing traits only where
several implementations justify them: `Endpoint` (typed URL generation producing LS
`Action`s), `SuggestionSource`, `FacetSource`, `UploadHandler`/`DownloadSource`,
`SessionStore` integration, `FlashStore`, `EventSource` (with optional resumption),
`JobSource`/`JobSubmitter`/`JobCanceller`, `NotificationSource`, `IdempotencyStore`.
Mappings to remember: Combobox ← SuggestionSource + Endpoint; InlineEdit ←
RecordSource + Action + Versioned; RemoteDialog ← Endpoint + fragment rendering;
NotificationCenter ← snapshot source + Action + optional EventSource; JobProgress ←
JobSource + optional JobCanceller/EventSource.

## 10. Decisions for review

1. Crate name `stucco-tower`; not part of the `stucco` facade.
2. axum is the reference host and supplies extraction/response traits for now.
3. CSRF via Fetch Metadata / Origin checks with a session token fallback.
4. Generic (non-object-safe) async traits with `Send` futures in this phase.
5. Actions as Tower services with an explicit `Outcome`; `action_fn` always ready,
   backpressure opt-in via standard layers.
6. Outcome → HTTP status mapping in §4 (422 for invalid, 409 for conflict, 303/204
   for successful mutations).
7. Default storage: `stucco-redb` (redb, versioned records with compare-and-swap,
   key cursors, session store), built in steps 4–5; storage stays swappable.
8. `stucco-redb` depends on serde + postcard for its default codec and on tokio
   (optional, default on) for `spawn_blocking`.
