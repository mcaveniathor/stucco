# Redb-backed Orders List Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver roadmap step 4: a reusable server-mode collection UI and a working orders list backed by redb, including GET search, filtering, sorting, and cursor/numbered pagination without JavaScript.

**Architecture:** Framework-independent collection state lives in stucco-core; components consume it in stucco-ui. stucco-tower provides RequestContext and CollectionSource. A new stucco-redb crate supplies synchronous typed storage, transactional indexes, and an optional Tokio collection adapter; examples/orders composes them through axum.

**Tech Stack:** Rust edition 2024; component/Tower MSRV 1.85, storage adapter/orders MSRV 1.90; existing Tower/axum versions; latest stable redb (4.3.0 verified for this plan), serde 1, postcard 1 with alloc, base64 0.22, form_urlencoded 1, tempfile 3 for tests. Re-verify latest stable redb at implementation time; preserve the lockfile and verify each package's declared MSRV before committing dependencies.

**Spec:** `docs/superpowers/specs/2026-10-01-stucco-design.md` §6–8 and §14 step 4; `docs/superpowers/specs/2026-10-01-stucco-integration-design.md` §5.1–5.3, §5.5 (read-side storage), and §7 step 4. Read both specs and this plan before execution.

## Scope review and proposed spec clarifications

This is a proposed plan for review, not approval to implement. Apply the following clarifications to the specs after approval and before freezing public interfaces:

