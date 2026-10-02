# stucco — design

Date: 2026-10-01 · Status: draft for review

## 1. Purpose

`stucco` is a public (crates.io) Rust library for building server-rendered web UIs.
It gives Rust web applications a typed, themeable, accessible component kit whose
output is plain HTML and CSS, enhanced by small JavaScript behaviours only where a
component needs them. Diagrams are one optional module.

Primary goals, in order: **modularity** (pay only for what you use, at compile time
and on the page) and **intuitiveness** (one rendering trait, one set of component
conventions, sensible defaults).

### 1.1 Origin

The design starts from `skimasque-visual` and keeps its strongest ideas: builder-style
component structs, escape-by-construction (markup can only come from rendering a
component), token-driven theming checked by tests, accessible captions on diagrams,
and markup that works before any script runs. It drops everything SkiMasque-specific
(the fixed `NodeKind` vocabulary, the Alpine palette, access/policy/decision cards,
the site pages) and the hard dependency on askama.

### 1.2 Roadmap context

This spec covers sub-project 1 only: the component library. Sub-project 2, a
tower-based web framework built around it, gets its own spec later. This library must
not depend on any server framework; its one integration point for the framework is
`Bundle` (§4.5), which the framework will wrap as a tower `Service`.

### 1.3 Non-goals for v1

- No routing, server, dev server, hydration or islands (framework sub-project).
- No WebAssembly. Behaviours are JavaScript; WASM is reserved for future
  compute-heavy client work (first candidate: automatic graph layout), and the asset
  model already accommodates it (§4.4).
- No proc-macro templating DSL. One may be added later as a separate crate on top of
  the element builder.
- Deferred features: server-rendered SVG charts and sparklines, automatic diagram
  layout, syntax highlighting, self-hosted fonts, a custom date picker (native
  `<input type="date">` is used).

## 2. Users and success criteria

Users are Rust developers rendering HTML on the server with any framework (axum,
actix-web, the future stucco framework) or generating static pages, with or without
an existing template engine (askama, maud).

v1 succeeds when:

1. A user can depend on `stucco`, enable a few features, and render a themed,
   accessible page from an axum handler in under 20 lines, without Node or a bundler.
2. Every component works with JavaScript disabled; behaviours only enhance.
3. A page ships only the scripts for behaviours present on it.
4. User CSS overrides library styles without specificity hacks.
5. All presets pass WCAG AA contrast in light and dark; axe-core reports no
   violations on any gallery page.
6. Every public item is documented with a runnable example.

## 3. Workspace layout

Users depend on the `stucco` facade only and select features.

| Crate | Published | Purpose | Depends on |
|---|---|---|---|
| `stucco-core` | yes | `Render`, `Cx`, element builder, escaping, `Href`, `Attrs`, `Asset`, `Page`, `Bundle`, `behavior` vocabulary; `askama` / `maud` adapter features | — |
| `stucco-theme` | yes | `Theme`, OKLCH colour, palettes, presets, token CSS generation, contrast checking | core |
| `stucco-ui` | yes | Components, feature-gated by family | core, theme |
| `stucco-diagram` | yes | Diagram system | core, theme, ui (icons) |
| `stucco` | yes | Facade: re-exports and features | all of the above |
| `gallery` | no | Docs/gallery site generator and browser test bed | stucco |

Facade features: `layout`, `typography`, `forms`, `feedback`, `navigation`,
`overlay`, `data`, `marketing`, `shells`, `diagram`, `icons`, `markdown`, `askama`,
`maud`. Default: `layout`, `typography`, `forms`, `feedback`, `navigation`,
`overlay`, `data`, `shells`, `icons`. Feature dependencies are explicit (for example
`shells` enables `navigation`; `diagram` enables `icons`).

## 4. Core (`stucco-core`)

### 4.1 Rendering

```rust
pub trait Render {
    fn render(&self, cx: &mut Cx);
}
```

