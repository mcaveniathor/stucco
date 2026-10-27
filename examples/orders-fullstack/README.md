# Orders with Leptos and Stucco

The reference application implements order listing, customer search,
pagination, detail, creation, editing and confirmed deletion. The server renders
each screen, and a Rust/WASM application hydrates it. Leptos owns routing,
rendering, signals, resources, actions and typed server functions. Stucco supplies
shared themes, CSS, application components and CSRF middleware. There is no extra
application framework or build wrapper.

The application uses embedded **redb 4.3 NoSQL storage by default** for orders,
accounts, sessions, attachments and export job state. SQLite is available behind an optional feature. Each account
belongs to one tenant. There are no default accounts or automatically seeded
orders. This application's redb schema is separate from `examples/orders`; pointing
it at that legacy database is not an import. No importer or automatic conversion
between redb and SQLite is provided.

## Run and provision

From the workspace root, with Rust **1.90** or newer:

```sh
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --version 0.3.11 --locked
cargo leptos build -p orders-fullstack --cargo-locked
mkdir -m 700 -p var/orders
export ORDERS_DATABASE="$PWD/var/orders/orders.redb"
```

Set `STUCCO_SECRET_KEY` to 64 random hexadecimal digits from your secret manager
(`openssl rand -hex 32` can generate a new key). Keep the same value across restarts.
**Both debug and release serving require this key.** Never commit it or use the
browser tests' public fixture key in a deployment.

Create an account explicitly. Only `user-add` initializes a missing application
database; its parent directory must already exist. Passwords must contain 12–1024
bytes and are read from stdin, not process arguments. For example, in Bash:

```sh
read -r -s -p 'New account password: ' orders_password
printf '%s\n' "$orders_password" | target/debug/orders-fullstack user-add \
  --tenant my-team --username alice --display-name Alice
unset orders_password
# Optional sample orders, only when this tenant is empty:
target/debug/orders-fullstack seed --username alice
cargo leptos watch -p orders-fullstack --cargo-locked
```

Open **http://127.0.0.1:4183** and sign in. Use the same hostname in every tab.
`cargo-leptos` supplies the `LEPTOS_*` configuration and generates the standard
wasm-bindgen bridge; application behavior is written in Rust, without handwritten
application JavaScript.

To run the already built server:

```sh
LEPTOS_OUTPUT_NAME=orders-fullstack \
LEPTOS_SITE_ROOT=target/orders-fullstack-site \
LEPTOS_SITE_PKG_DIR=pkg \
LEPTOS_SITE_ADDR=127.0.0.1:4183 \
target/debug/orders-fullstack serve
```

`--database PATH` overrides `ORDERS_DATABASE`; the default is `orders.redb` in the
working directory. `serve`, `seed`, `backup`, `sessions-revoke` and `user-disable`
require an existing database. Startup applies known schema upgrades before
listening. New database files use mode 0600 on Unix; protect existing files and
their parent directory as well. The live database contains account hashes,
sessions and possibly short-lived recovery drafts. No development auth bypass is
compiled into the app.

**Stop the redb server before running provisioning or maintenance commands.**
redb holds an exclusive database file lock. This includes adding users, seeding,
revoking sessions, disabling accounts and making CLI backups. Restart the server
when maintenance is complete.

## Configuration and operation

| Setting | Behavior |
| --- | --- |
| `ORDERS_ORIGIN` | Exact public browser origin. Default is `http://` plus `LEPTOS_SITE_ADDR`. Set it explicitly behind a proxy. Forwarded headers do not choose this security boundary. |
| `STUCCO_SECRET_KEY` | Stable 256-bit CSRF signing key, encoded as 64 hexadecimal digits; required for serving. |
| `ORDERS_DATABASE` / `--database` | Local database file for the selected backend. Its extension does not select the engine. |
| `--backend` | `redb` by default; `sqlite` is available only in a build enabling the `sqlite` feature. |
| `ORDERS_SESSION_SECONDS` | Absolute login lifetime, default 28800 (eight hours); valid range 1–604800. Requests do not slide expiry. |
| `/ready` | Returns 200 when the selected storage backend responds, 503 otherwise. Does not expose schema or account information. |

Use HTTPS at the public origin. HTTPS uses Secure, HttpOnly, SameSite=Lax,
host-only `__Host-` session and CSRF cookies. HTTP cookies are for local development.
Configure the trusted proxy for HTTPS redirects, request size and connection
limits, and rate limiting for login, `POST /orders/*` and export submissions.
Allow the attachment route’s documented multipart bound and configure a request-body
read timeout at the proxy. Anonymous native
recovery submissions can create short-lived database rows; CSRF protection does
not rate-limit these writes. Password verification uses Argon2 through
`password-auth`, off the async executor with at most four concurrent hashes;
this concurrency bound does not replace deployment rate limiting.

