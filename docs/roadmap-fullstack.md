# Stucco components for Leptos and a maintained full-stack reference

Status: development plan from 0.5.0. Steps 1–4 are implemented and locally
validated as of 26 October 2026: Leptos integration, complete Orders workflows,
default redb persistence and identity, richer components, attachments and durable
exports. Step 5 now prepares the preview package and deployment workflow;
publication and post-publication registry verification remain pending.

Stucco will provide its design system and application components for Leptos,
alongside a maintained example of a complete Rust web application. Leptos owns
reactivity, rendering, hydration, routing and server-function transport. Developers
use Leptos and `cargo-leptos` directly, with Axum, Tower and Tokio on the server.
Stucco contributes reusable components, shared themes and styles, and examples
that demonstrate how these pieces work together.

This work does not introduce a Stucco application framework, router, reactive
runtime, build CLI or ORM. The existing HTML builders, server helpers and embedded
storage remain supported. Backend integrations belong in the reference application
until concrete reuse calls for a small library addition.

## Scope and ownership

Application behavior and component interaction in the new example are authored
in Rust. Its server renders HTML, and Leptos hydrates that HTML using the compiled
WASM application. Generated `wasm-bindgen` JavaScript bindings and the normal
Leptos bootstrap are expected browser artifacts. The goal is no handwritten
application JavaScript; it is not a JavaScript-free browser delivery format.
Browser tests can continue using Playwright.

Ordinary links and forms should remain useful before hydration and when WASM is
unavailable. Enhancements such as immediate totals add interaction to that native
path. Each richer component must state which behavior requires hydration and how
loading or failure is presented.

| Boundary | Responsibility |
| --- | --- |
| Leptos and `cargo-leptos` | Components, signals, resources, actions, routing, SSR/hydration and the standard build/development workflow. Use their APIs directly. |
| `stucco-leptos` | Idiomatic Leptos components sharing Stucco's design and accessibility conventions. Separate `ssr` and `hydrate` feature builds. Independent `0.1.0-alpha.1` preview prepared; unpublished. |
| `stucco-theme` and existing CSS | Shared visual tokens and styles. Avoid a second design system or a copy of each stylesheet. |
| `stucco-core` and `stucco-ui` | Preserve existing HTML rendering and builder APIs. Share pure logic and visual assets where useful; do not convert arbitrary HTML strings into reactive components. |
| `stucco-tower` | Preserve existing middleware, request context, CSRF, flash and native response helpers. Add integration hooks only for demonstrated needs. |
| `stucco-redb` | Preserve the existing embedded-storage API and Orders example. SQL access uses its ecosystem directly. |
| `orders-fullstack` shared code | Public inputs/results, fixed-width IDs and revisions, integer money calculations and validation used by native and browser builds. |
| `orders-fullstack` server code | Application services, storage, identity, uploads and export workers. Keep server-only dependencies and private state out of the WASM build. |

The existing `stucco` prelude and default features retain their meanings. There
is no new `stucco dev`, `stucco build` or scaffolding system in this plan. Build
instructions and example configuration should make the ordinary `cargo-leptos`
workflow repeatable without wrapping it in another application toolchain.

## Current implementation

The first integration slice introduced one editable order. Step 2 extends that
foundation to a complete Orders workflow. Step 3 adds persistent storage and
identity. Step 4 adds richer components and application workflows. The
implementation includes:

- An unpublished [`stucco-leptos`](../crates/stucco-leptos) crate with `TextField`,
  `PageHeader`, `SubmitButton`, `ErrorSummary`, `StatusMessage`, `Dialog`,
  `Autocomplete`, `RepeatedField` and `FileInput`, using existing Stucco theme
  generation and CSS.
- A separate [`orders-fullstack`](../examples/orders-fullstack) example with SSR,
  hydration, direct Leptos routing and live totals calculated in Rust for two line
  items. It includes listing, customer search, five-order pagination, detail,
  creation, editing and confirmed deletion, plus attachments and CSV exports.
  A separate component gallery demonstrates native fallbacks, localization and RTL.
- A shared domain module that retains submitted text, validates quantities,
  calculates integer cents with checked arithmetic, and carries expected revisions.
- Native POST paths returning 303 after success and rerendering validation or
  revision conflicts with 422 or 409. Typed Leptos server-function mutations call
  the same create, save and delete application services.
- CSRF protection on all mutation paths, checked revisions, nonreused IDs across
  restarts, normal 404 pages for missing records, and explicit unknown
  outcomes for transport failures.

