# stucco

Server-rendered, themeable, accessible UI components for Rust. Plain HTML and CSS,
enhanced by small JavaScript behaviours only where a component needs them.

Status: early development. See `docs/superpowers/specs/` for the design.

## Quick start

```rust
use std::sync::Arc;
use stucco::{Bundle, Page, el, theme::Preset};
use stucco_tower::{LayerConfig, PageResponse, assets_router, with_standard_layers};

let bundle = Arc::new(Bundle::new(Preset::Slate));
let page = Page::new(&bundle, "Hello").body(el::main().child(el::h1().text("Hello"))).render();
let routes = axum::Router::new().route("/", axum::routing::get(move || async move { PageResponse::new(page) }));
let app = with_standard_layers(routes.merge(assets_router(bundle)), &LayerConfig::default());
```

A complete app is in `examples/hello` (`cargo run -p hello`).

The persistent collection example is in [examples/orders](examples/orders/README.md)
(`cargo run -p orders`, then http://localhost:4181/orders). It composes typed
columns, native GET filters, semantic tables and an application shell over
`CollectionSource` and the optional `stucco-redb` adapter. Enable the facade's
`collections` feature to use these components; components do not depend on
storage or Tokio.

Core, components and Tower support Rust 1.85. The latest redb adapter (4.3)
and orders example require Rust 1.90. Numbered mode counts through a full scan;
default cursor mode leaves the total unknown.

## Icons

The `icons` feature vendors the [Lucide](https://lucide.dev) icon set
(ISC licence, `crates/stucco-ui/LICENSE-LUCIDE`). Regenerate it with
`cd tools && npm install && npm run icons`.

## License

Licensed under either of Apache License, Version 2.0 or MIT license, at your option.