Login rotates the session ID and CSRF cookie/token, including when switching
accounts. Logout deletes the session and rotates CSRF again. Disabled users,
changed password hashes, tenant reassignment or a changed session epoch invalidate
existing authentication. With the redb server stopped, revoke all devices or
disable an account:

```sh
target/debug/orders-fullstack sessions-revoke --username alice
target/debug/orders-fullstack user-disable --username alice
```

All read and write paths resolve identity on the server. The client sends its
expected user/tenant only to detect stale tabs; these values never select an
authorized tenant. Unknown IDs and IDs belonging to another tenant both appear
missing. A redb write transaction commits the complete order document and its
index together, comparing the stored revision before changing it. ID counters
remain monotonic after deletion and restart. Schema version 2 adds the tenant/order
index; version 3 adds a persistent database generation; version 4 includes
order attachments. Opening older application
databases upgrades them transactionally and repairs allocation counters to at
least the highest stored IDs. Literal Unicode-lowercase substring search scans that tenant's
candidates and is not a full-text index.

redb operations run outside async request tasks with bounded concurrency and a
five-second permit wait. Identity lookups have separate capacity from order work.
Use local durable disk; this reference does not demonstrate network filesystems
or a distributed deployment. SIGINT and SIGTERM stop accepting requests, drain
active requests, stop the export worker and expired-session cleanup, and release
storage handles. Session cleanup runs at startup and every five minutes; export
cleanup runs at startup and every minute. Serve the matching server/WASM/CSS
build together.

## Attachments and exports

An order’s detail page accepts one nonempty file per submission, with at most
**three attachments per order and 1 MiB per file**. The complete multipart body is
limited to **1 MiB + 16 KiB**. The upload handler has a 30-second parsing deadline;
native CSRF verification buffers the bounded body before that handler, so the
deployment must also enforce a request-body read timeout. File names are
limited to 180 bytes, strip path components and reject control characters. The
upload route accepts only the attachment and expected form fields. Native forms
submit multipart data directly; the hydrated view submits the same browser form
through Rust/WASM. Both pass through the existing signed CSRF and authentication
middleware.

The complete file and its metadata commit together in the selected application
database. Incomplete, invalid and rejected parsing creates no saved attachment or
staging file. Removing an attachment or deleting its order removes the stored
bytes transactionally. A backup includes attachments in the same database snapshot
as their orders, so there is no separate upload directory to synchronize. This is a
bounded reference for small files; it does not implement object storage, malware
scanning or inline content previews. Downloads require a current authorized team
member and use `application/octet-stream`, attachment disposition, `nosniff` and
`private, no-store` headers.

Browser file selections cannot be restored after navigation, a native response or
sign-in recovery; select the file again before resubmitting. A definite rejection
in the hydrated view preserves its current selection. After an unknown
upload/removal outcome, the hydrated view blocks another mutation until **Refresh
attachments** checks the saved list. Native pages offer **Reload saved attachments**.
A lost response may mean the upload committed; inspect that list before choosing
to upload again. Upload mutations are never retried automatically.

Open **`/exports`** to request a CSV of the current tenant’s orders. Each export
captures a consistent database snapshot when its worker attempt starts and allows
at most **1,000 orders and 1 MiB of output**. Completed snapshots remain unchanged
when orders are later edited or deleted. Only the requesting account can inspect,
retry, cancel or download its export, including when another account belongs to
the same team. Downloads check the database generation, enabled account, session
epoch, job state, expiry and bytes in one storage snapshot. Session revocation,
account disablement and restoring a database invalidate access to old exports.

**Apalis 0.7.4** runs one bounded worker using its in-memory queue. Job identity,
state, attempt number and completed bytes persist in redb or optional SQLite;
the queue contains only dispatch identifiers. Startup reconstructs queued or
interrupted running jobs from those records. This is one executor per application
process and database, not a distributed worker deployment. A worker attempt has a
30-second deadline. Stuck running records become failed during cleanup rather
than remaining pending indefinitely.

