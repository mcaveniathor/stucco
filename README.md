# stucco

Server-rendered UI components for Rust, built from semantic HTML and themeable
CSS. Compose layouts, forms, tables, and application shells with ordinary Rust
builders. Search, filters, sorting, and pagination can work through native GET
requests without JavaScript.

Stucco **0.1.0** is available on [crates.io](https://crates.io/crates/stucco).
The [documentation site](https://mcaveniathor.github.io/stucco/) has the guide,
a theme playground with Rust, CSS and JSON export, the component gallery and
the API reference.
The public API is experimental. Install the crates below, or run the examples
from this repository.

## What is included

- Layout primitives, typography, buttons, form controls, feedback, and icons.
- Tables, cards, panels, page headers, footers, and application shells.
- Typed collection columns and capability-aware GET search/filter controls.
- OKLCH themes with semantic color tokens, light/dark schemes, and contrast checks.
- Escaped text, checked URLs, generated IDs, and explicit accessible labels.
- Optional Tower/Axum integration and embedded redb storage.

Components render HTML through the `Render` trait. A `Bundle` supplies themed,
content-hashed assets, and a `Page` produces the document. Applications provide
data and backend capabilities; components remain independent of HTTP, Tokio,
and storage.

## Render a page

Add:

```toml
[dependencies]
stucco = "0.1"
```

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
assert!(html.contains("Hello, stucco"));
```

The page links to assets under `/_stucco/`. Serve them through
`stucco_tower::assets_router`, or write the files exposed by `Bundle::paths`
and `Bundle::get` to your static asset directory.

## Serve with Axum

Add `stucco-tower = "0.1"`, `axum = "0.8"`, and
`tokio = { version = "1", features = ["macros", "rt-multi-thread", "net"] }`.

```rust,no_run
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
    let app = with_standard_layers(
        routes.merge(assets_router(bundle)),
        &LayerConfig::default(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await
}
```

The [hello example](https://github.com/mcaveniathor/stucco/tree/main/examples/hello)
also demonstrates fragment responses.

## Themes

Fourteen presets cover common looks, each in light and dark. For anything
else, build a `Theme`, or derive a whole one from a seed:

```rust
use stucco::Bundle;
use stucco::theme::{Elevation, Fonts, Preset, Radius, Seeded, TableStyle, Theme};

// Colours, fonts, type scale, spacing, radius, density and a style
// personality, all from one number. The same seed always gives the same theme,
// and seeded themes always pass the contrast checks.
let theme = Theme::seeded(42);
// Or hash a name, then adjust any option.
let theme = Theme::seeded_str("acme").elevation(Elevation::Raised);
// Every option implements `Seeded`, so you can seed options one at a time:
// here, Slate's colours with seed 14's fonts, corners and tables.
let theme = Theme::preset(Preset::Slate)
    .fonts(Fonts::seeded(14))
    .radius(Radius::seeded(14))
    .table_style(TableStyle::seeded(14));
let bundle = Bundle::new(theme.build().expect("contrast passes"));
```

An option seeded on its own matches what the full seeded theme chooses for it,
because each option draws from its own named stream. Implement `Seeded` for
your own types with `SeedRng`.

For a surprise, every `Seeded` type also implements `Random`:
`Theme::random()` picks a fresh seed, and `Theme::random_with_seed()` also
returns that seed so you can keep a result with `Theme::seeded(seed)`.

The style personality changes presentation without changing markup: surface
elevation, table rows, input style, header treatment, heading weight and
button shape. Add more themes to a bundle with `Bundle::with_theme` and apply
them to a subtree with `ThemeScope`. The gallery's seeded themes page shows a
dozen seeds side by side.

## Collections and persistence

Enable the `collections` feature for typed columns, a `DataTable`, native GET
controls, and cursor or numbered pagination:

```toml
stucco = { version = "0.1", features = ["collections"] }
```

```rust
use stucco::collections::{Col, DataTable};
use stucco::{CollectionPage, to_html};

let page = CollectionPage {
    rows: vec!["Ada", "Grace"],
    next: None,
    prev: None,
    total: None,
};
let table = DataTable::from_page(&page, "Customers")
    .column(Col::text("name", "Customer", |row: &&str| row.to_string()));
assert!(to_html(&table).contains("Ada"));
```

Declare sortable/searchable/filterable columns and provide backend capabilities
to enable their controls. `CollectionQuery` validates incoming query parameters;
`CollectionSource` in `stucco-tower` loads the corresponding page.
`stucco-redb` adds typed records, transactional secondary indexes, and bounded
blocking scans.

The [orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders)
persists 67 seeded records and demonstrates the complete flow. Its default
cursor mode leaves the total unknown. Numbered mode deliberately scans the whole
collection and is intended for small datasets. Authentication, authorization,
schema migrations, and mutation endpoints belong to the application.

## Crates and compiler support

| Crate | Purpose | Minimum Rust |
| --- | --- | --- |
| `stucco` | Facade and component families | 1.85 |
| `stucco-theme` | Themes, color scales, and contrast validation | 1.85 |
| `stucco-core` | Rendering, assets, pages, fragments, and query state | 1.85 |
| `stucco-ui` | Component implementations | 1.85 |
| `stucco-tower` | Tower services, Axum responses, and collection sources | 1.85 |
| `stucco-redb` | Optional redb 4.3 storage adapter | 1.90 |

## Features

The facade enables `layout`, `typography`, `actions`, `forms`, `feedback`,
`navigation`, `data`, and `app` by default. Use `default-features = false`
to select only the families you need.

| Feature | Adds |
| --- | --- |
| `layout` | Stack, Cluster, Grid, Container, Sidebar, and related primitives |
| `typography` | Headings, text, links, code, and keyboard keys |
| `actions` | Buttons, button links, and icon buttons |
| `forms` | Forms, fields, inputs, selects, and choice controls |
| `feedback` | Empty states and live status regions |
| `navigation` | Pagination |
| `data` | Cards, panels, semantic tables, and result counts |
| `app` | Application shells, page/section headers, and footers |
| `collections` | Typed DataTable and GET controls; enables data/forms/navigation/feedback |
| `icons` | Vendored Lucide icon catalog |

The manifest also contains reserved feature names for planned families
(`overlay`, `marketing`, `diagram`, `markdown`, `askama`, and `maud`).
They do not yet provide those integrations or component families. The reserved
`overlay` name is also in the default set; it currently adds nothing beyond
`actions`.

`stucco-tower` enables `axum` and `tower-http` by default.
`stucco-redb` enables its `tokio` collection adapter by default; disable defaults
for synchronous storage alone.

## Try the examples

```sh
git clone https://github.com/mcaveniathor/stucco.git
cd stucco
cargo run -p hello                 # http://localhost:4180
cargo run -p orders                # http://localhost:4181/orders
cargo run -p gallery -- target/gallery
```

Serve `target/gallery` with a static file server to explore the gallery.
The orders example supports `PORT` and `ORDERS_DB` environment variables;
its default database is `target/orders.redb`.

## Development and releases

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd browser
npm ci
npx playwright install chromium firefox webkit
npm test
```

See the [release guide](https://github.com/mcaveniathor/stucco/blob/main/RELEASING.md)
for package verification, dependency order, and the first crates.io release.

## License

Licensed under MIT or Apache-2.0, at your option. The optional Lucide icon
catalog is covered by the ISC notice distributed as
[`LICENSE-LUCIDE`](https://github.com/mcaveniathor/stucco/blob/main/crates/stucco-ui/LICENSE-LUCIDE).
Regenerate it with `cd tools && npm install && npm run icons`.
