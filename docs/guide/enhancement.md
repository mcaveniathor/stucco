# Progressive enhancement

Pages work as plain HTML first. When you want a faster interaction, answer the same request with a fragment and let a small script swap it in.

## The baseline

Everything in stucco works with JavaScript turned off: links navigate, GET forms filter and search, POST forms submit and redirect. Build that first. Enhancement then changes how a response arrives, not what it contains, so the fallback never drifts from the enhanced path.

## Fragment requests

An enhanced request sends two headers: `Stucco-Request: fragment` and `Stucco-Target: <element id>`. `PageCx::respond` reads them: name the regions that can be requested on their own, then the full page. Only the output that is sent gets rendered, and a request for any other region gets the full page:

```rust
use stucco::prelude::*;

async fn index(page: PageCx, query: Query<Params>) -> Response {
    let name = query.name.as_deref();
    page.respond()
        .fragment("greeting", || greeting(name))
        .page(|| page.title("Hello").main(content(name)))
}
```

For finer control, `RequestKind` is an extractor of its own, and `stucco::server::respond` runs exactly one of a full-page and a fragment closure.

Fragments render through `render_fragment`, which prefixes generated ids with a namespace so a fragment's ids can't collide with the page's. Responses vary on `Stucco-Request` and `Stucco-Target`, and fragments are never cached, so a cache never serves one in place of the other. The [hello example](https://github.com/mcaveniathor/stucco/tree/main/examples/hello) is a complete program.

## Behaviours

A component that needs client code declares an `Asset` with a `Behavior`: a dependency-free ES module. Pages load a behaviour only when something on the page requires it.

```rust
use stucco::{Asset, Behavior, register_asset};

pub static TABS: Asset = Asset {
    name: "tabs",
    css: Some(include_str!("tabs.css")),
    behavior: Some(Behavior::Js(include_str!("tabs.js"))),
    deps: &[],
};
register_asset!(TABS);
```

Call `cx.require(&TABS)` in the component's `render`. A full page links the module; a fragment response announces it in an `<st-require>` element, and the stucco runtime loads it once, only from the bundle's own directory. If a module fails to load, the runtime marks the nearest `data-st-region` as failed and dispatches `stucco:asset-error`, so the page can fall back to a full navigation.

Call `Page::enhanced()` on pages that will receive fragments before any behaviour is on the page, so the runtime is already loaded.

## Delivery

Pages link one hashed, cacheable stylesheet by default. `Page::delivery(Delivery::Inline)` instead inlines only the CSS and scripts the page uses, for single-file output such as emails or offline documents; inline pages can't receive fragments.

## Content Security Policy

`Page::csp_nonce` adds a nonce to every inline `<script>` and `<style>` the page emits, including the theme script in the head, so pages work under a strict nonce-based policy.