Jobs expire **24 hours after creation**, and cleanup deletes their records and
CSV bytes. Limits are **eight retained jobs per account and 64 across the database**,
including cancelled replay records. **Cancel and remove export** deletes output
while preserving the request’s cancelled identity and quota slot until expiry,
so repeating an old request cannot create another export. Cancellation also
prevents late worker completion from publishing a result. Starting a new export
after session revocation removes that account’s inaccessible old-session records
and output before applying these limits.

The form’s random request token makes a repeated start idempotent. A lost response
can be reconciled through **Check request status**, the recent exports list or a
repeat submission of that same form. Failure never automatically starts another
attempt: **Retry export** explicitly authorizes a fresh snapshot, up to **three
attempts total**. The expected attempt number makes repeating a retry after a lost
response idempotent. Cancelled exports cannot be retried; request a new export
when needed. The native status page has a refresh link, and Rust/WASM polls while
work is pending, so refreshes and connection interruptions recover from durable
state without depending on a stream of past events.

CSV columns are order ID, revision, customer, the two quantities and total cents.
Customer cells are quoted and escaped; a leading apostrophe neutralizes names
that spreadsheet applications might interpret as formulas. CSV downloads are
private attachments with `nosniff` and `no-store` headers. Exports are derived,
short-lived data: **backup omits their metadata and bytes, and restore clears any
included export tables**. Request a new export after restoring.

## Optional SQLite

The normal SSR build does not include SQLx or the SQLite session adapter. Enable
them for the server binary with cargo-leptos:

```sh
cargo leptos build -p orders-fullstack --bin-features sqlite --cargo-locked
export ORDERS_DATABASE="$PWD/var/orders/orders.sqlite3"
```

Use `--backend sqlite` before the subcommand on every provision, seed, serve,
backup and restore command. The feature affects the server build; the hydration
build remains free of database dependencies. Provision this database separately
with `user-add` before serving it. The same account, tenant, draft and revision
contracts apply to both backends.

SQLite uses a four-connection pool, a five-second busy/acquisition timeout,
foreign keys, WAL journaling and FULL synchronous writes. Its transactions commit
the order and its lines together. Protect WAL/SHM files alongside the database.
Unlike redb CLI maintenance, SQLite backup can run while the server is serving.
The backend defaults to `orders.sqlite3` when neither `--database` nor
`ORDERS_DATABASE` is supplied.

## Backup and restore

Stop the default redb server, then create a fresh snapshot and restore it into a
new path:

```sh
target/debug/orders-fullstack backup --destination var/orders/backup.redb
target/debug/orders-fullstack --database var/orders/restored.redb \
  restore --source var/orders/backup.redb
```

redb backup copies the application tables from one read transaction into a fresh
file, including attachment metadata and bytes. Sessions, bearer IDs, raw recovery
drafts and derived exports never enter that snapshot. The optional SQLite adapter
uses `VACUUM INTO`, removes session and export rows from the copy, and vacuums it
again to remove their bytes. For an online SQLite backup:

```sh
target/debug/orders-fullstack --backend sqlite \
  backup --destination var/orders/backup.sqlite3
target/debug/orders-fullstack --backend sqlite \
  --database var/orders/restored.sqlite3 restore --source var/orders/backup.sqlite3
```

Backup refuses any existing destination. Restore requires a self-contained
snapshot and a fresh destination; source or destination `-wal`, `-shm` or
`-journal` sidecars are rejected. It preserves the source, validates the selected
backend's schema and data in a temporary file beside the destination, clears
restored sessions and exports, and changes database generation before publishing the validated
file without overwriting an existing destination. Failures before publication
clean up the temporary file. Completed backup and restore files are synced,
including their parent directory on Unix. redb restore accepts schema version 2,
3 and 4 snapshots; opening a version 1 application database for upgrade is a separate
path. SQLite validation checks integrity, foreign keys and known
migration checksums, then applies pending known migrations to the destination.

Stop the service before switching it to the restored file. Keep the original and
snapshot until the restored application has been checked; verify record counts
and representative revisions, then sign in again. Backups still contain account
hashes and business data, so protect them. This workflow does not provide automatic
retention or replication.

## Dependencies

Pins: Leptos **0.8.22**, router **0.8.17**, meta **0.8.7**, Axum integration
**0.8.10**, Axum **0.8.9**, wasm-bindgen **0.2.129**, cargo-leptos **0.3.11**,
redb **4.3.0**, axum-login **0.17.0**, tower-sessions **0.14.0**,
password-auth **1.0.0**, time **0.3.44**, clap **4.6.7**, Apalis **0.7.4**. The optional SQLite build
adds SQLx **0.8.6** and the SQLite session store **0.15.0**.

