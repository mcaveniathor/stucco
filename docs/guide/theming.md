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

Twenty-five options change how components look without changing their markup:

| Option | Choices |
| --- | --- |
| `elevation` | Flat, Outlined (default), Raised |
| `table_style` | Lined (default), Striped, Open |
| `control_style` | Outlined (default), Filled, Underlined |
| `header_style` | Bar (default), Plain, Tinted |
| `heading_weight` | Regular, Bold (default), Heavy |
| `button_shape` | Rounded (default), Pill |
| `link_style` | Underlined (default), Subtle, Bold, Highlight |
| `nav_style` | Soft (default), Solid, Bar |
| `focus_style` | Ring (default), Thick, Snug |
| `finish` | Smooth (default), Sand, Float, Knockdown |
| `shell_layout` | Sidebar (default), Rail, Topbar |
| `panel_style` | Boxed (default), Ruled, Headed |
| `corner_style` | Even (default), Squircle, Bevel, Hand |
| `line_weight` | Fine (default), Heavy |
| `button_depth` | Flat (default), Raised, Offset |
| `label_style` | Plain (default), Caps, Strong |
| `icon_weight` | Light, Regular (default), Bold |
| `motion` | Smooth (default), Snappy, Gentle, Springy |
| `heading_font` | Body (default), Serif, Rounded, Mono |
| `leading` | Tight, Normal (default), Airy |
| `tag_style` | Pill (default), Square, Outline |
| `rule_style` | Solid (default), Dashed, Dotted |
| `material` | Solid (default), Glass, Frost |
| `backdrop` | Plain (default), Glow, Wash |
| `shadow_style` | Soft (default), Layered, Tinted, Hard |

Each option sets tokens such as `--st-card-shadow` or `--st-table-rule` that the component styles read. Every link style keeps an underline, so links never depend on colour alone.

### Layout and shape

`shell_layout` arranges `AppShell`. Sidebar puts the primary links at the top of a filled column beside the content, and Rail does the same in a narrower column on the page background. Topbar puts the primary links in a row under the header. The sidebar, if the page has one, stays a column beside the content in every layout, so nested navigation keeps working. See [Application shell](components.html#application-shell).

`panel_style` frames panels: Boxed is a bordered box, Ruled drops the box for a strong rule above the title, and Headed sets the title in a raised band.

`corner_style` shapes the corners of cards, panels, filter bars and table cards. Squircle and Bevel use the CSS `corner-shape` property, so browsers without it show ordinary rounded corners. Hand makes each corner slightly different, like plaster shaped by hand. Every style keeps the theme's radius, so sharp themes stay sharp.

### Type

`heading_font` sets the typeface of headings and table captions, leaving body text in the theme's fonts: a serif for an editorial look, a rounded sans where the platform has one, or the theme's monospace font. Like the body fonts, these are system fonts, so nothing is downloaded.

`leading` sets the line height of running text: 1.4, 1.5 or 1.65.

### Lines, depth and labels

`line_weight` sets the width of the borders and rules that outline cards, panels, controls, fieldsets and the application shell. Heavy doubles them, for a bold, graphic look; table row rules stay fine either way.

`button_depth` decides how buttons stand off the page. Flat buttons have no shadow and shrink slightly when pressed. Raised buttons have a soft shadow and a highlight along the top, and sink when pressed. Offset buttons cast a hard shadow in the text colour, down and to the side, and slide onto it when pressed. Ghost buttons stay flat in every style.

`label_style` styles small labels: table column headings, the sidebar's heading and the labels above collection controls. Plain is small, semibold and muted; Caps sets them in spaced capitals; Strong uses bold text in the full text colour.

`tag_style` draws tags, such as status values in tables, as filled pills, filled labels with small corners, or outlined pills.

`rule_style` makes dividing rules (separators, table row rules and the footer's top rule) solid, dashed or dotted. The outlines of cards, panels and controls stay solid.

`icon_weight` sets the stroke width of icons: 1.5, 2 or 2.5.

`motion` sets the durations and easing of transitions, such as hover fills and button presses. Snappy is quicker than the default, Gentle slower and softer, and Springy overshoots slightly so presses bounce. Visitors who ask for reduced motion get none, whatever the style.

```rust
use stucco::theme::{ButtonDepth, LineWeight, Preset, Radius, Theme};

// A bold, graphic look: sharp corners, heavy lines and offset buttons.
let theme = Theme::preset(Preset::Slate)
    .radius(Radius::Sharp)
    .line_weight(LineWeight::Heavy)
    .button_depth(ButtonDepth::Offset);
```

### Transparency and depth

`material` decides what large surfaces are made of: cards, panels, filter bars, table cards, the application header and the side column. Glass is 70% opaque with a light blur and a lit top edge; Frost is 85% opaque with a heavier blur. Both show the backdrop and finish through them, and turn solid for visitors who ask for reduced transparency or more contrast. Inputs and other controls stay solid.

`backdrop` puts soft colour behind the page, drawn from the theme's soft accent: Glow adds two glows at the top of the page, and Wash fades from the top down. Pages drop it for visitors who ask for more contrast.

`shadow_style` draws shadows at every elevation. Soft is one blurred shadow; Layered stacks close and far shadows for realistic depth; Tinted colours them with the accent; Hard drops the blur for solid ledges. Shadows show on surfaces that `elevation` raises.

`build` checks text on the page against the backdrop's and finish's strongest points, and text on translucent surfaces against every colour the page behind them can show, so a theme that builds stays readable.

```rust
use stucco::theme::{Backdrop, Elevation, Material, Preset, ShadowStyle, Theme};

let theme = Theme::preset(Preset::Slate)
    .material(Material::Glass)
    .backdrop(Backdrop::Glow)
    .elevation(Elevation::Raised)
    .shadow_style(ShadowStyle::Layered);
```

### Finish

`finish` gives the page background a plaster texture: a fine sand grain, a softer float coat, or the broad patches of a knockdown coat. Cards, inputs and other surfaces stay smooth. The grain is a faint grey, so it works in both schemes. `build` checks text on the page background against the grain's darkest and lightest points, and pages drop the grain for visitors who ask for more contrast.

```rust
use stucco::theme::{Finish, Preset, Theme};

let theme = Theme::preset(Preset::Sand).finish(Finish::Float);
```

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
