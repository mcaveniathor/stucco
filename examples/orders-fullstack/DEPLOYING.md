# Deploying the Orders reference

This is a maintained example for adapting an application, with one server process
and a local embedded redb database by default. The component library is reusable;
the example's account model, quotas, CLI, schema and HTTP endpoints are application
code. Run it on a host compatible with the native binary's OS, architecture and C
runtime. The local rehearsal below does not establish production load capacity or
validate your TLS termination, proxy, disks or recovery objectives.

## Build and package

Use Rust 1.90 or newer, the WASM target and cargo-leptos 0.3.11. Run ordinary Cargo
tests first: cargo-leptos supplies compile-time hydration names, so an ordinary
Cargo build afterwards can replace the server with one that no longer matches
the browser artifacts. From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --version 0.3.11 --locked
cargo leptos build -p orders-fullstack --release --cargo-locked
python3 tools/package-orders.py --output target/orders-release
cd target/orders-release
sha256sum --check SHA256SUMS
```

The packager copies the native executable, the matching JS/WASM/CSS from
`target/orders-fullstack-site/pkg`, this runbook, build metadata and checksums. It
refuses an existing output directory. It does not copy databases, environment
files, account credentials or repository source. `build.json` records whether the
source checkout was dirty; an uncommitted build is suitable for local review, not
an identified release. Checksums detect changed files; successful hydration in the
rehearsal establishes that the executable and assets work together.

The default bundle uses redb. To build the optional SQLite adapter, add
`--bin-features ssr,sqlite` to the same cargo-leptos build and use `--backend sqlite`
for every CLI operation. The current deployment rehearsal targets default redb.
The app does not load Cargo.toml at runtime.

## Configuration and storage

Run from the bundle directory with these nonsecret environment settings:

```sh
export LEPTOS_ENV=PROD
export LEPTOS_SITE_ADDR=127.0.0.1:4183
export LEPTOS_SITE_ROOT=site
export LEPTOS_SITE_PKG_DIR=pkg
export LEPTOS_OUTPUT_NAME=orders-fullstack
export LEPTOS_HASH_FILES=false
export ORDERS_ORIGIN=https://orders.example.com
export ORDERS_DATABASE=/var/lib/orders/orders.redb
```

Provide `STUCCO_SECRET_KEY` as a persistent secret containing 64 random hexadecimal
digits through your service manager or secret store. Never put it in the bundle,
command-line arguments or version control. Provision an account with `user-add`
while the server is stopped, piping its password from a secure prompt/secret store
on stdin. Then start `./orders-fullstack serve` as a dedicated unprivileged user.
See the example README in the repository for the full CLI/configuration contract.

Keep the database and backups outside the replaceable bundle directory. Restrict
their filesystem permissions to the service operator; backups contain account
password hashes and attachments. redb owns an exclusive file lock. Only one
application process may open the database, including administrative commands.

Expose the app through a TLS reverse proxy that preserves the public host and
request paths. The bind address can remain private; `ORDERS_ORIGIN` is the exact
public HTTPS origin and controls Secure cookies and CSRF checks. Configure request
body size/read-time limits and rate limits, including login, native recovery,
attachments and export submissions. The attachment bound is 1 MiB + 16 KiB. Its
handler parsing deadline does not cover the earlier CSRF body buffering.

## Paths, caching and content security

This reference serves at the origin root only: `/orders`, `/exports`, `/components`,
`/login`, `/api`, `/pkg` and `/theme.css`. A URL prefix such as `/my-app` is unsupported;
stripping it only at the proxy breaks forms, redirects, cookies and hydration URLs.
Use a dedicated hostname. The server rejects custom output/package names, asset
hashing and server-function routing options that disagree with these fixed routes.

HTML, authentication, API, downloads and error responses use
`Cache-Control: private, no-store`. Do not cache them at a proxy or CDN. The current
unversioned JS/WASM and theme CSS use `Cache-Control: no-cache`, requiring
revalidation; never add `immutable` or a long freshness lifetime to those URLs.
Serve `/pkg` and `/theme.css` from the same running release. Missing assets must
remain 404 responses, not an SPA HTML fallback.

The server sends a Content Security Policy with a fresh nonce for Leptos bootstrap
and resource scripts, same-origin scripts/styles/connections, and
`wasm-unsafe-eval` for WebAssembly compilation. It does not allow arbitrary inline
scripts or JavaScript `unsafe-eval`. Framing, objects and base URL overrides are
disabled. Preserve this CSP at the proxy; do not append a conflicting second
policy or cache nonce-bearing HTML. Native forms still work when WASM is blocked.
The generated wasm-bindgen bridge is part of the cargo-leptos output; application
behavior remains authored in Rust.

## Deployment, old tabs and rollback

Use a maintenance restart for this single-writer reference:

1. Build and rehearse a new bundle in a separate directory. Check its checksums.
2. Stop new traffic at the proxy, then send SIGTERM to the running process and
   wait for it to exit. In-flight HTTP requests drain; the export worker has a
   bounded shutdown window and pending work is reconstructed at startup.
3. With the server stopped, run its `backup --destination <new-file>` command.
4. Switch the service's working directory and executable to the new bundle,
   retaining the database location and secret. Start it and wait for `/ready` to
   return 200 before reopening traffic. Readiness checks storage access; the bundle
   rehearsal also checks assets, login, hydration and persisted data.
5. Ask users to reload existing tabs before resuming edits. Capture unsaved text
   before reload. There is no cross-release browser/server API compatibility
   guarantee: server-function routes and serialized inputs can change. Stable
   native URLs are not a promise that an old draft is valid against a new schema.

Do not run old and new servers concurrently against one redb file, mix assets
from two builds or automatically replay failed writes. The app's unknown-outcome
UI requires checking saved state before retrying. Cross-release rolling upgrades
and multi-host storage require a different deployment design. Because asset URLs
are unversioned, a page load straddling the cutover can also receive mismatched
HTML and WASM; reload after maintenance. Supporting uninterrupted old tabs would
require retained versioned assets and an explicit client/server compatibility
protocol.

For rollback, stop the new server first. Use a binary that understands the current
schema, or restore the pre-upgrade backup into a **new** database path using the
matching maintenance binary. Never downgrade an upgraded database in place.
Restore validates the snapshot, rotates database generation and invalidates old
sessions/drafts. Attachments survive; derived export jobs/results do not. Verify
the restored application before reopening traffic and explicitly account for any
writes made since the backup. The original database remains available for operator
reconciliation.

## Rehearse the bundle locally

From the repository root, after packaging:

```sh
python3 tools/smoke-orders-bundle.py target/orders-release
cd browser
STUCCO_E2E_BUNDLE=../target/orders-release npx playwright test --config playwright.fullstack.config.ts
```

The CLI rehearsal creates its own temporary account/database, serves from the
bundle directory, checks native login and asset delivery, stops the process,
backs up and restores into a new path, and verifies restored state and invalidated
sessions. It removes only its own temporary files. The browser harness separately
provisions disposable accounts and runs the real native/hydrated workflows against
the copied executable and assets. Neither check uses a production database.

Upstream references: [cargo-leptos 0.3.11 configuration](https://docs.rs/crate/cargo-leptos/0.3.11/source/README.md),
[Leptos SSR deployment](https://book.leptos.dev/deployment/ssr.html), and
[Leptos nonce support](https://docs.rs/leptos/0.8.22/leptos/nonce/index.html).