The default store is now embedded redb, chosen to match the user's NoSQL
preference and Stucco's existing storage stack. SQLite is an explicit optional
adapter. Both persist orders, accounts, sessions, attachments and bounded export
records, use a trusted server principal for tenant scope, and retain the same
revision and recovery contracts. New files
are provisioned explicitly; opening the application never inserts sample accounts
or orders. The older Orders database remains separate. The validation records for
Steps 1–3 below remain historical; Step 4 validation is recorded with its milestone.

## Shared behavioral contracts

1. **One owner of each DOM subtree.** Leptos renders and hydrates the same tree.
   Existing `Page` routes can coexist on separate routes, but legacy fragment
   swapping and behavior scripts must not manage a hydrated subtree. Use Leptos
   lifecycle cleanup for listeners, subscriptions and asynchronous work.
2. **Stable initial state.** Keep IDs, themes and initial values consistent between
   SSR and hydration. Preserve pre-hydration input. Serialize only the data needed
   by that view, using safe escaping and lossless fixed-width number handling.
3. **One server service per operation.** Native form handlers and Leptos server
   functions call the same authoritative validation, revision and write logic.
   SSR loaders and HTTP endpoints need equivalent application context. Authorization resolves a trusted principal at HTTP/SSR boundaries and requires
   tenant scope in every service operation;
   server functions can execute directly during SSR.
   [Leptos context requirements](https://docs.rs/leptos_axum/latest/leptos_axum/fn.handle_server_fns_with_context.html).
4. **Explicit public inputs.** Editable drafts carry submitted text and revisions,
   not authoritative prices or permissions. The server supplies trusted prices
   and resolves the principal and tenant. Database models, secrets, pools
   and internal diagnostic details do not become browser DTOs.
5. **Native and hydrated form parity.** Preserve entered values and the submitted
   revision after validation failure or conflict. Keep native 303/422/409 behavior
   explicit. A native POST adapter is a valid application boundary, not a temporary
   requirement to replace with a new generic transport. Framework redirect helpers
   and streaming need deliberate status handling.
   [Leptos redirect behavior](https://docs.rs/leptos_axum/latest/leptos_axum/fn.redirect.html).
6. **Honest asynchronous outcomes.** Discard stale query results. A cancelled or
   disconnected save may already have committed, so retain the draft and reconcile
   before retrying. Automatic retries of non-idempotent operations require an
   application-defined idempotency mechanism.

Use Leptos server functions for typed application calls and ordinary Axum handlers
where appropriate, including native forms, uploads and streams. Do not introduce
a parallel RPC or endpoint framework.

## Step 1 Validate the initial component and editor

Finish the integration slice above using separate native SSR and
`wasm32-unknown-unknown` hydration builds. Record exact dependency, compiler and
`cargo-leptos` versions in the example's setup instructions. Keep existing package
compiler floors and feature combinations working; any broader requirement change
must be explicit.

**Gate:** the editor renders on the server, hydrates without mismatches, updates
totals in the browser and saves through both paths. Invalid input and stale
revisions preserve the submitted draft. Missing/invalid CSRF tokens are rejected
on both write paths. Verify 303/422/409 native responses, duplicate submissions,
uncertain save outcomes and input entered before hydration. Inspect the rendered
component with keyboard interaction and JavaScript disabled. State the process-local
store limitation in the example itself and its documentation.

This initial gate passes for the single-order editor. Broader component coverage,
client-only mounting and performance budgets remain outside this slice.

### Recorded Step 1 validation — 26 October 2026

These counts describe the single-order integration slice before the Step 2
expansion. They do not certify the current expanded workflow.

- `cargo-leptos 0.3.11` built native SSR and WASM hydration separately with
  Leptos 0.8.22, `leptos_axum` 0.8.10, Axum 0.8.9 and wasm-bindgen 0.2.129.
- The new native and WASM dependency graphs passed `cargo check` on Rust 1.88.
  The existing non-storage workspace passed its Rust 1.85 check. The WASM normal
  dependency graph contains no Axum, Tokio, redb, `stucco-core`, `stucco-ui` or
  `stucco-tower`.
- Seventeen new Rust tests passed: six domain tests, seven native HTTP tests and
  four SSR component tests. The existing workspace tests, workspace Clippy,
  target-specific SSR/hydration Clippy, formatting and all-feature documentation
  checks passed.
- Forty-two editor browser checks passed across Chromium, Firefox and WebKit.
  These cover local totals, typed saves, native 303/422/409, retained drafts,
  conflicts, delayed hydration, input restoration, duplicate submissions,
  keyboard focus, CSRF on both endpoints, hostile serialized text and a lost
  response after an actual committed save. Wide/light and narrow/dark layouts
  passed Axe and overflow checks; representative captures were inspected.
- Existing browser regression coverage passed 443 checks, with two existing
  platform-specific skips. This Ubuntu host needed three missing WebKit libraries
  extracted under `/tmp` and supplied through `LD_PRELOAD`; no system packages
  were installed. CI installs Playwright's normal host dependencies.
- Two read-only Claude reviews informed fixes to browser restoration, pending
  focus and ARIA attributes, stale error messages, pending-state allocation and
  preview documentation publication. Browser tests verify the affected paths.

Reproduction commands are in the [example README](../examples/orders-fullstack/README.md)
and the dedicated `fullstack` CI job. Release size/startup measurements and a
production deployment have not been performed.

## Step 2 Grow components through a complete Orders workflow

**Implemented and locally validated.** The example now covers
list, search, pagination, detail, create, edit and delete using Leptos routing,
resources and actions directly. Query state is carried in URLs, and the application
owns loading/empty/error views, per-order editor drafts, navigation focus and
mutation reconciliation. The store seeds twelve records and orders filtered
results by descending ID. Create, edit and delete use the same service logic from
native forms and typed endpoints, including atomic revision checks. The original
Orders example remains separate.

Per-order drafts retain raw input and the submitted revision across internal
navigation. Reload or discard clears only the selected draft, and missing saved
orders retain their cached raw draft for inspection until explicit discard. Query
results are tied to their requested search and page, with retry for read failure.
Pending mutations prevent repeated submission, and unknown create/delete outcomes
remain blocked rather than retrying automatically.

The returned-create recovery path handles a delayed result from the earlier
request: a definite validation error unlocks and focuses the form; definite success
confirms creation while preventing duplicate submission; a lost response stays
unknown. The create fields are read-only while blocked. This behavior has targeted
browser coverage for those cases; it does not guarantee delivery of every request
after navigation or cancellation. Editor reload/discard prevents old values from
being cached again while the replacement loads, and pending deletion cannot be
replaced through its reload link.

Components expose focused props and events while application state stays in the
example. Current coverage is:

| Area | Step 2 implementation | Deferred |
| --- | --- | --- |
| Fields | `TextField` with labels, hints and reactive validation | Rich input types, repeated fields, autocomplete |
| Actions | `SubmitButton` with pending and blocked states | General button/link component family |
| Feedback | `ErrorSummary` and `StatusMessage` | Toasts and overlays |
| Page structure | `PageHeader`, application-owned shell and links | Reusable navigation system |
| Collections | Semantic table, search and URL pagination in the example | Reusable collection/table components |
| Data and identity | Shared process-local store and revision-checked mutations | Persistence, authentication and tenancy |

**Gate:** the full Orders workflow works through direct links, refresh and browser
history, with useful native form behavior. Shared components demonstrate labels,
errors, keyboard interaction and narrow layouts. Test delayed responses, stale
responses, load failure and concurrent edits. Build a component coverage table
showing what is implemented, intentionally static or deferred.

### Recorded Step 2 validation — 26 October 2026

- Forty Rust tests passed: fifteen domain tests, fourteen native HTTP tests and
  eleven SSR component tests. They cover creation, filtering and pagination,
  revisions and concurrent changes, nonreused IDs, native 303/422/409 responses,
  missing-record 404 responses without fabricated data, CSRF on all mutation
  endpoints, escaping and component semantics. Existing backend workspace tests
  also passed.
- `cargo-leptos` built the native SSR server and WASM hydration package separately.
  Native and WASM Clippy checks passed with warnings denied, and both new dependency
  graphs passed their Rust 1.88 compiler-floor checks at the integration checkpoint.
- Targeted Chromium checks passed for retained drafts, unknown creation, late
  validation/success after returning to a create form, independent draft discard,
  controlled input replacement, navigation focus and pending deletion reload.
  These include a real server commit followed by a lost response; unknown creation
  remains blocked and repeated Enter does not issue another mutation.
- Three read-only Claude reviews informed fixes to pending/unknown mutation
  behavior, stale errors, draft recovery, missing-order pages, controlled input
  synchronization and accessibility. The final focused review found no remaining
  correctness defects in late-create settlement or controlled `TextField`
  synchronization; it was a static review, not an additional execution result.

**Final cross-browser result:** all 93 tests passed, comprising 31 scenarios each
in Chromium, Firefox and WebKit. This includes native and hydrated CRUD, browser
history, delayed hydration and search, pending and lost mutation responses, draft
recovery, keyboard focus, Axe audits and narrow layouts. The editor's wide light
and narrow dark captures and the collection/detail/create/delete narrow dark
captures were visually inspected. Formatting and all-feature documentation checks
also passed.
The earlier Step 1 counts remain historical. Step 3 still owns persistence, identity
and tenancy; these checks do not establish production durability or isolation.

## Step 3 Make the reference application durable and authenticated

**Implemented and locally validated.** The default database is
embedded **redb 4.3**, with **SQLite/SQLx 0.8.6** behind the explicit `sqlite` feature
and `--backend sqlite` selection. The full-stack application now requires Rust
1.90, matching the existing redb backend; `stucco-leptos` remains at Rust 1.88.
The existing Orders database is not imported or modified.

Application-owned redb transactions store whole order documents, account records,
monotonic allocation counters and explicit tenant/username indexes. Versioned
schema initialization and upgrades run before serving. Blocking redb work runs
outside Tokio's executor with bounded concurrency. The optional SQL adapter uses
explicit queries, migrations, transactional revision checks and indexed pagination.
Both adapters preserve raw input, checked arithmetic and nonreused IDs after restart.

Authentication uses axum-login, password-auth's Argon2 implementation and
tower-sessions. Sessions persist in the selected database, have an absolute
persisted deadline, and rotate their ID and CSRF protection at login/logout. Every
operation checks the server session; expected client identity fields only reject
stale tabs. Tenant predicates apply to reads, conflict snapshots and mutations.
Explicit provisioning, account disabling and session revocation are operator CLI
actions, with no shipped credentials or development bypass.

The hydrated app scopes drafts and pending delete confirmations by account, tenant
and database generation,
hides them on expiry, and recovers them only for the same identity. Identity changes
invalidate late async responses. Native expired submissions retain one bounded,
short-lived submitted draft in the session; matching reauthentication restores its
original revision for explicit review and resubmission. Stucco's CSRF middleware
now exposes token rotation and a rejection extension so the Leptos adapter can
distinguish its own definite rejection from an uncertain transport failure.

Backups omit session bearer credentials and recovery drafts. Restore validates a
self-contained snapshot in a temporary file before publishing it to a new path. It
rotates database generation so restored IDs cannot match stale tabs, refuses existing
targets and never
silently omits SQLite sidecars. Redb maintenance requires a stopped server because
it owns the database file lock. Setup, public origin, cookie settings, proxy rate
limiting, readiness, shutdown, schema limits and restore instructions are documented
in the [example README](../examples/orders-fullstack/README.md).

**Gate:** data survives restart; fresh and upgrade migrations work; concurrent
writes preserve invariants. Two tenants cannot access each other's records through
any read or write path. Session expiry has a recoverable UI and CSRF rotates with
the session. Exercise backup/restore and document configuration, readiness,
graceful shutdown and the deployment's public origin.

### Step 3 validation

The default SSR suite passes **73 tests** (including 28 native HTTP, 18 redb storage
and 11 component cases). The optional SQLite suite passes **82 tests**, exercising
that backend's HTTP, session, migration and backup/restore paths alongside the redb
contracts. The CSRF middleware suite passes **16 tests**. Workspace tests,
native/WASM Clippy and documentation with warnings denied pass. Rust 1.90 checks
pass for the application on both targets; component checks also pass on Rust 1.88.
All **144 browser cases passed** against the matching server/WASM build: 48 each
in Chromium, Firefox and WebKit. Coverage includes native and hydrated login/logout,
tenant isolation, expiry and recovery, CSRF rotation, stale and delayed responses,
keyboard focus, Axe accessibility audits and narrow layouts. Wide light and narrow
dark recovery screenshots were also visually inspected.

Four read-only Claude reviews informed fixes for absolute session expiry, stale
identities and tokens, failed restore cleanup, allocation counters, database
generation and bounded recovery. The final focused review found no remaining
actionable findings in its scope. Earlier Step 1 and Step 2 counts remain historical.
This implementation has not been deployed or benchmarked as a production service.

## Step 4 Demonstrate richer application components

**Implemented and locally validated.** The implementation adds `Dialog`,
`Autocomplete`, `RepeatedField` and `FileInput`
to `stucco-leptos`. They use native dialog, datalist and file-input behavior, with
Rust enhancements and configurable labels, language and direction. The independent
`/components` gallery demonstrates native fallback, repeated recipients, local
suggestions, file metadata, English/Arabic labels and RTL layout. It depends on
Leptos and the component library; Orders supplies its route and authenticated shell.

Order details now support three attachments of up to 1 MiB each. Bytes and metadata
commit together in the selected database, are included in backups, and are removed
with the order. Native multipart forms and Rust/WASM uploads share an authenticated
endpoint. A new opt-in `stucco-tower/multipart` feature checks bounded native bodies
for CSRF tokens and replays their exact bytes. Interrupted or rejected requests do
not commit partial files; uncertain client outcomes require explicit reconciliation.

CSV exports run through Apalis 0.7.4 with durable job/result records in redb or the
optional SQLite adapter. The memory queue is rebuilt from those records on startup.
Exports are private to the requesting account, limited to 1,000 orders and 1 MiB,
and expire after 24 hours. Eight records per user and 64 records globally bound
stored jobs and retained idempotency history. Cancellation removes the result but
retains its request token and quota slot until expiry. A stable request token
avoids duplicate starts, and explicit retries carry the expected
attempt, with a three-attempt limit. Derived CSV results are excluded from backups
and cleared during restore. Native status pages support refresh; hydrated pages
poll pending jobs in Rust and clean up polling when the page is left. SSE remains
unnecessary for this bounded workflow.

**Gate:** components work across supported browsers, keyboard use, light/dark
themes, narrow layouts and RTL. Rejected/interrupted uploads are cleaned up;
unauthorized downloads fail. Export retries avoid duplicate effects, and progress
recovers after refresh, reconnect or lost events. Each supported enhancement has
an explicit loading/failure path.

### Step 4 local validation

On 26 October 2026, the application passes **84 tests with default redb** and
**104 tests with optional SQLite enabled**. The component crate passes **17 SSR
checks**, and the multipart-enabled CSRF/flash suite passes **22 tests**. Native
and WASM Clippy checks pass with warnings denied, as do workspace tests (one
existing ignored doctest) and all-feature documentation. The application checks
on Rust 1.90 for native and WASM, the component crate on Rust 1.88, and multipart
middleware on Rust 1.85. The WASM dependency graph excludes Apalis, databases,
Axum, session storage and password verification libraries.

A read-only Claude backend review identified three issues that now have fixes
and regression tests: cancelled jobs exhausting global capacity, old running jobs
failing after a long restart, and revoked sessions retaining inaccessible quota.
Claude's session limit prevented its frontend review from completing. Independent
local frontend review found and fixed stale export controls during delayed route
changes.

All **207 browser cases passed** against the matching server/WASM build: **69 each
in Chromium, Firefox and WebKit**. Coverage includes native and hydrated uploads,
authenticated downloads and removals, unknown outcomes and identity changes,
export replay/cancellation and delayed history navigation, and gallery keyboard
behavior, pre-hydration input, file metadata, localization and RTL. Existing CRUD,
authentication, recovery, Axe accessibility and layout checks also pass. Selected
wide light and narrow dark captures of attachments, exports, dialogs and the
English/Arabic RTL gallery were visually inspected across the three browsers.

## Step 5 Publish components and maintain the reference

**Release preparation implemented; publication pending.** `stucco-leptos` now has
an independent **0.1.0-alpha.1** preview version, explicit package contents and
license texts, SSR-only docs.rs configuration, setup instructions and a native
form/CSS example. Its default features remain empty; existing Stucco builders,
preludes and defaults retain their meanings. The stable workspace remains at 0.5.0.

The repository-owned package check extracts a fresh archive outside the workspace
and compiles a consumer for native SSR and WASM hydration separately. SSR consumes
published Stucco core/theme/UI crates, renders all public wrappers and checks their
shared CSS. Resolved dependency sources and feature separation are verified. A
separate exact-registry mode is available after publication; archive validation
does not claim the preview is already installed from crates.io. The full Orders
reference remains checkout-based because its Tower additions are unreleased.

The [deployment guide](../examples/orders-fullstack/DEPLOYING.md) uses ordinary
`cargo-leptos` release output. A packager copies the executable, generated assets,
license texts, build metadata, checksums and guide into a new runtime directory.
It rejects existing destinations, unsafe links and unexpected assets. A separate
rehearsal runs that copy with a disposable redb database and exercises native
login, order creation, attachments, exports, shutdown, backup and restore. Browser
CI runs against the copied release bundle and retains a tarball with executable
permissions preserved.

The reference now enforces a fresh nonce CSP for Leptos scripts and WASM, private
no-store application responses, and revalidation for its unversioned public
assets. Runtime configuration rejects unsupported asset/routing overrides. The
documented deployment is one writer at an origin root, using a maintenance restart
and explicit browser reload. Path prefixes, uninterrupted old tabs and overlapping
redb writers are unsupported. Generated bindings ship with the matching server;
application behavior remains authored in Rust.

[Release instructions](../RELEASING.md) keep preview publication separate from the
existing eight-crate stable workflow and add the preview's SSR API docs to Pages.
Release preparation does not upload a package, create a release tag or deploy a
service; publication follows the separate procedure after review and CI.

**Remaining publication gate:** merge reviewed preparation, publish the exact
preview through the separate release procedure, then run the exact-registry
consumer check and inspect the published README/docs. Publishing the component
library does not turn the reference's backend choices into a public framework
contract. Production deployment and load testing remain operator work.

### Step 5 local validation

The isolated preview consumer passes on Rust 1.88 with registry styling crates
0.5.0, including SSR rendering/CSS and a separate WASM compile. The component suite
passes 17 SSR tests and two doctests. The application passes 88 default-redb tests;
the four new deployment-policy tests also pass with SQLite. Native Clippy,
Rust 1.90 application checks and 11 packaging/checksum boundary tests pass.

The optimized cargo-leptos build and copied-bundle rehearsal pass. The server
executable now uses the same recursion limit as the library, required by its
optimized nested SSR views. Three focused CSP browser checks pass, one per engine.
All **210 release-bundle browser cases pass**, **70 each in Chromium, Firefox and
WebKit**, including native fallback, hydration, mutations, CSP enforcement,
accessibility, attachments, exports and recovery. The release WebKit attachment
and Arabic RTL narrow-dark captures were also visually inspected. The actual
Cargo publish dry run passes on Rust 1.88 with SSR and the packaged registry
dependencies; it aborts before upload as expected.

The Linux x86_64 release was built with rustc 1.101.0-nightly (2026-10-02),
cargo-leptos 0.3.11 and wasm-opt version_123. Its WASM profile uses `opt-level="s"`,
LTO, one codegen unit and aborting panics. The WASM is **1,959,184 bytes**
(733,223 bytes with gzip level 9); generated JS is **28,297 bytes** (7,776 gzip).
These are artifact sizes, not network, startup or hydration measurements. The
existing Stucco loader's 6 KiB budget is separate.

Independent reviews informed packaging no-clobber, asset filtering, checksum
coverage and clean-shutdown checks, and found no remaining actionable issue in
their scope. Claude's previously reported session limit was still in effect;
Step 5 did not receive a new Claude review. No production service or published
registry installation is claimed by these local checks.

## Validation requirements

Preserve existing native tests, semver checks, package verification and browser
coverage. Build SSR and hydration targets separately instead of treating an
all-features build as evidence for both. Check compiler floors and keep server-only
dependencies out of the browser target.

Use shared calculation/validation fixtures on native and WASM targets. Exercise
actual HTTP and browser flows for native form behavior, CSRF, hydration, retained
input, typed failures, revisions and request races. Add authorization, upload and
worker cases as those reference features are implemented. Test lossless public
data serialization, including IDs beyond JavaScript's exact integer range.

Use Chromium, Firefox and WebKit, accessibility checks and visual inspection as
components mature. Record production build settings when measuring WASM/bindings/CSS
size, startup and hydration. The old loader's budget remains separate from the
Leptos application's assets. Validation results must distinguish completed checks
from planned coverage and unavailable tooling.

## References

The repository is the source of truth for implementation status:
[workspace](../Cargo.toml), [Leptos component crate](../crates/stucco-leptos),
[full-stack example](../examples/orders-fullstack),
[existing forms](guide/forms.md), [existing Orders](../examples/orders/README.md),
[browser workflow](../browser/README.md), and [release checks](../RELEASING.md).

Upstream APIs and tooling remain the application foundation:
[Leptos SSR/hydration](https://book.leptos.dev/ssr/22_life_cycle.html),
[server functions](https://book.leptos.dev/server/25_server_functions.html), and
[`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos).
