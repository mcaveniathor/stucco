# Changelog

## 0.2.1 — 2026-10-02

Interactive components arrive: dialogs, menus, toasts, tooltips and tabs,
built on native HTML so they work with little or no script. This release
also fixes headers with shaped edges clipping their menus, empty table pages
losing their pagination, and a caching header. No 0.2.0 API was removed or
changed incompatibly.

- The `overlay` family (on by default) adds `Dialog`, `Menu`, `Toast` and
  `Tooltip`, built on native `<dialog>`, the popover API and invoker
  commands. One behaviour script, loaded only where they are used, adds
  invoker commands and outside-click closing to browsers without them,
  anchors menus under their buttons from their first frame, makes toasts
  dismissible and lets Escape hide tooltips. All four are in the prelude.
- `Tabs` (navigation) are links to sections of a page, each with its own
  URL, styled as tabs.
- A dialog's link opener opens the dialog only on a plain primary click;
  Ctrl/Cmd, Shift, Alt and middle clicks, links with a `target`, and clicks
  another handler already cancelled follow the link as usual.
- `AppShell`'s header no longer clips what opens out of it, such as a
  menu or dropdown panel, when the theme gives it a shaped edge. The fill,
  pattern, blur and edge are painted on a layer behind the header's content,
  and the header sits above the content that follows it.
- A `DataTable` page with no rows still shows its pagination, so a visitor
  past the last page (from a stale link, or after deleting the last row)
  can get back with their search and filters kept. `Pagination` links back
  to page 1 from past the end even when everything fits on one page, and
  never links to a page 0.
- Page and fragment responses send `Vary: Stucco-Request, Stucco-Target`,
  since negotiation reads both, so a cache can't serve a full page cached
  for an invalid fragment request in place of a fragment.
- The gallery has an Overlays page and a working Tabs example, a page per
  tab, and the documentation site's gallery pages carry the site's
  navigation. The browser tests now cover each
  overlay, and the site in Chromium, Firefox and WebKit.

## 0.2.0 — 2026-10-02

Applications get easier to start and write: one-call axum setup, page
shortcuts, a fuller prelude, derived table columns and an app generator.
Themes can be seeded, and gain twenty-eight personality options, including
generated SVG ornaments, with a playground and a command-line tool. No 0.1
API was removed or changed incompatibly.

- `stucco new` writes a starter axum app: a theme (a preset, a seed, a name
  or random), a shared layout with navigation, a home page and a table with
  derived columns. It takes stucco from crates.io, its repository (`--git`)
  or a local checkout (`--stucco-path`).
- A recipes page in the guide answers common questions in a few lines each:
  shared layouts, forms with errors, filtered tables, partial updates, a
  second theme, static pages and request limits.
- `DataTable` over rows without capabilities no longer shows a filter bar
  of controls that can't do anything, and its empty state no longer
  suggests clearing filters.
- `#[derive(Columns)]` (the new `stucco-macros` crate, through `stucco`'s
  `derive` feature) builds a row type's data table columns from its fields,
  with `#[col(...)]` for the kind, key, label, value, display and controls.
  The `Columns` trait's `columns()` go to `DataTable::columns`, and its
  `column_specs()` to the query parser, so the two always agree.
  `Col::spec` exposes a column's key and kind.
- The orders example derives its columns.
- `Stack::of`, and `of` on every layout container, takes all the children in
  one call: one item, or a tuple of up to twelve.
- A `Theme` can go straight into `Bundle::new` (and `Router::stucco`): it is
  built there, and panics with the contrast report if it fails.
  `Bundle::try_new` returns the report instead. `From<Theme> for
  BuiltTheme` makes this work anywhere a built theme is taken.
- `CollectionSource::load` parses a request's query string and reads the
  page in one call, returning a `Collection` (query, capabilities and page)
  that `DataTable::from_collection` takes whole.
- `PageCx::respond` routes fragment requests: name the regions with
  `fragment(id, || content)`, then `page(|| document)`. Only the output that
  is sent is rendered, and requests for other regions get the full page.
  `PageCx::kind` exposes the request kind.