1. **Use latest stable redb, as requested.** [redb 4.3.0 declares Rust 1.90](https://raw.githubusercontent.com/cberner/redb/v4.3.0/Cargo.toml). Use `redb = "4.3"` at the currently verified release, and re-verify the latest stable release before adding the dependency. Set explicit `rust-version = "1.90"` for stucco-redb and orders; retain workspace inheritance of 1.85 for component/Tower packages. If a newer stable redb needs a higher compiler, update only storage consumers and their MSRV job. Explicitly document database-format version and migration obligations; avoid experimental APIs/features.
2. **One page size for both windows.** `CollectionQuery.per_page` owns page size; `Window::Offset { page }`, `After(Cursor)`, `Before(Cursor)` select position. This changes the illustrative Offset shape to eliminate duplicate size state. Default page size 25, maximum 100; page numbers are one-based.
3. **Unknown totals and cursors.** Cursor result text says "Showing N results", not a fabricated absolute range. Show numbered links only for offset results with a total. Add `Capabilities.offset: bool` so parsers/UI do not advertise expensive offsets by accident.
4. **Filter metadata.** Capabilities authorizes column operations; typed column/filter declarations supply enum choices and numeric/date bounds. Reject unknown kinds, non-finite numbers, invalid ISO dates, or reversed ranges. Parse failures drop the affected filter, not the entire request. Custom columns have no implicit sort/filter semantics.
5. **Bounded examples, honest costs.** Numbered pagination and total counts use a deliberately small, fully scanned example mode. Large-store mode uses key/index cursors and `total: None`. Search and non-indexed filters may scan records; document this instead of claiming database-indexed search.
6. **Keep phases explicit.** Step 4 contains Card, Panel, Table/Row, ResultCount, SortControl, PageSizeSelect, Pagination, SearchForm, FilterBar, CollectionToolbar, EmptyState, static LiveRegion, AppShell, PageHeader, SectionHeader, and Footer. No sessions, VersionedTable, editing, mutations in HTTP handlers, CSRF, client DataTable mode, partial updates, or streaming in this slice.

The example is a local public demo with seeded nonsensitive data. It is not an authenticated application template. Request-context plumbing prepares for later identity work without inventing authentication now.

## Global Constraints

- Rust edition 2024; component/Tower packages retain `rust-version = "1.85"` and no post-1.85 APIs or syntax. stucco-redb/orders explicitly declare Rust 1.90 for the currently verified redb release.
- Public crates use `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]`; callable public APIs have compiling rustdoc examples.
- Components remain independent of Tower, HTTP, Tokio, redb, and serde. Small URL query encoding dependencies are allowed in core.
- Use existing `Render`, `Slot<'a>`, `Attrs`, passthrough/reserved-attribute conventions, and inventory asset registration.
- Rendering never reads storage or performs a backend operation.
- All text/URLs use existing safe rendering; never emit trusted markup to render database values.
- CSS uses semantic tokens and existing stucco layers; logical properties, narrow-width layouts, reduced motion, and light/dark schemes remain supported.
- Every listed component renders useful native HTML without scripts; this slice emits no new behavior modules.
- Actions on the orders HTTP endpoint are GET reads only. Seed and index maintenance happen during startup/setup, not GET handling.
- Request failures render generic user-facing text plus request ID; internal errors go to tracing.
- Commits and PRs have no AI attribution. Do not publish crates or add a remote.
- Execution starts in an isolated branch/worktree from the merged Tower integration or its exact reviewed head if it is still pending; inspect existing artifacts first. Do not continue adding feature commits to the already-open Tower PR.

## Review Focus

1. Cursor query changes: changing search, filters, sort, direction, or page size resets position; a stale or malformed cursor cannot crash or expose unrelated index data (Tasks 1, 5, 8).
2. Duplicate sort values and reverse traversal: tie-breaking by primary key avoids missing/duplicate rows; Before returns display order rather than scan order (Tasks 4–5).
3. Filtering during a scan: page boundaries and next/previous links use matching records, not raw scan counts; deleted anchors still work by exclusive range (Task 5).
4. Empty/out-of-range collections: no "1–0", overflow, bogus next link, or division by zero; unsupported capabilities never produce active UI controls (Tasks 1, 6–8).
5. Untrusted data/query strings: escaping, bounded page/query values, correctly encoded links, accessible labels, and preserved filter state across GET navigation (Tasks 1, 6–9).

## Files and ownership

| Area | Files to create | Existing files to modify |
|---|---|---|
| Core query/state | `crates/stucco-core/src/collection/{mod,query,cursor,filter,tests}.rs` | core `src/lib.rs`, `Cargo.toml`; workspace dependencies |
| Tower source contract | `crates/stucco-tower/src/collection.rs`, `tests/collection.rs` | tower `src/lib.rs` |
| Redb adapter | `crates/stucco-redb/Cargo.toml`, `src/{lib,codec,store,table,index,scan,collection,error}.rs`, `tests/{storage,index,scan,collection}.rs` | workspace dependencies; lockfile |
| Display primitives | `crates/stucco-ui/src/data/{mod,table,panel,result_count,tests}.rs`, `css/data.css` | UI/facade features, exports, asset list |
| Collection UI | `crates/stucco-ui/src/collections/{mod,column,toolbar,data_table,tests}.rs`, `src/navigation/{mod,pagination}.rs`, `src/feedback/{mod,empty_state}.rs`, `css/{collections,navigation,feedback}.css` | forms exports for SearchForm/FilterBar if housed there; passthrough cfg |
| Shells | `crates/stucco-ui/src/app/{mod,shell,header,footer,tests}.rs`, `css/app.css` | UI/facade feature exports, asset list |
| Reference app | `examples/orders/Cargo.toml`, `src/{lib,main,model,source,view}.rs`, `tests/http.rs` | workspace dependencies if needed; README |
| Gallery/browser/CI | `gallery/src/{data_page,collections_page,app_page}.rs`, `browser/tests/orders.spec.ts` | gallery exports/index/snapshots, browser config, `.github/workflows/ci.yml` |

Existing wildcard workspace membership includes new crates/examples. Do not refactor unrelated files. Public modules may use private helpers rather than expanding the public type inventory.

## Task 1: Framework-independent collection query model

**Files:** core collection files and exports; core/workspace dependency declarations.

**Interfaces produced:**

```rust
pub enum Direction { Asc, Desc }
pub enum Window { Offset { page: u64 }, After(Cursor), Before(Cursor) }
pub struct Cursor(String); // new(&str) -> Option<Self>; as_str() -> &str
pub struct Capabilities {
    pub sortable: Vec<String>, pub filterable: Vec<String>,
    pub searchable: bool, pub total_count: bool, pub offset: bool,
}
pub enum ColumnKind { Text, Number, Date, Enumeration(Vec<String>), Custom }
pub struct ColumnSpec { pub key: String, pub kind: ColumnKind }
pub enum Filter {
    Text(String), Enumeration(String),
    Number { min: Option<f64>, max: Option<f64> },
    Date { min: Option<String>, max: Option<String> },
}
pub struct CollectionQuery {
    pub sort: Option<String>, pub direction: Direction, pub search: String,
    pub filters: std::collections::BTreeMap<String, Filter>,
    pub per_page: u16, pub window: Window,
}
pub struct CollectionPage<T> {
    pub rows: Vec<T>, pub next: Option<Cursor>, pub prev: Option<Cursor>,
    pub total: Option<u64>,
}
// CollectionQuery: Default, Clone, Debug, PartialEq;
// parse(raw: &str, caps: &Capabilities, columns: &[ColumnSpec]) -> Self
// to_query_string(&self) -> String
// with_sort(self, key: &str, direction: Direction) -> Self
// with_search(self, value: &str) -> Self
// with_filter(self, key: &str, value: Option<Filter>) -> Self
// with_per_page(self, per: u16) -> Self
// with_window(self, window: Window) -> Self
// link(&self, action: &Href) -> Href
```

- [ ] Add tests using a literal Capabilities and ColumnSpec list: parse known sort/search/filter values, reject unsupported columns and malformed percent encodings, round-trip reserved Unicode/ampersand text, validate Gregorian ISO dates, reject NaN/infinity and reversed bounds.
- [ ] Use query names `sort`, `dir`, `q`, `f.<key>`, `f.<key>.min`, `f.<key>.max`, `page`, `per`, `after`, `before`. Last valid occurrence wins for scalar parameters; conflicting after/before clears position. Limit raw query processing to 16 KiB, search to 256 characters, filter text to 256 characters, cursors to 4096 ASCII URL-safe characters. Oversized fields are ignored. Never panic on input.
- [ ] Write this regression before the builder implementation:

```rust
#[test]
fn query_changes_reset_the_window() {
    let q = CollectionQuery::default()
        .with_window(Window::After(Cursor::new("YWJj").unwrap()));
    assert_eq!(q.clone().with_search("Ada & Co").window,
        Window::Offset { page: 1 });
    assert_eq!(q.with_per_page(0).per_page, 25);
}
```

- [ ] Run `cargo +stable test -p stucco-core collection` and observe failures before implementing types/parser/builders.
- [ ] Implement parsing with form_urlencoded, deterministic serialization, validated filters, and Href links. Merge existing action query parameters; collection-owned keys are replaced, unrelated keys and URL fragment survive. A GET form must carry unrelated action-query values as hidden inputs because browsers replace the action query on submission.
- [ ] For unsupported offset mode, normalize incoming `page` to first position. Page zero/overflow defaults to one; invalid `per` defaults to 25 and valid values clamp to 100. Cursor navigation preserves per_page.
- [ ] Run core tests and Rust 1.85 check; commit `Add collection queries and presentation state`.

## Task 2: RequestContext and CollectionSource

**Files:** Tower collection module/tests and exports.
**Consumes:** Task 1 state.
**Produces:** the integration spec's generic `CollectionSource` with `type Row: Send`, `capabilities`, and `query(&self, &CollectionQuery, &RequestContext) -> impl Future<Output = Result<CollectionPage<Self::Row>, SourceError>> + Send`.

RequestContext fields: `request_id: String`, `locale: Option<String>`, private cloned `http::Extensions`. `from_parts(&http::request::Parts)` reads the already-sanitized request ID; missing ID becomes an empty string. Locale defaults to None, set explicitly by the application. `get<T: Clone + Send + Sync + 'static>(&self) -> Option<&T>`. Under axum, implement infallible FromRequestParts. SourceError owns `Box<dyn std::error::Error + Send + Sync>` with new, Display, Error; its text is diagnostic only.

- [ ] Write a fake source returning two rows; await it in a Tokio test and verify capabilities/result. Test typed extensions and request ID extraction without changing parts. Add a compiler assertion that its returned future is Send:

```rust
fn assert_send<T: Send>(_: T) {}
// Inside the fake-source test, before the awaited query:
assert_send(source.query(&query, &cx));
```

- [ ] Run `cargo +stable test -p stucco-tower --test collection` to see missing-contract failures.
- [ ] Implement the contract and extraction without adding an alternative request-extraction trait or requiring Tokio in the no-default-feature public contract.
- [ ] Test all Tower feature combinations and rustdocs; commit `Add collection sources and request context`.

## Task 3: Typed synchronous redb storage and codec

**Files:** new adapter crate, codec/store/table/error modules, storage tests.
**Produces:**

```rust
pub trait Codec<T>: Send + Sync + 'static {
    fn encode(&self, value: &T) -> Result<Vec<u8>, StoreError>;
    fn decode(&self, bytes: &[u8]) -> Result<T, StoreError>;
}
pub trait KeyCodec<K>: Codec<K> {} // encoded byte order must equal intended key order
pub struct U64Key; // fixed eight-byte big-endian, implements Codec<u64>, KeyCodec<u64>
pub struct Utf8Key; // implements Codec<String>, KeyCodec<String>
pub struct PostcardCodec<T>(std::marker::PhantomData<fn() -> T>);
pub struct Store; // Clone, Arc<redb::Database> internally
pub struct Table<K, V, KC = U64Key, VC = PostcardCodec<V>>;
// Store::open(path: impl AsRef<Path>) -> Result<Self, StoreError>
// Store::table<K,V,KC,VC>(&self, name: &str, keys: KC, values: VC)
//     -> Result<Table<K,V,KC,VC>, StoreError>
// Table::{get(&K)->Result<Option<V>,StoreError>,
//         put(&K,&V)->Result<(),StoreError>, remove(&K)->Result<bool,StoreError>}
```

PostcardCodec is Default; encode/decode require serde Serialize + DeserializeOwned. Do not use postcard bytes as ordered keys implicitly. StoreError covers database/storage/codec/join failures, retaining underlying causes. Private redb table definitions use byte-slice keys/values. Validate table names as nonempty `[a-zA-Z0-9_-]+`, max 64 bytes; return errors on invalid names.

- [ ] Add crate manifest with default `tokio` feature, and synchronous APIs available without it. Only the tokio feature enables dependencies on stucco-tower/Tokio and the collection adapter; stucco-core may be unconditional for cursor scans. Do not re-export the adapter through the component facade.
- [ ] Write tempfile-backed tests for persistence after close/reopen, replacement/deletion, missing keys, malformed codec data, invalid table names, numeric ordering 1/2/255/256, and durable committed writes.
- [ ] Write the following round-trip first:

```rust
#[test]
fn values_survive_reopening() -> Result<(), StoreError> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("orders.redb");
    {
        let store = Store::open(&path)?;
        let rows = store.table::<u64, String, _, _>(
            "orders", U64Key, PostcardCodec::default())?;
        rows.put(&256, &"Ada".to_owned())?;
    }
    let store = Store::open(&path)?;
    let rows = store.table::<u64, String, _, _>(
        "orders", U64Key, PostcardCodec::default())?;
    assert_eq!(rows.get(&256)?, Some("Ada".to_owned()));
    Ok(())
}
```

- [ ] Run `cargo +stable test -p stucco-redb --test storage` before implementing.
- [ ] Implement write transactions with commit only after successful encoding/insertion; return owned decoded values before transaction guards drop. Keep database locking internal; document one writer, and keep transactions short.
- [ ] Run adapter tests both with defaults and no defaults, and `cargo +1.90 check -p stucco-redb`; commit `Add typed redb tables and codecs`.

## Task 4: Transactional secondary indexes

**Files:** adapter index module/tests; private table/store helpers.
**Produces:** `IndexTable<K,V,KC,VC>` built from a Table via `index(name, projection: impl Fn(&V)->Vec<u8> + Send + Sync + 'static) -> Result<IndexTable<...>, StoreError>`; get/put/remove and `rebuild() -> Result<(),StoreError>`; ordered scan support for Task 5.

Index keys are `(sort bytes, primary key bytes)` using an explicit self-delimiting encoding that preserves lexicographic ordering (escape zero bytes and terminate each field). Numeric sort projections use big-endian order-preserving bytes; text uses UTF-8 byte order, documented case-sensitive. Row primary key breaks ties. Index values store the encoded primary key. IndexTable::put/remove update the data table and remove/add index entries in the SAME redb write transaction. Initial `rebuild` reads the data table and clears/repopulates the index in one transaction.

- [ ] Write tests proving duplicate sort values retain all records, sort changes remove stale entries, deletes remove entries, rebuild handles existing records, and a codec/projection failure leaves data and index unchanged. Projection is infallible in this API; simulate encode failure with a deliberately rejecting value codec.
- [ ] Run `cargo +stable test -p stucco-redb --test index`, observe failures, then implement encoding and transaction helpers.
- [ ] Test ascending and descending ties using values `(name="Ada", id=1)` and `(name="Ada", id=2)`; assert ascending `[1,2]` and descending `[2,1]` in scan tests once Task 5 lands.
- [ ] Document that writes through the underlying unindexed Table do not maintain an IndexTable. The application must write exclusively through IndexTable once configured; do not expose mutable raw redb handles.
- [ ] Run storage/index tests and Clippy; commit `Add transactional secondary indexes`.

## Task 5: Cursor scans and async collection adapter

**Files:** adapter scan/collection modules and tests.
**Consumes:** Tasks 1–4.
**Produces:**

```rust
pub struct ScanRequest {
    pub index: Option<String>, pub direction: Direction,
    pub window: Window, pub per_page: u16, pub scope: String,
}
// Table and IndexTable:
// scan(&self, request: &ScanRequest,
//      keep: impl Fn(&V) -> bool) -> Result<CollectionPage<V>, StoreError>
pub trait CollectionMapping<V>: Send + Sync + 'static {
    fn capabilities(&self) -> Capabilities;
    fn columns(&self) -> Vec<ColumnSpec>;
    fn scan_request(&self, q: &CollectionQuery) -> ScanRequest;
    fn matches(&self, value: &V, q: &CollectionQuery) -> bool;
}
pub struct RedbCollection<K,V,KC,VC,M>;
// new(table: IndexTable<K,V,KC,VC>, mapping: M) -> Self
// implements CollectionSource<Row=V> under tokio feature
```

Cursor payload is a versioned base64url-without-padding encoding of index identity, direction, scope, and anchor bytes. Scope is the canonical sort/search/filter query excluding position, incorporating per_page. Validate all fields before using an anchor. A malformed/mismatched payload falls back to the first page rather than errors. It is an opaque locator, not an authorization mechanism; all scans remain bounded to the selected table/index.

Only one secondary index is needed in this slice (customer name), with primary-key ordering as the other sort. Extend ScanRequest to support additional named indexes only once exercised; reject unknown requested index names safely.

- [ ] Build test fixtures with seven rows, duplicate names, and alternating statuses. Page forward at size two to exhaustion and backward to the beginning; assert no omissions/duplicates and correct display order for both directions.
- [ ] Test filters crossing several unmatched rows, exact full last page, empty results, deleted anchors, invalid base64, foreign index/direction/scope cursors, zero/oversized limits, and injected hostile decoded bytes.
- [ ] Pin the storage work path with a Tokio test: the mapping records `Handle::try_current()` or a thread identifier to prove the synchronous scan runs on the blocking pool rather than directly in the request task. Return owned rows; no transaction/guard crosses an await.
- [ ] Run `cargo +stable test -p stucco-redb --test scan` and `--test collection` before implementation.
- [ ] Implement exclusive anchor ranges and limit+1 matching records. Use an opposite-side existence scan in the same read transaction to decide previous/next availability. Before scans invert scan direction and reverse the selected records back into display order. Scans do not assume the anchor still exists.
- [ ] Offset windows are supported only for mappings declaring offset=true, explicitly full-scanning matching records, counting total before slicing. Use checked arithmetic; beyond-end pages produce no rows. Small mode can sort/filter a Vec using mapping-provided order, while cursor mode must use the declared physical order. Do not make a generic Vec helper guess semantics from formatted cell strings.
- [ ] Implement the adapter using `spawn_blocking` with owned cloned query and table/mapping handles. Bound concurrent scans with a configurable semaphore (default 4); acquire before spawn. Move its owned permit INTO the blocking closure so cancellation of the awaiting request does not release capacity while the scan still runs. Document that started blocking scans are not forcibly cancellable.
- [ ] Run cursor/index/adapter tests, no-default builds, adapter Rust 1.90 and core/Tower Rust 1.85 checks; commit `Add cursor scans and redb collection sources`.

## Task 6: Semantic table, panel, and shell components

**Files:** data/app modules and CSS; feature exports/asset registration; feedback/navigation foundations.
**Produces:**

- `Card<'a>::new().child(Render)` with optional header/footer slots.
- `Panel<'a>::new(title).description(Render).actions(Render).body(Render).footer(Render).level(u8)`; labelled section, heading defaults to h2.
- `Table<'a>::new(caption).header(Row).row(Row)` and `Row<'a>::new().cell(Render).header(Render)`; caption required, th scope explicit, scroll container labelled without replacing native table semantics.
- `ResultCount::new(shown: usize, total: Option<u64>, offset: Option<u64>)`; cursor/unknown total uses "Showing N results"; zero is "No results".
- `EmptyState<'a>::new(title).description(Render).actions(Render)`.
- `LiveRegion<'a>::new().child(Render)` rendering a polite, atomic status region; static on initial server pages.
- `PageHeader<'a>::new(title).description(Render).actions(Render).breadcrumbs(Render).level(u8)`; h1 default.
- `SectionHeader` with h2 default; `Footer::new().child(Render)`.
- `AppShell<'a>::new().header(Render).sidebar(Render).main(Render).footer(Render)`; one main#main and SkipLink. Sidebar is a native disclosure on narrow screens, not an ARIA menu. Nav children/labels are application-supplied.

- [ ] Write markup tests for escaped titles/cells, required captions, th scopes, heading/section labelling, named slots, one main, working skip target, unique generated IDs, zero counts, reserved attributes, declared assets.
- [ ] Add this first count regression:

```rust
#[test]
fn empty_count_has_no_impossible_range() {
    let html = stucco_core::to_html(&ResultCount::new(0, Some(0), Some(0)));
    assert!(html.contains("No results"));
    assert!(!html.contains("1–0"));
}
```

- [ ] Run UI tests before implementing.
- [ ] Implement existing Attrs/passthrough/Slot conventions and registered DATA, APP, FEEDBACK assets. `data` enables layout/typography as used; `app` enables layout/navigation/typography; reflect exact dependencies in both UI and facade features. Include each new family in ui_assets and update passthrough dead-code cfg.
- [ ] Run UI tests, CSS checks and each-feature builds; commit `Add tables panels and application shells`.

## Task 7: Typed columns and server collection compositions

**Files:** collection/navigation modules, form compositions, feature wiring, CSS/tests.
**Consumes:** Task 1 state and Task 6 components.
**Produces:**

```rust
pub struct Col<'a,T>; // Debug with closure fields described by placeholders
// Col::text(key, label, Fn(&T)->String)
// Col::number(key, label, Fn(&T)->f64)
// Col::date(key, label, Fn(&T)->String) // ISO Gregorian dates
// Col::enumeration(key, label, choices: Vec<(String,String)>, Fn(&T)->String)
// Col::custom(key, label, Fn(&T)->Slot<'a>)
// .sortable(), .searchable(), .filter(); constructors own key/label strings
pub struct DataTable<'a,T>;
// new(rows: &'a [T], caption: impl Into<String>) -> Self
// column(Col<'a,T>), query(&'a CollectionQuery),
// capabilities(&'a Capabilities), page(&'a CollectionPage<T>), action(impl Into<Href>)
```

`page` is used for totals/cursors; rows passed to new must be that page's rows. Prefer a final `DataTable::from_page(&CollectionPage<T>, caption)` convenience constructor to avoid duplicated/inconsistent state. Render applies intersection of declared column operations and capabilities. Numeric non-finite output renders a documented unavailable marker; custom cell Render output is not reparsed as markup.

Additional public compositions: SearchForm, FilterBar, CollectionToolbar, SortControl, PageSizeSelect, Pagination. Their constructors accept action Href, query, capabilities, and column specifications as needed. Put shared immutable `CollectionView<'a> { action: Href, query: &'a CollectionQuery, capabilities: &'a Capabilities, columns: &'a [ColumnSpec] }` in the collections module, and pass it to toolbar subcomponents. Pagination accepts action/query/next/prev/total separately so navigation alone does not enable the collections family.

- [ ] Write tests for aria-sort on active sortable headers, labelled GET controls, enum/min/max filters, escaped custom cells, absent unsupported controls, filter resets, preserved unrelated query params, cursor next/prev URLs, total/offset numbered links, and an empty table state retaining the toolbar.
- [ ] Run `cargo +stable test -p stucco-ui --features collections` before implementing.
- [ ] Implement URL navigation using query builders. Sort changes reset window; current sort toggles direction, new sorts start ascending. GET filter forms include sort/dir/per and unrelated parameters, omit page/after/before, and submit normally. Reset links retain unrelated parameters while clearing collection search/filters/position.
- [ ] Typed filter controls use native number/date/select inputs; search is one shared labelled field. Date filtering uses actual date comparisons; number formatting is presentation only. Implement numeric/date lower and upper bounds rather than collapsing every column filter to a string select.
- [ ] Numbered pagination renders a bounded neighborhood (first, last, up to two neighbors, ellipses) rather than one link per page. Saturating/checked counts prevent overflow; no numbered controls when offset unsupported or total missing. For cursor first page with no next/prev, omit redundant links.
- [ ] Add features `collections = ["data", "forms", "navigation", "feedback"]` to UI, forward existing facade placeholders to implemented UI families; preserve optionality and avoid enabling backend dependencies.
- [ ] Run rustdocs, CSS checks, minimal/each-feature builds; commit `Add server collection controls and DataTable`.

## Task 8: Orders application and HTTP acceptance tests

**Files:** example crate/model/source/view/main/lib, HTTP tests; README.
**Consumes:** Tasks 1–7 and existing assets_router/standard layers.
**Produces:** a runnable `orders` binary at `127.0.0.1:${PORT:-4181}`, database `${ORDERS_DB:-target/orders.redb}`; `/orders` list, `/` redirect to it, asset routes. `orders::app(db_path: &Path) -> Result<axum::Router, StoreError>` enables HTTP tests with an isolated tempfile.

Order model: `id: u64`, `customer: String`, `status: String`, `total_cents: u64`, `created: String` (ISO date). Seed 67 deterministic rows in one startup transaction only when empty. Statuses pending/paid/shipped; duplicate customer names and distinct IDs. Index customer; primary sort id. Demo small mode (`mode=pages`) enables offset/total, full-scan filtering and sorting; default cursor mode enables id/customer sorts, status/total/created filters, customer search, unknown total. Capabilities differ honestly by mode. Use integer cents for persisted values; presentation converts to currency.

- [ ] Write tempfile HTTP tests with the actual router: default GET has 25 rows, search/filter/sort operate on storage, next/prev restore row identities, mode=pages supports numbered navigation and total, invalid params/cursors are safe, unknown capabilities are ignored.
- [ ] Add escaping tests with a stored customer `<script>alert(1)</script>` and verify encoded text, no script element. Insert the fixture through a test-only seeding helper in the example, not a public mutation endpoint.
- [ ] Run `cargo +stable test -p orders --test http` before implementing.
- [ ] Implement source via RedbCollection, query parse with source capabilities, one query per request, and server-rendered DataTable in AppShell. Ensure startup seeding writes through index-maintaining transaction helpers, never an unindexed Table after index initialization.
- [ ] Use `RawQuery`, not strict typed Query extraction, so malformed collection input follows the forgiving parser contract. Return PageResponse; source errors produce generic 500 pages with request ID and detailed tracing. Do not render internal errors into HTML.
- [ ] When total mode uses vectors, document its scan cost and explicit small-data restriction in the README/example. Keep persistence across restarts; do not seed on every request or overwrite user-modified files.
- [ ] Run HTTP tests, workspace tests, and MSRV check; commit `Add the redb-backed orders example`.

## Task 9: Gallery, browser checks, CI, and final review

**Files:** gallery data/collection/app pages, browser orders tests and config, CI, README.
**Consumes:** all prior tasks.

- [ ] Add gallery pages exercising variants, narrow layouts, empty/unknown-total states, cursor/numbered pagination, and unsupported capabilities. Add index links and snapshot assertions.
- [ ] Add a third Playwright webServer at port 4181. Use a dedicated `target/orders-browser.redb`, configure PORT and ORDERS_DB, and avoid concurrent test writes to seed data. Set explicit baseURL for manually-created browser contexts.
- [ ] Write the no-JavaScript acceptance test before wiring the server:

```ts
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test.use({ baseURL: "http://localhost:4181" });

test("GET filters work without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({
    javaScriptEnabled: false, baseURL: "http://localhost:4181",
  });
  try {
    const page = await context.newPage();
    await page.goto("/orders");
    await page.getByLabel("Search orders", { exact: true }).fill("Ada");
    await page.getByLabel("Status", { exact: true }).selectOption("paid");
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page).toHaveURL(/q=Ada/);
    await expect(page).toHaveURL(/f\.status=paid/);
    const rows = page.getByRole("table", { name: "Orders", exact: true })
      .locator("tbody tr");
    expect(await rows.count()).toBeGreaterThan(0);
    for (const row of await rows.all()) {
      await expect(row).toContainText("Ada");
      await expect(row).toContainText("paid");
    }
  } finally { await context.close(); }
});

test("orders page is accessible", async ({ page }) => {
  await page.goto("/orders");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});
```

- [ ] Also test cursor links preserving query/per size, filter changes dropping after/before, customer sort ties, descending direction, numbered mode, zero results, keyboard reachability, table overflow at 320px without full-page overflow, light/dark axe, and no unexpected behavior script for a server-only DataTable.
- [ ] Run focused Playwright tests before implementing gallery/browser integration, then add server config and seed fixtures matching the tests.
- [ ] Add CI feature checks for stucco-tower and stucco-redb; no-default synchronous adapter checks. Keep a Rust 1.85 component/Tower/gallery/hello check using `cargo check --workspace --exclude stucco-redb --exclude orders --locked`, and add a Rust 1.90 `cargo check -p stucco-redb -p orders --locked` job. Keep existing component-only checks. Preserve the added Swatinem/rust-cache steps after each compiling job's toolchain setup; default keys isolate jobs/toolchains and hash manifests/lockfiles. Preserve setup-node npm caching keyed by browser/package-lock.json. Formatting needs no Cargo build cache. Continue installing Playwright browsers and OS dependencies normally.
- [ ] Run final gates: `cargo +stable fmt --all --check`; `cargo +stable clippy --workspace --all-targets -- -D warnings`; workspace debug/release tests; `cargo +1.85 check --workspace --exclude stucco-redb --exclude orders --locked`; `cargo +1.90 check -p stucco-redb -p orders --locked`; facade/UI/Tower/redb feature checks; full Playwright matrix. Verify compiler warnings in release tests and document/fix regressions introduced by this slice.
- [ ] Run a fresh whole-branch review checking transaction/index integrity, cursor boundaries, component feature isolation, malformed input, accessibility, and tests. Fix actionable findings with regressions and rerun affected checks.
- [ ] Commit `Document and test the orders collection slice`; report exact validation and limitations. Push/create PR only under the user's existing authorization for that action or a new explicit request.

## Plan self-review

- Step-4 source/storage/UI/application coverage maps to Tasks 1–9. Step-5 versioned edits/session adapter and step-6 enhanced behavior are deliberately excluded.
- Five review focus areas each have owning tests. Mutation security is not claimed for this read-only demo.
- Key ordering is separate from value serialization; transactional indexes and duplicate ties are explicit.
- Core carries only presentation/query contracts; async storage lives outside components.
- Plan proposes two public-model clarifications (page size placement and offset capability). Latest stable redb is explicitly requested; the plan separates storage MSRV from the component MSRV. User review is required before implementation freezes the public-model changes.
- A generic `CollectionQuery::apply(&mut Vec<T>)` cannot infer row accessors. In this slice, small-mode application occurs through CollectionMapping with explicit row projection/comparison. Update the illustrative spec promise accordingly rather than adding hidden reflection or coupling core to UI Col closures.
- Component client mode, icon introduction in fragments, CSRF, and sessions remain covered by their later roadmap slices, not this acceptance gate.

## Execution handoff

Review this plan and its proposed clarifications before product implementation. Recommended execution: native implementation task-by-task, with one independent whole-branch review at the end. Storage/query/UI interfaces are tightly coupled; preserving one implementation context avoids repeatedly reconstructing those contracts. Subagent-driven implementation remains an option if independent review after each task is preferred.
