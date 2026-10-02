# Getting started

Install stucco, render your first page, and serve it with Axum.

## Install

stucco is split into crates so you only compile what you use. Most applications add the facade with its `axum` feature, which brings in the Tower integration:

```sh
cargo add stucco --features axum
cargo add axum
cargo add tokio --features macros,rt-multi-thread,net
```

| Crate | What it does | Minimum Rust |
| --- | --- | --- |
| `stucco` | The facade: components, pages, themes | 1.85 |
| `stucco-tower` | Axum responses, asset serving, form extraction, middleware (also `stucco::server` with the `axum` feature) | 1.85 |
| `stucco-redb` | Embedded storage with typed tables and indexes | 1.90 |
| `stucco-cli` | The `stucco` command: generate and export themes | 1.85 |
| `stucco-macros` | `#[derive(Columns)]`, used through `stucco`'s `derive` feature | 1.85 |

## Render a page

Components implement `Render`. A `Bundle` holds your theme and the hashed CSS and scripts components need, and a `Page` turns a body into a complete document:

```rust
use stucco::prelude::*;

let bundle = Bundle::new(Preset::Slate);
let html = Page::new(&bundle, "Hello, stucco")
    .main(
        Container::new().child(
            Stack::new()
                .child(Heading::new(1, "Hello, stucco"))
                .child(Text::new("A page rendered entirely in Rust."))
                .child(Button::new("Continue")),
        ),
    )
    .render();
```

`stucco::prelude` brings in the page and theme types, the everyday components and their shared enums such as `Space` and `Variant`; less common components are in their modules, such as `stucco::forms::CsrfToken`. `main` puts your content in the page's `<main id="main">` landmark, after a skip link to it, so keyboard users can jump past the header. For an application layout, `app` takes an `AppShell`, which brings its own landmark and skip link.

The page links its stylesheet under `/_stucco/`. Serving with Axum, below, takes care of those files; otherwise write them out yourself with `Bundle::paths` and `Bundle::get`.

## Serve with Axum

One call sets up a router: `stucco` serves the bundle's assets with long-lived cache headers, wraps your routes in a standard middleware stack (request ids, `nosniff`, tracing, a body size limit and a timeout) and makes the theme available to handlers.

```rust
use axum::{Router, routing::get};
use stucco::prelude::*;

async fn index(page: PageCx) -> Document {
    page.title("Hello").main(Heading::new(1, "Hello"))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app = Router::new().route("/", get(index)).stucco(Preset::Slate);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await
}
```

- Call `stucco` last, after your routes and `with_state`, so the setup covers every route. `stucco_with` takes a `LayerConfig` to change the body limit and timeout.
- `PageCx` is an extractor. `title` starts a `Document`, which handlers return like any response. Set its `status` for errors, such as 422 for a form that failed validation.
- A document renders its content as soon as you set it, so the content can borrow request data such as a query or a page of rows.
- `PageCx::respond` answers fragment requests for one region of a page; see [Progressive enhancement](enhancement.html).
- Containers take their children in one call: `Stack::of((heading, text, button))` is the same as three `child` calls.

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
