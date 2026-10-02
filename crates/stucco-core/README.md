# stucco-core

Rendering contracts for stucco: safe markup, attributes and URLs, generated
identity, asset bundles, pages, fragments, form state, and collection query state.
This crate is independent of HTTP and asynchronous runtimes.

```rust
use stucco_core::{el, to_html};
let html = to_html(&el::p().text("<untrusted>"));
assert_eq!(html, "<p>&lt;untrusted&gt;</p>");
```

Text escapes by default. `Raw` is an explicit opt-out for trusted markup;
applications must not use it with untrusted data. `Href` rejects unsafe URL
schemes. `CollectionQuery` validates search, sort, typed filters, and page state
against application-provided capabilities.

Minimum Rust: **1.85**. The 0.1 API is experimental.
Most applications should use the [stucco facade](https://github.com/mcaveniathor/stucco).

Licensed under MIT or Apache-2.0.