`Cx` (render context) owns the output buffer, the asset collector and a per-page id
generator (`cx.id("tabs")` returns `tabs-1`, `tabs-2`, …; unique within one page
render). Rendering is infallible.

`Render` is implemented for:

- `&str`, `String`, `Cow<str>`, integers, floats, `char` — escaped as text;
- `Option<T: Render>` — `None` renders nothing;
- `Vec<T>`, slices, and tuples up to 12 — rendered in order;
- `&T`, `Box<T>`, `Rc<T>`, `Arc<T>` where `T: Render`;
- `F: Fn(&mut Cx)` via the wrapper `render_fn(f)`.

`Slot = Box<dyn Render>` is the type components use for arbitrary children. Any
`Render + 'static` value converts into a `Slot` via `Into`.

`fn to_html(r: &impl Render) -> String` renders a fragment standalone (assets are
discarded; meant for tests and htmx-style partial responses, where `Page`'s asset
handling does not apply).

### 4.2 Element builder

Module `stucco::el` has one function per HTML element (`el::div()`, `el::a()`,
`el::input()`, …) returning `Element`. Void elements reject children at the type
level (`VoidElement` has no `child` method).

```rust
el::section().class("hero").id("intro")
    .attr("hx-get", "/more")
    .bool_attr("hidden", collapsed)
    .data("state", "open")              // data-state="open"
    .aria("label", "Introduction")      // aria-label="…"
    .child(el::h1().text("Hello"))
    .child_if(show, Alert::warning("Beta"))
    .children(items.iter().map(|i| el::li().text(&i.name)))
```

Rules:

- Attribute values and text are always escaped.
- `.class()` appends and de-duplicates; it never replaces.
- Attribute names are validated (ASCII letters, digits, `-`, `_`, `:`, `.`); invalid
  names panic in debug builds and are dropped in release builds.
- URL-bearing attributes (`href`, `src`, `action`, `formaction`, `poster`) accept
  only `Href`. `Href::new` accepts relative URLs and the schemes `http`, `https`,
  `mailto`, `tel`; others (including `javascript:` and `data:`) produce
  `Href::invalid()`, which renders as `#` and is reported by `Href::is_valid`.
  `Href::trusted(s)` bypasses the check.
- Raw markup is only possible via `Raw::trusted(s)`, named so that it is easy to find
  in review.

### 4.3 Component conventions

Every component in every crate follows the same shape:

- `Component::new(required…)` plus chained setters that take `impl Into<…>`;
- every component exposes `.class()`, `.id()`, `.attr()`, `.data()`, `.aria()`
  through a shared `Attrs` value applied to its root element, so users can add
  classes, ids or htmx attributes without forking;
- variants and sizes are enums (`Variant::Primary`, `Size::Sm`) rendered as
  `data-variant` / `data-size`;
- components are plain data: `Clone` where their slots allow, `Debug` always.

### 4.4 Assets

```rust
pub struct Asset {
    pub name: &'static str,          // "tabs"
    pub css: Option<&'static str>,   // component stylesheet chunk
    pub behavior: Option<Behavior>,  // client-side enhancement
    pub deps: &'static [&'static Asset],
}
pub enum Behavior { Js(&'static str) /* , Wasm(…) reserved */ }
```

A component calls `cx.require(&TABS)` while rendering. Requirements are de-duplicated
and resolved with dependencies in a stable order.

The `behavior` module defines the client contract: behaviour names, custom-element
tag names (`st-tabs`), and `data-*` attribute and state names. A build-time test
generates the matching JavaScript constants and fails if the scripts drift from them.

### 4.5 Bundle and Page

`Bundle` is built once at startup and owns all static assets:

```rust
let bundle = Bundle::new(Theme::preset(Preset::Slate))
    .with_theme("marketing", Theme::preset(Preset::Iris));
```

It generates the stylesheet (tokens for each theme, base layers, and the CSS of every
component compiled in), and every behaviour script, each content-hashed and served
under a configurable prefix (default `/_stucco/`). Integration with any server:

