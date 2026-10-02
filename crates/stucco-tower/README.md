# stucco-tower

Tower integration for stucco: asset services, page and fragment responses,
request negotiation, request context, and backend collection source traits.

The default `axum` and `tower-http` features add Axum asset routing and the
standard middleware stack. Disable defaults for the base Tower/HTTP interfaces.

With direct dependencies on stucco-core, stucco-theme, and axum:

```rust
use std::sync::Arc;
use stucco_core::Bundle;
use stucco_theme::Preset;
use stucco_tower::{LayerConfig, assets_router, with_standard_layers};

let bundle = Arc::new(Bundle::new(Preset::Slate));
let routes = axum::Router::new().merge(assets_router(bundle));
let app = with_standard_layers(routes, &LayerConfig::default());
```

`CollectionSource` loads owned pages through a Send future.
`RequestContext` preserves typed request extensions, allowing applications to
provide their own authentication and authorization context. The default stack
does not implement an application's authorization policy.

Minimum Rust: **1.85**. The 0.1 API is experimental.
See [the hello example](https://github.com/mcaveniathor/stucco/tree/main/examples/hello)
for a runnable application.

Licensed under MIT or Apache-2.0.
