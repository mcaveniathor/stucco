# Signature directions and complete screens

Atelier, Precision, Prism and Play coordinate typography, density, surfaces and
motion. They are additional recipes; existing presets and `Theme::seeded` keep
their original mappings.

```rust
use stucco::theme::{Direction, Radius};
let theme = Direction::Atelier.theme().radius(Radius::Soft);
let variation = Direction::Prism.guided_v1(42);
```

## Choose a direction

| Direction | Composition |
| --- | --- |
| Atelier | Warm terracotta, system serif headings, quiet rules, gentle motion |
| Precision | Cool grey/blue, compact controls, crisp sans headings, sharp edges |
| Prism | Violet, layered depth, selective frost, luminous backdrop |
| Play | Gold, heavy rounded headings, strong outlines, offset button shadows |

Each supports light and dark. Tables and reading content stay opaque. The
project's focal card uses the material; supporting metrics use simple rules.
Reduced transparency makes translucent surfaces solid. Increased contrast and
forced colors remove decorative backgrounds; reduced motion disables movement.

Explore [orders](../gallery/showcase.html), the
[reading page](../gallery/showcase-atelier-editorial.html), and the
[project overview](../gallery/showcase-atelier-project.html). Each page links all
four directions and renders both schemes without JavaScript. The copyable
compositions live in `examples/showcase`, an unpublished workspace crate with no
HTTP or storage dependency. Replace its sample content with application data;
keep page-scoped metrics honestly labeled.

## Guided variations and locks

The [playground](../playground.html) defaults to guided Atelier. Version 1 varies
compatible palettes, typography and shape choices. Seed zero is the exact
signature. Unrestricted mode still accepts the original presets, numeric seeds
and text names.

Palette, typography and surfaces/motion locks capture the current effective
choices when generating. Unlocked groups return to the new seed's choices.
Changing direction clears locks and overrides so the new signature is visible.
Explicit option edits remain available. Six favorites can be stored locally;
when storage is unavailable they last only for the current visit. Comparison
uses the same screen, scheme and narrow-width setting for both themes.

The canonical recipe retains the direction, generation version, seed and
resolved overrides. Rust, CLI and WebAssembly use the same implementation:

```sh
stucco theme --direction atelier --format rust
stucco theme --guided prism:1:42 --format css
stucco theme 'guided=atelier:1:42&palette=sand&heading-font=serif' --format json
```

JSON preserves the existing `light` and `dark` color maps and adds
`recipe.format` and `recipe.query`. Feed that query back into the CLI or parse it
with `stucco::theme::spec::ThemeSpec` to recreate typography and all other
choices. CSS exports contain all theme tokens. Unsupported generation versions
are rejected by the strict parser. Browser URL parsing is lenient for old or
malformed links. Fonts are opt-in presentation assets, separate from theme
recipes; their setup must accompany an exported theme if used.

## Data recipes

`Avatar`, `Badge`, `Stat` and ordinary text compose identity cells and statuses.
`Progress::new(label, value, maximum).show_value()` provides native progress
semantics and a visible percentage.

```rust
use stucco::data::{Sparkline, Timeline, TimelineEntry};
let trend = Sparkline::new("Tasks per week: 2 to 7 over six weeks", [2., 3., 3., 5., 4., 7.]);
let activity = Timeline::new("Recent activity, newest first")
    .entry(TimelineEntry::new("2026-10-21T14:30:00Z", "21 October · 14:30 UTC", "Keyboard review complete"));
```

Sparklines display the supplied summary and expose all finite samples as text.
Empty series say “No data”; one sample is a point; a constant series is a flat
line. Non-finite values are omitted. Supply ordered data and a meaningful period
and unit in the summary. Timelines use an ordered list and native timestamps;
callers determine order and time-zone labels. Optional entry icons are
decorative, so the content must explain the activity.

## Optional local fonts

The default directions use system fonts and request no font assets. The
showcase's optional `atelier-fonts` feature registers local Newsreader and DM
Sans faces through the existing asset registry. Add `showcase-fonts` to an
Atelier composition to use them:

```sh
cargo run -p gallery --features atelier-fonts -- target/gallery-fonts
```

Open `showcase-fonts.html` in that output. The faces use `font-display: optional`
to avoid a late swap; system sans and Georgia remain usable fallbacks. This
opt-in adds about 127 KB of uncompressed CSS (95,016 bytes of WOFF2 before base64).
It is deliberately excluded from default builds. The bundled Latin subset
covers the showcase's accented names; other scripts and italic styles use the
system fallback. See `examples/showcase/fonts/README.md` for exact coverage,
weights, provenance and the included SIL Open Font Licenses.