```rust
bundle.get(path) -> Option<AssetFile>   // bytes, MIME type, ETag, immutable cache flag
```

`Page` renders a complete document:

```rust
Page::new(&bundle, "Dashboard")
    .lang("en")
    .meta(Meta::description("…").og_image(href))
    .head(extra)                      // user head content
    .body(app)
    .csp_nonce(nonce)
    .render()                         // -> String
```

`Page` renders the body first, then emits `<head>` from the collected requirements.
Delivery modes (`.delivery(…)`):

- `Linked` (default): one site-wide stylesheet link (identical on every page, so it
  stays cached) plus one `<script type="module">` per behaviour used on this page;
- `Inline`: only the CSS chunks this page uses, in a `<style>` element, and used
  behaviours as inline module scripts — for emails, embeds and single files.

The theme-switch script that prevents a flash of the wrong theme is inlined in the
head. When a CSP nonce is set, every inline `<script>` and `<style>` the page emits
(including those of `Inline` delivery) carries it.

### 4.6 Template engine adapters

- Feature `askama`: any `askama::Template` can be wrapped with `Askama(t)`, which
  implements `Render` (template errors render an HTML comment in release and panic in
  debug). stucco values implement `Display` through `Rendered(r)` for embedding in
  askama templates with `|safe`.
- Feature `maud`: `maud::Markup` implements `Render`; stucco values implement
  `maud::Render`.

Assets required by components embedded inside foreign templates are still collected,
because the adapters render into the active `Cx`.

## 5. Theme and CSS (`stucco-theme`)

### 5.1 Theme API

```rust
Theme::from_seed(250.0)                        // OKLCH hue of the accent
    .neutral_tint(0.01)                        // chroma of the neutral scale
    .accent(Color::oklch(0.65, 0.18, 145.0))
    .fonts(Fonts::system().mono("JetBrains Mono"))
    .type_scale(TypeScale::new(16.0, 1.2))     // base px, ratio; fluid via clamp()
    .space(4.0)                                // spacing unit in px
    .radius(Radius::Soft)
    .density(Density::Comfortable)
    .build()?                                  // Result<BuiltTheme, ContrastReport>
```

- Palettes: 12-step OKLCH scales for neutral, accent, success, warning, danger, info,
  each generated separately for light and dark.
- Semantic roles map onto scale steps: `--st-bg`, `--st-surface`, `--st-surface-raised`,
  `--st-text`, `--st-text-muted`, `--st-border`, `--st-border-strong`, `--st-accent`,
  `--st-accent-text`, `--st-on-accent`, `--st-focus`, and `--st-{success,warning,danger,info}`
  with `-text` and `-soft` variants. Diagram tones get their own roles.
- Other tokens: type scale steps, spacing steps, radii, elevation shadows, motion
  durations and easings, z-index layers.
- `build()` checks every text/background role pair against WCAG AA (4.5:1 text,
  3:1 large text and UI) in both schemes and returns a report on failure.
  `Bundle::new` takes `impl Into<ThemeSource>`; presets are pre-validated and
  infallible.

### 5.2 Presets

`Theme::preset(Preset::X)` returns a customisable `Theme`. All presets are
contrast-checked in CI in both schemes.

| Preset | Neutral | Accent | Character |
|---|---|---|---|
| Slate (default) | cool grey | blue | neutral app/dashboard |
| Graphite | true grey | ink (accent = text) | minimal, monochrome |
| Stone | warm grey | amber | friendly, warm |
| Sand | paper beige | terracotta | editorial / long-form |
| Sepia | parchment | brown | reading mode |
| Pine | green-tinted | mint | calm, natural |
| Fjord | arctic blue-grey | frost blue | muted, cool |
| Ocean | blue-teal | cyan | crisp, technical |
| Iris | violet-tinted | violet | creative, product |
| Rose | rose-tinted | pink | soft, expressive |
| Ember | charcoal | red-orange | bold, high energy |
| Sol | warm cream | gold | bright, optimistic |
| Terminal | near-black | phosphor green | monospace, sharp corners |
| Contrast | pure black/white | strong blue | WCAG AAA throughout |