- The hello and orders examples use these.
- Easier setup for axum apps:
  - `Router::stucco(theme)` (the `StuccoRouter` trait in `stucco-tower`)
    serves a bundle's assets, adds the standard layers and makes the bundle
    available to handlers in one call; `stucco_with` takes a `LayerConfig`.
  - The `PageCx` extractor builds a `Document`, a page that handlers return
    directly. Its content is rendered when set, so it can borrow request
    data; `status` sets the response code, and `PageCx::fragment` answers
    fragment requests.
  - The `stucco` crate's new `axum` feature re-exports `stucco-tower` as
    `stucco::server` and puts `PageCx`, `Document` and `StuccoRouter` in the
    prelude.
- `PageExt` adds `main` and `app` to pages and documents: `main` wraps
  content in `<main id="main">` after a skip link, and `app` takes an
  `AppShell`. They build on the new `WithBody` trait in `stucco-core`.
- `stucco::prelude` now includes the everyday components of each enabled
  feature (layout, typography, actions, forms, feedback, navigation, data,
  app and collections) and the shared `Size`, `Space`, `Tone`, `Variant`
  and `Measure` enums.
- The hello and orders examples use the new setup.
- The finish grain's filter works in sRGB, so its grey is the 0.5 that
  `build` checks text against. It was computed in linear light, which drew
  it as a lighter 0.735 grey: grain was slightly weaker on light pages and
  slightly lighter than checked on dark ones.
- Three ornament options drawn as generated SVG, each seedable and in
  `ThemeSpec`, the CLI and the playground (`relief`, `pattern`, `edge`):
  - `Relief` (Flat, Venetian, Trowel, Skip) lights a raised plaster surface
    on the page background;
  - `Pattern` (Plain, Dots, Grid, Contours, Waves) draws a faint motif
    behind the application header;
  - `HeaderEdge` (Straight, Wave, Zigzag, Torn) shapes the header's bottom
    edge.
  `Theme::motif_seed` varies the relief's light, the pattern's layout and
  contours, and the edge's rhythm; seeded themes take it from their seed.
  `build` fits the relief's and pattern's strength to the theme's contrast,
  and both go under `prefers-contrast: more`. Defaults reproduce the
  previous look, and existing seeds keep every option they had.
- Three personality options for transparency and depth, each seedable and
  in `ThemeSpec`, the CLI and the playground (`material`, `backdrop`,
  `shadows`):
  - `Material` (Solid, Glass, Frost) makes cards, panels, filter bars, table
    cards, the header and the side column translucent and blurred; they turn
    solid under `prefers-reduced-transparency`, `prefers-contrast: more` and
    forced colours;
  - `Backdrop` (Plain, Glow, Wash) adds soft accent colour behind the page;
  - `ShadowStyle` (Soft, Layered, Tinted, Hard) draws the shadow steps.
  `build` now checks text on the page against the backdrop as well as the
  finish, and text on translucent surfaces against every colour behind them.
  Defaults reproduce the previous look, and existing seeds keep every option
  they had.
- Shadows are black in dark schemes instead of the light text colour, so
  they no longer glow.
- Four more personality options, each seedable and in `ThemeSpec`, the CLI
  and the playground (`heading-font`, `leading`, `tags`, `rules`):
  - `HeadingFont` (Body, Serif, Rounded, Mono) sets the typeface of
    headings, using system fonts only;
  - `Leading` (Tight, Normal, Airy) sets the line height of running text;
  - `TagStyle` (Pill, Square, Outline) draws `.st-tag`;
  - `RuleStyle` (Solid, Dashed, Dotted) patterns separators, table row rules
    and the footer's top rule.
  Defaults reproduce the previous look, and existing seeds keep every option
  they had.
