# stucco-tower

Tower integration for stucco: asset services, page and fragment responses,
request negotiation, request context, and backend collection source traits.

The default `axum` and `tower-http` features add Axum asset routing and the
standard middleware stack. The default `csrf` feature adds explicit CSRF and
flash layers and their signing primitives; it works without Axum. Disable
defaults for the base Tower/HTTP interfaces.

Most applications use it through the `stucco` crate's `axum` feature, which
puts the setup in `stucco::prelude` and the rest under `stucco::server`.
Directly, with stucco-core, stucco-theme and axum:

```rust
use axum::{Router, routing::get};
use stucco_core::el;
use stucco_theme::Preset;
use stucco_tower::{Document, PageCx, StuccoRouter};

async fn index(page: PageCx) -> Document {
    page.title("Hello").body(el::main().id("main").child(el::h1().text("Hello")))
}

// Assets under /_stucco/, the standard layers, and the bundle for `PageCx`.
let app: Router = Router::new().route("/", get(index)).stucco(Preset::Slate);
```

For finer control, `assets_router` and `with_standard_layers` are the
pieces `stucco` puts together.

`CollectionSource` loads owned pages through a Send future.
`RequestContext` preserves typed request extensions, allowing applications to
provide their own authentication and authorization context. The default stack
does not implement an application's authorization policy.

For browser writes, install `CsrfLayer::new(key, "https://example.com")?`
on your routes before calling `.stucco(...)`. Read `CsrfToken` through
`RequestContext::get` and pass `as_str()` to `Form::csrf`. The layer verifies
unsafe methods, checks source headers against your public origin, and replays
bounded form bodies for `Submission`. It is not installed automatically.

Native multipart forms can opt in with the `multipart` crate feature and
`CsrfLayer::multipart(true)`. The layer requires exactly one `_csrf` text field,
buffers within `body_limit`, parses the complete multipart body, and replays its
bytes unchanged to the application's upload handler. Missing, duplicate or
file-valued tokens fail before dispatch, as do malformed trailing fields. This
mode is disabled by default. Configure its body limit together with the server's
upload limits; the token field itself is limited to 1024 bytes. Applications still
own upload authorization, file validation, storage and cleanup. Requests carrying
a valid `stucco-csrf` header pass their body through without buffering, so they
remain subject to the application's independent streaming/body limits.

`FlashLayer::new(key)` transports `SeeOther::new("/done").flash(Flash::new(
FlashLevel::Success, "Saved"))` through a signed five-minute cookie. Read the
next request's `Flash` from its context and render it with `FlashMessages`.
Both layers use Secure, HttpOnly, host-only cookies and private, non-cacheable
responses. Install them before mounting assets so assets do not consume messages.

`SecretKey::from_env()` reads 64 hex digits from `STUCCO_SECRET_KEY`; missing
keys are errors in release builds, or generate an ephemeral development key
with a tracing warning in debug builds. Share one stable key across replicas.
Use `.secure(false)` on both layers only for local HTTP development. Messages
are signed, not encrypted, limited to 1024 bytes, and are not authorization.
See [the forms guide](https://github.com/mcaveniathor/stucco/blob/main/docs/guide/forms.md)
for setup, layer ordering, limits and session lifecycle notes.

Minimum Rust: **1.85**. The API is experimental.
See [the hello example](https://github.com/mcaveniathor/stucco/tree/main/examples/hello)
for a runnable application.

Licensed under MIT or Apache-2.0.