Exact OKLCH values are tuned during implementation and recorded in the presets
module.

### 5.3 Light and dark

Roles are declared once with `light-dark()`. Without a `data-theme` attribute the
operating system preference decides; `data-theme="light"` or `"dark"` on any element
forces that scheme for its subtree by setting `color-scheme`. Named extra themes apply
to a subtree with `data-st-theme="<name>"`. The theme switcher behaviour persists the
choice in `localStorage` and tolerates storage being unavailable.

### 5.4 CSS architecture

```css
@layer stucco.reset, stucco.tokens, stucco.base, stucco.layout,
       stucco.components, stucco.utilities;
```

Unlayered user CSS always wins over the library.

- Class names use the `st-` prefix (`st-card`, `st-card-title`). Variants and states
  are data attributes shared with the behaviour contract.
- Components use container queries for their own responsiveness and logical
  properties throughout (right-to-left layouts work).
- `prefers-reduced-motion: reduce` disables non-essential animation.

Enforced by tests:

1. No colour literals outside generated token CSS.
2. Component CSS references only semantic role tokens, never palette steps.
3. Every role is defined for both schemes in every preset.
4. All component CSS sits inside a `stucco.*` layer.
5. Reduced motion disables every animation and transition marked non-essential.

## 6. Components (`stucco-ui`)

**JS** marks components with a behaviour. Each JS component works without its
script; the fallback is stated.

### 6.1 Icons (feature `icons`)

Vendored Lucide set (ISC licence, attribution shipped) exposed as typed constants:
`Icon::new(icon::CHECK).label("Done")` (labelled icons get `role="img"`, unlabelled
ones are `aria-hidden`). `Page` emits a sprite containing only the icons used on that
page. `Icon::custom(name, svg)` registers user icons (the SVG is trusted input).

### 6.2 Layout (feature `layout`)

Stack, Cluster, Sidebar, Switcher, Center, Container, Grid (auto-fit, min column
width), Cover, Frame (aspect ratio), Reel. Each takes spacing from the theme's
spacing steps (`Space::S3`).

### 6.3 Typography (feature `typography`)

Heading (level and visual size set separately), Text (size, tone, weight), Link,
Code, Kbd, Blockquote, Prose (styles arbitrary long-form HTML children). Feature
`markdown` adds `Prose::markdown(src)` (pulldown-cmark; raw HTML in the source is
escaped unless `allow_html()` is set).

### 6.4 Forms (feature `forms`)

- `Field` wraps a control with label, hint and error; it wires `for`/`id`,
  `aria-describedby` and `aria-invalid` automatically, and accepts server-side
  validation errors via `.error(msg)` or `.errors(iter)`.
- Controls: Input (types; prefix/suffix adornments), Textarea, Select, Checkbox,
  RadioGroup, Switch (styled checkbox), Range, File, Fieldset, Button
  (variant, size, loading state, `type`), ButtonGroup, Form (method, action, CSRF
  token slot).
- **JS** PasswordInput reveal toggle (fallback: plain password input).
- **JS** Combobox: filters a `<datalist>`-backed or server-backed option list with
  full keyboard support (fallback: native `<input list>`).

### 6.5 Feedback (feature `feedback`)

Alert/Callout, Badge, Tag, StatusDot, Progress (native), Meter (native), Spinner,
Skeleton, EmptyState. **JS** Toast region (fallback: messages render inline as Alerts).

### 6.6 Navigation (feature `navigation`)

Navbar (responsive; the mobile menu is a `<details>` disclosure), Breadcrumbs,
Pagination, SidebarNav, Steps, TableOfContents, SkipLink. **JS** Tabs (fallback: tab
list is a list of anchor links to the panels, all panels visible).

### 6.7 Overlay (feature `overlay`)

