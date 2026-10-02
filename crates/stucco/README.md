# stucco

Server-rendered, themeable UI components for Rust. This is the facade crate:
start here for layouts, typography, buttons, forms, tables, and application shells.
The 0.1 API is experimental.

```rust
use stucco::prelude::*;
use stucco::layout::{Container, Stack};
use stucco::typography::{Heading, Text};

let bundle = Bundle::new(Preset::Slate);
let html = Page::new(&bundle, "Hello")
    .body(el::main().id("main").child(Container::new().child(
        Stack::new().child(Heading::new(1, "Hello"))
            .child(Text::new("Rendered in Rust."))
    )))
    .render();
assert!(html.contains("Rendered in Rust."));
```

Serve the Bundle's assets with stucco-tower, or write them to a static directory.
Enable `collections` for typed columns and native GET collection controls;
enable `icons` for the Lucide catalog. Use `default-features = false` to select
individual families. Reserved feature names for overlays, marketing, diagrams,
Markdown, Askama, and Maud are not implemented yet.

Minimum Rust: **1.85**. Storage and HTTP integrations are separate crates.
See the [repository README](https://github.com/mcaveniathor/stucco) for installation,
features, runnable examples, and the release status.

Licensed under MIT or Apache-2.0. The optional Lucide catalog uses ISC.
