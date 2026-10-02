# stucco-theme

Themes for stucco: OKLCH color scales, semantic CSS tokens, light/dark schemes,
presets, and contrast validation. This crate has no dependencies.

```rust
use stucco_theme::{BuiltTheme, Preset};
let theme: BuiltTheme = Preset::Slate.into();
// Pass this theme to stucco_core::Bundle, or customize a Theme builder.
```

Minimum Rust: **1.85**. The 0.1 API is experimental.
See [stucco](https://github.com/mcaveniathor/stucco) for the component library
and runnable theme gallery.

Licensed under MIT or Apache-2.0.