- Dialog and Drawer: native `<dialog>` opened by invoker commands
  (`command="show-modal"` / `commandfor`); no script required.
- Popover: native popover API.
- Tooltip: CSS anchor positioning, text also available to assistive technology via
  `aria-describedby`.
- **JS** DropdownMenu: popover plus menu keyboard navigation (fallback: popover with
  ordinary focusable links/buttons).

### 6.8 Data display (feature `data`)

Card, Table (sticky header, responsive overflow, optional sortable headers as server
links), DescriptionList, Avatar (image or initials), Stat (KPI tile), Timeline,
Accordion (`<details>`), **JS** CodeBlock copy button (fallback: no button rendered
without script; the block stays selectable).

**DataTable** — one API, two modes:

```rust
DataTable::new(&rows)
    .column(Col::text("Name", |r| &r.name).sortable().searchable())
    .column(Col::enumeration("Status", |r| r.status).filter())
    .column(Col::number("Size", |r| r.bytes).sortable().filter())
    .column(Col::date("Created", |r| r.created).sortable())
    .query(&query)                // TableQuery parsed from the request
    .page(Pagination::new(page, per_page, total))
    .client_side()                // optional; omit for server mode
```

- Column kinds (text, number, date, enumeration, custom) set sort order and filter
  control: enumeration → select, number/date → min/max range, searchable text → the
  shared search box.
- **Server mode (default):** state lives in query parameters
  (`sort`, `dir`, `q`, `f.<column>`, `page`). Headers are sort links, filters and
  search are a GET form; works without JS on any data size. `TableQuery::parse(query_string, &columns)`
  validates parameters against the declared columns (unknown or malformed parameters
  are ignored, never errors) and exposes `sort()`, `filters()`, `search()`, `page()` for
  the handler to apply to its data source. A helper `TableQuery::apply(&mut Vec<T>)`
  covers in-memory data.
- **Client mode (JS):** for tables with all rows on the page. Sorting, filtering and
  search happen in place, a polite `aria-live` region announces result counts, and the
  URL query parameters are updated with `history.replaceState`, so links remain
  shareable and work in server mode when scripts are off.
- Headers carry `aria-sort`; all controls are real buttons, links and form controls.

### 6.9 Marketing (feature `marketing`)

Hero, Section, FeatureGrid, CtaBand, ComparisonTable, Faq (`<details>`), PricingTiers,
Testimonial, LogoCloud. Ported and generalised from `skimasque-visual`'s chrome.

### 6.10 Shells (feature `shells`)

AppShell (header, collapsible sidebar, main), DocsShell (sidebar nav, content, table
of contents), MarketingShell (navbar, content sections, footer), Footer, and the
**JS** ThemeSwitcher (fallback: hidden; the OS preference applies).

## 7. Diagrams (`stucco-diagram`, feature `diagram`)

```rust
pub trait NodeKind {
    fn label(&self) -> &str;
    fn icon(&self) -> Icon;
    fn tone(&self) -> Tone;
}

Node::new(kinds::DATABASE).label("Orders").sub("db.prod:5432").status(Status::Healthy)
Node::new(Kind::new("Ledger", icon::BOOK).tone(Tone::Structure))
```

- Starter kinds in `diagram::kinds`: service, database, queue, user, client, api,
  cloud, server, function, storage, external, gateway, process, decision.
- Tones: Primary, Structure, Edge, Neutral, Success, Danger, Info, Warning, each
  backed by theme roles.
- Connections: Data (solid), Control (dashed), Active (emphasised), Potential
  (dotted), Denied (broken with ×). Each takes a label; its meaning is also present as
  visually hidden text.
- Containers: Flow, Branch, Layers, Sequence, Compare, Boundary (group / trust zone),
  Topology. Each requires a caption, which becomes its text equivalent.
- Width-safety is enforced by types: containers implement `Block`, simple nodes
  implement `Inline`; content-sized slots (a Flow step, a Branch root) accept only
  `Inline`, while width-defined slots (Compare sides, Branch arms, Boundary bodies)
  accept `Block`. This replaces the doc-comment warning in `skimasque-visual`.
