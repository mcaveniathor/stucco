# stucco

Server-rendered, themeable UI components for Rust. This is the facade crate:
start here for layouts, typography, buttons, forms, tables, and application shells.
The 0.2 API is experimental.

```rust
use stucco::prelude::*;

let bundle = Bundle::new(Preset::Slate);
let html = Page::new(&bundle, "Hello")
    .main(Container::new().child(
        Stack::new()
            .child(Heading::new(1, "Hello"))
            .child(Text::new("Rendered in Rust.")),
    ))
    .render();
assert!(html.contains("Rendered in Rust."));
```

Enable `axum` to serve pages from axum handlers: `Router::stucco` serves the
Bundle's assets, and handlers return a `Document` built from the `PageCx`
extractor. Without it, write the assets to a static directory.
Enable `collections` for typed columns and native GET collection controls;
enable `icons` for the Lucide catalog. Use `default-features = false` to select
individual families. Overlays (dialogs,
menus, toasts and tooltips) are on by default. Reserved feature names for
marketing, diagrams, Markdown, Askama, and Maud are not implemented yet.

Minimum Rust: **1.85**. Storage is a separate crate.
See the [repository README](https://github.com/mcaveniathor/stucco) for installation,
features, runnable examples, and the release status.

Licensed under MIT or Apache-2.0. The optional Lucide catalog uses ISC.
