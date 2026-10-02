# stucco-ui

Semantic HTML components for stucco. Enable component families with Cargo
features: `layout`, `typography`, `actions`, `forms`, `feedback`,
`navigation`, `data`, `app`, `collections`, or `icons`.
This implementation crate has no default component families.

With the `actions` feature and a direct dependency on stucco-core:

```rust
use stucco_ui::{actions::Button, Variant};
let html = stucco_core::to_html(&Button::new("Save").submit().variant(Variant::Primary));
assert!(html.contains("Save"));
```

The `collections` feature composes tables and native GET search, filters,
sorting, and pagination. Components use backend capabilities without depending
on a database, HTTP server, or Tokio.

Minimum Rust: **1.85**. The 0.2 API is experimental.
Most applications should use the [stucco facade](https://github.com/mcaveniathor/stucco)
which re-exports the components with common families enabled.

Licensed under MIT or Apache-2.0. Vendored Lucide icons are ISC licensed;
their notice is included in LICENSE-LUCIDE.