- Motion: `Reveal` (steps appear in order) and **JS** `Walkthrough` (playable stages
  with captions, user-supplied; fallback: all stages listed in order).
- Rendering stays HTML/CSS (not absolutely positioned SVG), so diagrams reflow on
  narrow screens, follow the theme and remain readable by screen readers.

## 8. Client behaviours

- Each behaviour is a dependency-free ES module defining one custom element
  (`<st-tabs>`, `<st-combobox>`, `<st-data-table>`, …) that wraps native markup in the
  light DOM (no shadow DOM, so theme CSS applies).
- `connectedCallback` / `disconnectedCallback` attach and detach behaviour, so
  content inserted later (htmx swaps, the future framework) enhances itself without a
  global scan.
- Behaviours read configuration from `data-*` attributes defined in
  `core::behavior`, never from inline JSON or globals.
- Behaviours: theme-switcher, tabs, dropdown-menu, combobox, password-reveal, toast,
  copy, data-table, walkthrough.
- Target: each module under 4 KB minified and gzip-compressed; scripts are shipped
  unminified-but-small source in v1 (no JS build step), and the size budget is checked
  in CI against gzip output.

## 9. Error handling

- Rendering cannot fail. Invalid input degrades safely: invalid attribute names
  are dropped (debug panic), unsafe URLs become `#`, unknown table query parameters are
  ignored.
- `Theme::build()` is the one fallible public API, returning `ContrastReport`
  (each failing role pair, scheme, measured and required ratio).
- `Bundle::get` returns `None` for unknown paths; callers map that to 404.

## 10. Quality

### 10.1 Tests

- Rust unit tests per component: markup contract, escaping, aria wiring, id
  uniqueness, declared assets.
- Snapshot tests (`insta`) of the rendered HTML of every gallery example.
- CSS rule tests (§5.4) and contrast tests for every preset in both schemes.
- Behaviour contract test: generated JS constants match `core::behavior`.
- Browser tests with Playwright (Chromium, Firefox and WebKit) against the generated
  gallery: keyboard interaction for every JS component, DataTable in both modes,
  axe-core (`@axe-core/playwright`) on every page in both schemes, and a pass with
  JavaScript disabled (`javaScriptEnabled: false`) to verify fallbacks.
- Node, Playwright and axe are development-only; crate users never need Node.

### 10.2 Documentation

- `#![deny(missing_docs)]`; every public component has a runnable rustdoc example.
- Gallery site: every component with variants and copyable code, the palette page
  (each preset's scales and role mapping), a live theme switcher across all presets.
  Built in CI, deployable to GitHub Pages.
- `examples/`: axum, actix-web, askama embedding, maud embedding, static site
  generation, DataTable server mode, DataTable client mode.

### 10.3 Tooling

- Rust edition 2024, MSRV 1.85 (checked in CI).
- `cargo clippy -D warnings`, `cargo fmt --check`, `cargo-deny` (licences, advisories),
  `cargo-semver-checks` before each release.
- GitHub Actions CI. Licence: MIT OR Apache-2.0.
- Repository: `C:\Users\Thor\Documents\src\stucco`, local git only until the user
  asks for a remote or a release.

## 11. Build order

Each step ends with its gallery pages and tests green.

1. Workspace, `stucco-core` (Render, Cx, element builder, Href, Attrs, assets, Page,
   Bundle), gallery skeleton.
2. `stucco-theme`: OKLCH, scales, roles, CSS generation, contrast checks, all presets,
   palette page.
3. Base CSS layers, icons, layout, typography.
4. Forms.
5. Feedback, navigation, overlay, and the behaviour runtime conventions plus the
   Puppeteer harness.
6. Data display including DataTable (both modes).
7. Shells and theme switcher.
8. Marketing.
9. Diagrams.
10. Adapters (askama, maud), examples, documentation pass, release checklist.
