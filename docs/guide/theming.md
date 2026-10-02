# Theming

Start from a preset, build your own theme, or derive one from a seed. Every theme is checked for WCAG contrast before it builds, and components only use semantic tokens, so any theme works with every component.

## Presets

Fourteen presets cover common looks, each with light and dark schemes: Slate (the default), Graphite, Stone, Sand, Sepia, Pine, Fjord, Ocean, Iris, Rose, Ember, Sol, Terminal and Contrast.

```rust
use stucco::{Bundle, theme::Preset};

let bundle = Bundle::new(Preset::Pine);
```

Pages follow the operating system's light or dark preference. Set `data-theme="light"` or `"dark"` on any element to force a scheme for that subtree. To let visitors choose, save `"light"` or `"dark"` under the `stucco-theme` localStorage key: every page's head script applies it before first paint.

## Building a theme

`Theme` is a builder. Start from a preset or an accent hue and change what you need:

```rust
use stucco::theme::{Color, Density, Fonts, Radius, Theme, TypeScale};

let theme = Theme::from_seed(160.0) // accent hue in OKLCH degrees
    .accent(Color::oklch(0.6, 0.14, 160.0))
    .neutral_hue(150.0)
    .neutral_tint(0.012)
    .fonts(Fonts::system().sans("Inter"))
    .type_scale(TypeScale::new(16.0, 1.25))
    .radius(Radius::Round)
    .density(Density::Compact);
let built = theme.build()?; // fails with a report if any pair misses its contrast target
```

From the accent and neutral hues, stucco generates twelve-step OKLCH scales for neutral, accent and the four status colours, maps them to semantic roles such as `surface`, `text-muted` and `accent-text`, and checks every text and interface pair in both schemes. `high_contrast()` steepens the scales so text pairs reach AAA.

## Personality

Six options change how components look without changing their markup:

| Option | Choices |
| --- | --- |
| `elevation` | Flat, Outlined (default), Raised |
| `table_style` | Lined (default), Striped, Open |
| `control_style` | Outlined (default), Filled, Underlined |
| `header_style` | Bar (default), Plain, Tinted |
| `heading_weight` | Regular, Bold (default), Heavy |
| `button_shape` | Rounded (default), Pill |

Each option sets tokens such as `--st-card-shadow` or `--st-table-rule` that the component styles read.

## Seeded themes

`Theme::seeded(n)` derives a complete theme from one number: the palette, fonts, type scale, spacing, corners, density and personality. The same seed gives the same theme on every platform, and seeded themes always pass the contrast checks. `Theme::seeded_str("acme")` hashes a name into a seed.

Every option implements the `Seeded` trait, so you can also seed options one at a time and mix them with a preset:

```rust
use stucco::theme::{Fonts, Preset, Radius, Seeded, TableStyle, Theme};

let theme = Theme::preset(Preset::Slate)
    .fonts(Fonts::seeded(14))
    .radius(Radius::seeded(14))
    .table_style(TableStyle::seeded(14));
```

An option seeded on its own matches what the full seeded theme chooses for it, because each option draws from its own named stream. Adding options in future releases won't change the ones you already rely on.

`Random` gives any seeded type a fresh seed: `Theme::random_with_seed()` returns the theme and the seed that made it, so you can keep a result you like with `Theme::seeded(seed)`. Implement `Seeded` for your own types with `SeedRng`.

## Several themes on one page

Add named themes to a bundle and apply them to a subtree:

```rust
use stucco::{Bundle, layout::ThemeScope, theme::Preset};

let bundle = Bundle::new(Preset::Slate).with_theme("brand", Preset::Iris);
let promo = ThemeScope::new().named("brand").child(/* … */);
```

## Playground and export

The [theme playground](../playground.html) runs the same Rust code, compiled to WebAssembly. Pick a preset or seed, override any option, preview both schemes, and export:

- **Rust**: the builder calls that produce the theme.
- **CSS**: the full token stylesheet, for use outside Rust or alongside your own styles.
- **JSON**: every colour role's light and dark value.

The `stucco` command-line tool exports the same formats; see [Command line](cli.html).

The Theme menu at the top of this site applies any preset or a random theme to the whole site, including the gallery.

## Custom styles

Component CSS only uses semantic tokens (`var(--st-surface)`, `var(--st-accent-text)`) and never palette steps or colour literals, so it adapts to any theme. Follow the same rule in your own styles, and put them in the `stucco.utilities` cascade layer so they override components without specificity battles:

```css
@layer stucco.utilities {
  .promo { background: var(--st-accent-soft); color: var(--st-accent-text); }
}
```
