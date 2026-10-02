# Getting started

Install stucco, render your first page, and serve it with Axum.

## Install

stucco is split into crates so you only compile what you use. Most applications add the facade and, to serve pages, the Tower integration:

```sh
cargo add stucco
cargo add stucco-tower axum
cargo add tokio --features macros,rt-multi-thread,net
```

| Crate | What it does | Minimum Rust |
| --- | --- | --- |
| `stucco` | The facade: components, pages, themes | 1.85 |
| `stucco-tower` | Axum responses, asset serving, form extraction, middleware | 1.85 |
| `stucco-redb` | Embedded storage with typed tables and indexes | 1.90 |
| `stucco-cli` | The `stucco` command: generate and export themes | 1.85 |

## Render a page

Components implement `Render`. A `Bundle` holds your theme and the hashed CSS and scripts components need, and a `Page` turns a body into a complete document:

```rust
use stucco::prelude::*;
use stucco::actions::Button;
use stucco::layout::{Container, Stack};
use stucco::typography::{Heading, Text};

let bundle = Bundle::new(Preset::Slate);
let content = Container::new().child(
    Stack::new()
        .child(Heading::new(1, "Hello, stucco"))
        .child(Text::new("A page rendered entirely in Rust."))
        .child(Button::new("Continue")),
);
let html = Page::new(&bundle, "Hello, stucco")
    .body(el::main().id("main").child(content))
    .render();
```

The page links its stylesheet under `/_stucco/`. Serve those files with `stucco_tower::assets_router`, or write them out yourself with `Bundle::paths` and `Bundle::get`.

## Serve with Axum

`stucco-tower` turns rendered HTML into responses, serves the bundle's assets with long-lived cache headers, and wraps your router in a standard middleware stack: request ids, `nosniff`, tracing, a body size limit and a timeout.

```rust
use std::sync::Arc;
use stucco::{Bundle, Page, el, theme::Preset};
use stucco_tower::{LayerConfig, PageResponse, assets_router, with_standard_layers};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let html = Page::new(&bundle, "Hello")
        .body(el::main().id("main").child(el::h1().text("Hello")))
        .render();
    let routes = axum::Router::new().route(
        "/",
        axum::routing::get(move || {
            let html = html.clone();
            async move { PageResponse::new(html) }
        }),
    );
    let app = with_standard_layers(routes.merge(assets_router(bundle)), &LayerConfig::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await
}
```

## Run the examples

The repository includes three runnable examples:

```sh
git clone https://github.com/mcaveniathor/stucco.git
cd stucco
cargo run -p hello     # http://localhost:4180: a page with a fragment-enhanced form
cargo run -p orders    # http://localhost:4181/orders: a persistent, filterable table
cargo run -p gallery -- target/gallery   # every component, as static pages
```

## Where to go next

- [Components](components.html) explains how components are built and composed.
- [Forms](forms.html) covers submission, validation and error display.
- [Collections](collections.html) builds searchable, filterable tables.
- [Theming](theming.html) covers presets, seeded themes and the playground.