The full-stack application requires Rust **1.90**, matching redb 4.3. The
`stucco-leptos` component crate retains Rust **1.88**; other package compiler floors
are unchanged. SSR-only features keep database, credentials and server
dependencies out of the browser build. `Cargo.lock` records exact transitive
versions.

## What to try

- Open `/orders`, search by customer name, and move between five-order pages.
  Search and page state live in the URL, with newest IDs shown first.
- Follow a detail link, create an order, edit it and use its delete confirmation.
  Deleted IDs are not reused after restart.
- Change either quantity. Hydrated totals update locally in Rust without a request.
- Save, then reload. The service validates and commits the order with a new revision.
- Enter `2x` as a quantity. The rejected save preserves the text and wires its error
  to the control using `aria-invalid` and `aria-describedby`.
- Open two tabs, save in one, then save the older draft. The older tab retains its
  values and revision, and offers an explicit link to reload the saved order.
- Open a delete confirmation, edit the order in another tab, then confirm deletion.
  The stale confirmation must be refreshed before it can delete that revision.
- Leave a dirty editor through an internal link and return. Its raw input and
  original revision remain available. Reloading or discarding one draft leaves
  other orders' drafts intact. If another tab deletes the order, the returned
  editor retains its raw draft for inspection until explicit discard.
- Add an order attachment, download it, and remove it. Refresh the saved list
  before retrying an upload whose outcome is unknown.
- Open `/exports`, request a CSV and refresh its status. Download or cancel it;
  another account cannot access the same export.
- Open `/components` to try the reusable controls with configurable labels,
  direction and formatting independently of order data.
- Disable JavaScript. The same form submits natively: success redirects with 303,
  validation errors render with 422, and stale revisions render with 409. Missing
  orders return 404 without an edit or delete form.

Native POST adapters and typed Leptos mutations call the same `OrderStore::create`,
`save_by_id` and `delete` services. Prices and tenant scope are server-owned; clients submit text, the expected
revision and an assertion of their current identity. All HTTP mutation paths pass through Stucco's signed
CSRF layer. The typed server-function transport returns business failures as data;
it does not share the native HTML status contract. Network failure is reported as
an unknown mutation outcome, with no automatic retry. In-app editor drafts are
retained in browser memory and partitioned by account, tenant and database generation. Restoring a backup
changes that generation, so stale tabs cannot target reused record IDs. Expiry hides
protected views but retains a quarantined draft for the same account. Signing in
as another account or explicitly signing out clears that state. Late responses
cannot populate a new identity's views. These drafts are not durable offline storage.

A native POST rejected for expired authentication saves one submitted draft or
delete revision in the database session, bounded to 1 KiB and a 15-minute recovery
window. Follow the sign-in link and authenticate as the original account to review
it; saving remains an explicit action and retains the original revision. Another
account cannot recover it. If the saved order has since been deleted, the submitted
text is shown for copying, without a mutation form. A newer rejected native
submission replaces that browser session's previous recovery. Native requests
rejected earlier by CSRF protection are not stored for recovery.

An unknown create outcome keeps its fields read-only and its submit button blocked
until reconciliation or explicit discard. If a delayed creation response reaches a
returned create form, a validation failure unlocks it with focused errors; a known
success confirms creation and prevents duplicate submission. A lost response stays
unknown. Delete also blocks retries while its outcome is unknown. Reload is guarded
while deletion is pending, and a discarded editor cannot re-cache its old values
while its replacement loads.

Collection loading follows the requested query and page, and a delayed earlier
search cannot replace newer results. Failed reads offer a retry. These recovery
paths supplement the native forms; they do not add offline persistence or automatic
retry of mutations.

## What Stucco adds

The example uses Leptos routing, `Resource`, `Action`, `RwSignal`, `#[server]` and
Axum directly.
A plain Leptos field would require the application to keep its label, stable IDs,
hint, reactive error, accessible description and input synchronization consistent:

```rust,ignore
view! {
    <TextField id="quantity" name="quantity" label="Quantity"
        value=quantity
        on_input=Callback::new(move |value| quantity.set(value))
        inputmode="numeric" hint="Whole quantities, 1–999"
        error=quantity_error/>
}
```

`TextField` packages that behavior and the established Stucco styling while leaving
state and domain rules with the application. `PageHeader`, `SubmitButton`,
`ErrorSummary` and `StatusMessage` supply the other reusable components used by
these screens. `Dialog`, `Autocomplete`, `RepeatedField` and `FileInput`
exercise richer interaction in the component gallery and attachment workflow.
Tables, navigation links and pagination remain ordinary semantic Leptos views in
the application.