- Five more personality options, each seedable and in `ThemeSpec`, the CLI
  and the playground (`lines`, `depth`, `labels`, `icons`, `motion`):
  - `LineWeight` (Fine, Heavy) sets the width of outlines and rules through
    `--st-line`;
  - `ButtonDepth` (Flat, Raised, Offset) gives buttons a shadow and a
    matching press;
  - `LabelStyle` (Plain, Caps, Strong) styles table column headings, the
    sidebar's heading and collection control labels;
  - `IconWeight` (Light, Regular, Bold) sets the icon stroke;
  - `Motion` (Smooth, Snappy, Gentle, Springy) sets transition durations and
    easing; reduced motion still turns transitions off.
  The motion tokens now come from the theme's personality. Defaults
  reproduce the previous look, and existing seeds keep every option they
  had.
- `AppShell` separates primary navigation from the sidebar: `link` and
  `links` add a flat list of `NavLink`s in a `<nav>` landmark (named with
  `nav_label`, default "Main"), and `sidebar` holds section navigation or a
  nested outline. The shell body is now a grid placed by the theme's shell
  layout, with `data-nav` and `data-sidebar` marking which regions exist.
- `NavLink` renders a navigation link that can mark the current page with
  `aria-current="page"`.
- Three more personality options, each seedable and in `ThemeSpec`, the CLI
  and the playground (`shell`, `panels`, `corners`):
  - `ShellLayout` (Sidebar, Rail, Topbar) arranges `AppShell`: the primary
    links sit at the top of the side column, or in a row under the header,
    and the sidebar stays a column beside the content;
  - `PanelStyle` (Boxed, Ruled, Headed) frames `Panel`;
  - `CornerStyle` (Even, Squircle, Bevel, Hand) shapes cards, panels, filter
    bars and table cards, using `corner-shape` where browsers support it.
  The playground preview gains an app-shell sample. Defaults reproduce the
  previous look, and existing seeds keep every option they had.
- Four more personality options, each seedable, in `ThemeSpec` and the CLI
  (`links`, `nav`, `focus`, `finish`) and in the playground:
  - `LinkStyle` (Underlined, Subtle, Bold, Highlight) for `.st-link`;
  - `NavStyle` (Soft, Solid, Bar) for the current page in the sidebar;
  - `FocusStyle` (Ring, Thick, Snug) for the focus outline;
  - `Finish` (Smooth, Sand, Float, Knockdown): a plaster grain on the page
    background. `build` checks text on the page against the grain, and the
    grain is removed under `prefers-contrast: more` and forced colours.
  Defaults reproduce the previous look, and existing seeds keep every option
  they had.
- `stucco-cli`, the `stucco` command: `stucco theme` prints a preset, seeded,
  named or random theme as token CSS (optionally scoped), Rust builder code,
  JSON colour roles, a query string or a summary, with a flag for every
  option; `stucco presets` and `stucco options` list the choices.
- `stucco_theme::spec` (re-exported as `stucco::theme::spec`): `ThemeSpec`
  describes a theme as a base (preset, seed or name) plus option overrides,
  parses and writes query strings such as `seed=42&radius=round`, builds the
  `Theme`, and writes the Rust code for it. `tokens_json` exports colour roles.
  The playground and the CLI both use it.
- `FormState::from_urlencoded` parses a form body (dropping `_csrf`), and
  `FormState::field_errors` lists fields with errors.
- `Validator` checks a submitted `FormState` and builds typed values, or returns
  the state with one message per invalid field.
- `ErrorSummary` lists a failed submission's errors above the form, links them
  to their controls, and takes focus on load.
- Collections look finished by default: the filter bar is a card built from
  the standard inputs, selects and buttons, with all controls one height; range
  filters sit side by side and stack with visible labels in narrow containers;
  the table is a card with a muted header row, sort indicators, end-aligned
  tabular numbers, and enumeration values shown as tags (`.st-tag[data-value]`);
  the result count and pagination share a footer row. Filter and sort labels
  come from column labels (`FilterBar::column_label`), and the reset link now
  reads "Clear" in the bar and "Clear filters" in the empty state.
- `Col::display` formats a column's cells without changing how it sorts or
  filters; `Row::cell_with_attrs` sets attributes on a data cell.
