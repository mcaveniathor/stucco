# stucco-tower

Tower integration for stucco: asset services, page and fragment responses,
request negotiation, request context, and backend collection source traits.

The default `axum` and `tower-http` features add Axum asset routing and the
standard middleware stack. Disable defaults for the base Tower/HTTP interfaces.

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

Minimum Rust: **1.85**. The 0.2 API is experimental.
See [the hello example](https://github.com/mcaveniathor/stucco/tree/main/examples/hello)
for a runnable application.

Licensed under MIT or Apache-2.0.