`stucco_leptos::stylesheet` generates CSS from the existing theme and component
assets. No legacy behavior scripts are mounted inside the Leptos tree. The component
crate's independent `0.1.0-alpha.1` preview is prepared and remains unpublished.
Its package can consume published Stucco styling crates; this application still
uses checkout code for the unreleased Tower multipart/session additions.

For release builds, runtime packaging, CSP/cache policy, root-path hosting,
maintenance upgrades and backup/restore rehearsals, follow
[the deployment guide](DEPLOYING.md). The reference uses one database writer and
does not promise compatibility between an old browser build and a new server.

## Validate

```sh
cargo test -p stucco-leptos -p orders-fullstack --features ssr --locked
cargo test -p orders-fullstack --features sqlite --locked
cargo clippy -p stucco-leptos -p orders-fullstack --all-targets --features ssr --locked -- -D warnings
cargo clippy -p stucco-leptos -p orders-fullstack --lib --target wasm32-unknown-unknown --features hydrate --locked -- -D warnings
cargo leptos build -p orders-fullstack --cargo-locked
cd browser
npm ci
npx playwright install chromium firefox webkit
npx playwright test --config playwright.fullstack.config.ts
```

SSR and hydration are separate builds. `--all-features` is useful for documentation
checks, but does not replace target-specific compilation or actual browser tests.
Run the cargo-leptos build after ordinary Cargo tests and before Playwright, so
the server binary and hydration assets use the same build configuration. The
separate Playwright config provisions an isolated database, starts the built server
and tests native and hydrated flows. Existing browser suites remain independent of cargo-leptos.

At the Step 2 integration checkpoint, **40 Rust tests passed**: 15 domain, 14 native
HTTP and 11 SSR component tests. Existing backend workspace tests, separate native
and WASM builds, target-specific Clippy and Rust 1.88 checks also passed.
All **93 browser tests passed** across Chromium, Firefox and WebKit, including
native forms, hydration, recovery, keyboard navigation and accessibility audits.
Three read-only Claude reviews informed fixes, with no remaining findings
in the final focused review of late creation and controlled input synchronization.

Step 3's default SSR suite passes **73 tests**, including 28 native HTTP cases,
18 redb storage cases and 11 component cases. The optional SQLite suite passes
**82 tests**, including the same HTTP contract against SQLite, 18 SQLite storage
cases, and restore failure cleanup. The CSRF middleware suite passes **16 tests**.
Workspace tests, native/WASM Clippy, documentation with warnings denied, Rust 1.90
application checks and Rust 1.88 component checks pass. All **144 browser cases
passed** against the matching server/WASM build: 48 each in Chromium, Firefox and
WebKit, including account isolation, native and hydrated recovery, keyboard focus,
Axe accessibility and layout checks. Recovery screenshots were visually inspected
in wide light and narrow dark layouts.

Four read-only Claude reviews informed the persistence, session and restore fixes.
The final focused review found no remaining actionable findings in its scope.
These are local integration checks; the reference has not been production load-tested.

At the Step 4 checkpoint, **84 application tests with default redb**, **104 with
optional SQLite**, **17 SSR component tests**, and **22 multipart-enabled CSRF/flash
tests** pass. Workspace tests, native/WASM Clippy, documentation and the supported
compiler checks pass.
The reference now includes the component gallery, attachments and durable exports.
A Claude backend review led to three fixes with regression coverage. Its frontend
review hit the account session limit; independent local review covered the frontend
and fixed stale export controls during delayed history navigation.
All **207 browser cases pass**: **69 each in Chromium, Firefox and WebKit**,
including the new native and hydrated attachment/export workflows, gallery
keyboard behavior, localization, RTL, accessibility and narrow layouts. Selected
wide light and narrow dark screenshots were visually inspected across the three
browsers.

At the Step 5 checkpoint, the copied **optimized release bundle passes all 210
browser cases**, 70 per engine, including enforced CSP and native/hydrated writes.
Its CLI rehearsal covers creation/reads, uploads, exports, clean shutdown and
backup/restore with session invalidation. The application passes 88 default-redb
tests; the four deployment-policy tests also pass with SQLite. The component
package passes isolated SSR/WASM consumption against registry styling crates and
an actual publish dry run. Eleven packaging/checksum tests pass. Preview
publication and post-publication registry verification remain pending.

See the [roadmap](../../docs/roadmap-fullstack.md) for validation details and the
remaining publication gate.