- The application shell has a fixed-width sidebar that stacks on narrow
  screens, styled navigation links (with `aria-current="page"`), a surface
  header and a quieter footer. `PageHeader` wraps its description in
  `.st-page-header-description`.
- Pagination links are bordered buttons with hover and current states.
- Text inputs and selects share `--st-control-h` as a fixed height.
- `ResultCount` says "Showing 1 result" instead of "Showing 1 results".
- Fieldsets and radio groups no longer reset their margins, so layout
  primitives space them like any other child.
- Pages emit `theme-color` meta tags for the light and dark background.
- Buttons scale to 0.96 while pressed; danger buttons keep their tint on
  hover; controls set `touch-action: manipulation`; icons read their stroke
  width from `--st-icon-stroke` (default 2).
- Panel titles render at the `lg` size; `PageHeader::size` and
  `SectionHeader::size` set a heading's size independently of its level;
  keyboard keys in `Kbd` are spaced.
- `DESIGN.md` records the design language and default tokens.
- A documentation site, built with stucco and deployed to GitHub Pages: a
  landing page, a guide written in Markdown, a theme playground (presets,
  seeds and per-option overrides, with Rust, CSS and JSON export), the
  component gallery, and the API reference. A Theme menu on every page
  applies any preset or a random theme to the whole site. The playground runs
  `stucco-theme` compiled to WebAssembly.
- `Theme::seeded(u64)` and `Theme::seeded_str(&str)` derive a complete theme
  (hues, tint, fonts, type scale, spacing, radius, density and personality)
  from a seed. Every seed passes the contrast checks; the mapping is pinned by
  a test and changes only in minor releases.
- The `Seeded` trait (`from_rng`, `seeded`, `seeded_str`) is implemented by
  `Theme` and every option: `Palette` (new: accent, neutral hue and tint, ink),
  `Fonts`, `TypeScale`, `Radius`, `Density` and the six personality enums.
  Each option draws from its own named fork of a `SeedRng`, so
  `Radius::seeded(s)` equals the radius `Theme::seeded(s)` picks, and adding
  options never reshuffles existing ones. `Theme::palette` applies a palette.
- The `Random` trait, implemented for every `Seeded` type, gives `random()`
  and `random_with_seed()` from a fresh seed (`random_seed()`); the returned
  seed recreates the value with `seeded`.
- Style personality options on `Theme`: `elevation`, `table_style`,
  `control_style`, `header_style`, `heading_weight` and `button_shape`. They
  emit tokens (`--st-card-shadow`, `--st-table-rule`, `--st-control-bg`, …)
  that the component CSS reads, so markup is unchanged. Defaults reproduce the
  previous look.
- Invalid inputs show a ring instead of a thicker border, so they no longer
  shift layout.
- Themes now also check `text` and `text-muted` against `accent-soft`; a custom
  theme with a very dark or saturated soft accent can now fail `build`.
- `stucco-tower`: the `Submission` axum extractor (415 for other content types)
  and the `SeeOther` 303 redirect for post-redirect-get.
- docs.rs now labels items that require a cargo feature.
- Document that the reserved `overlay` feature is enabled by default and adds nothing yet.
- CI checks each cargo feature on its own and runs `cargo-semver-checks` against
  the published release.

## 0.1.0 — 2026-10-02

Initial release:

- OKLCH themes, semantic color tokens, presets, and contrast validation.
- Escaped HTML rendering, checked URLs, page/fragment output, and asset bundles.
- Layout, typography, actions, forms, feedback, navigation, data, and app components.
- Typed collection columns with native GET search, filters, sorting, and pagination.
- Tower/Axum services, responses, request context, and collection sources.
- redb 4.3 storage with transactional indexes and bounded blocking collection scans.
- Hello, persistent orders, and static gallery examples.

Core, components, and Tower require Rust 1.85. The redb adapter requires Rust 1.90.
The API is experimental; later 0.x releases may introduce breaking changes.
