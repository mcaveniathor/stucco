# stucco — component library design

Date: 2026-10-01 · Revision 2 · Status: draft for review

Revision 2 applies `2026-10-01-stucco-spec-improvements.md`. Server execution, Tower
integration and data contracts are specified separately in
`2026-10-01-stucco-integration-design.md`. Open choices are collected in §15.

## 1. Purpose and boundaries

`stucco` is a public (crates.io) Rust library for building server-rendered web UIs: a
typed, themeable, accessible component kit whose output is plain HTML and CSS,
enhanced by small JavaScript behaviours only where a component needs them. Diagrams
are one optional module.

Priorities, in order: **modularity** (pay only for what you use, at compile time and
on the page), **intuitive APIs** (one rendering trait, one set of conventions,
sensible defaults), **accessible native HTML**, **progressive enhancement**,
**framework-independent rendering**, **minimal shipped assets**.

### 1.1 Boundaries

| Crate | Owns | Never does |
|---|---|---|
| `stucco-theme` | token generation, scales, presets, contrast validation | rendering |
| `stucco-core` | rendering, safe markup, identity, asset requirements, pages, fragments, the client runtime, presentation-state types | HTTP, I/O, async |
| `stucco-ui` | primitives, compositions, shells | executing requests, queries or mutations |
| `stucco-diagram` | diagram rendering and enhancement | — |
| `stucco` | lightweight component facade (re-exports + features) | server dependencies |
| `stucco-tower` (integration spec) | Tower services and layers, response conversion, data/action contracts | — |

The component library compiles without Tower, an HTTP server or an async runtime.
Components receive data, state and endpoint descriptions; rendering never performs
database queries or backend mutations.

### 1.2 Origin

The design starts from `skimasque-visual` and keeps: builder-style component
structs, escape-by-construction, token-driven theming enforced by tests, captioned
diagrams, and markup that works before scripts run. It drops everything
SkiMasque-specific and the hard dependency on askama.

### 1.3 Non-goals for v1

- Routing, a framework facade, hydration or islands (integration spec, later scope).
- WebAssembly. Behaviours are JavaScript; the asset model reserves a WASM variant for
  future compute-heavy work (first candidate: automatic graph layout).
- A proc-macro templating DSL (possible later crate on top of the element builder).
- HTML email. Email clients need a restricted subset; no component or delivery mode
  is guaranteed to work in email.
- Everything marked **later** in the catalogue (§8).

## 2. Users and success criteria

Users are Rust developers rendering HTML on the server with any framework, or
generating static pages, with or without askama/maud.

v1 succeeds when:

1. Depending on `stucco` pulls in no server, HTTP or async-runtime crate.
2. A themed, accessible page renders from an axum handler in under 20 lines, without
   Node or a bundler.
3. Every component works with JavaScript disabled; behaviours only enhance.
4. A page ships only the behaviour modules it uses; a fragment inserted later can
   introduce behaviour modules the page did not have, each loaded once, without ID
   collisions.
5. User CSS overrides library styles without specificity hacks.
6. All presets pass WCAG AA contrast in light and dark; axe-core reports no
   violations on any gallery page in the browser matrix (§11).
7. Every public item is documented with a runnable example.
8. Minimal, selected-combination and all-features builds compile and pass tests.

## 3. Features

Users depend on `stucco` and select features. Defaults: `layout`, `typography`,
`actions`, `forms`, `feedback`, `navigation`, `overlay`, `data`, `app`.

| Feature | Requires (unavoidable) | Optional embellishment |
|---|---|---|
| `layout` | — | — |
| `typography` | — | — |
| `actions` | — | — |
| `forms` | `actions` | — |
| `feedback` | — | — |
| `navigation` | — | — |
| `overlay` | `actions` | — |
| `data` | — | — |
| `collections` | `data`, `forms`, `navigation`, `feedback` | — |
| `app` | `navigation`, `layout` | — |
| `marketing` | `layout`, `typography`, `actions` | — |
| `diagram` | `icons` (starter node kinds use them) | — |
| `icons` | — | adds the Lucide set; any component with an icon slot accepts it |
| `markdown` | `typography` | — |
| `askama`, `maud` | — | adapters (§4.9) |

The `Icon` **type** (a name plus SVG path data) is always available, so components
with icon slots never require the `icons` feature; the feature only adds the vendored
Lucide constants (ISC licence, attribution shipped). No text-only component depends
on icons.

## 4. Core (`stucco-core`)

### 4.1 Rendering

```rust
pub trait Render {
    fn render(&self, cx: &mut Cx);
}
```

`Cx` owns the output buffer, the asset requirements and the identity state (§4.3).
Rendering is infallible.

`Render` is implemented for `&str`, `String`, `Cow<str>`, integers, floats, `char`
(escaped text); `Option<T>` (`None` renders nothing); `Vec<T>`, slices and tuples up
to 12; `&T`, `Box<T>`, `Rc<T>`, `Arc<T>`; closures via `render_fn(|cx| …)`.

**Slots.** `Slot<'a>` is a struct wrapping `Box<dyn Render + 'a>` and the
`type_name` of the value it was built from; its `Debug` prints that type name, so
components holding slots still derive `Debug`. Components carry the lifetime
(`Card<'a>`), which lets slots borrow request data (`Card::new().body(&rows[0])`)
without cloning. Builders infer the lifetime; users rarely write it.

### 4.2 Pages and fragments

- `Page` renders a complete document (§4.7).
- `render_fragment(namespace: &str, r: &impl Render) -> RenderedFragment`:

  ```rust
  pub struct RenderedFragment {
      pub html: String,
      pub assets: AssetRequirements,   // resolved, dependency-ordered
  }
  ```

  `fragment.to_response_html(&bundle) -> String` prefixes the HTML with an
  `<st-require>` element listing the hashed module URLs of required behaviours
  (§4.8). Fragments are the API for partial responses.
- `to_html(r) -> String` is an HTML-only helper for tests, static snippets and
  places where no behaviour can be required; it discards requirements and its docs say
  so. It is not recommended for partial responses.

### 4.3 Identity policy

- A component given an explicit id (`.id("orders")`) uses it verbatim on its root and
  derives internal ids from it: `orders-label`, `orders-panel-2`. Explicit ids are
  stable across rerenders regardless of render order.
- Components without an explicit id get generated ids `{namespace}{prefix}-{n}`,
  where `n` counts per prefix within one render. Prefixes match `[a-z0-9]+` (no
  hyphen), so an id parses uniquely and different namespaces can never collide. `Page` renders with the empty
  namespace; `render_fragment` requires a non-empty namespace (validated:
  `[a-z0-9-]+`, rendered as `{namespace}-`). Generated ids are stable across rerenders
  that use the same namespace and render the same structure.
- Fragments must use a namespace that is unique within the document they are
  inserted into; the conventional namespace is the target element's id or a record
  key (`order-42`). This is what prevents collisions between independently rendered
  fragments.
- Uniqueness of explicit ids is the caller's responsibility. `Cx` records every id
  emitted; a duplicate within one render panics in debug builds and is kept in
  release builds.
- Regions that are replaced by fragments should use explicit ids on interactive
  controls so focus can be restored (§4.8).

### 4.4 Element builder and attribute safety

`stucco::el` has one function per standard HTML element; `el::custom(tag)` accepts
custom element names (lowercase, containing a hyphen, validated). Void elements
return `VoidElement`, which has no child methods.

```rust
el::section().class("hero").id("intro")
    .attr("hx-get", "/more")
    .bool_attr("hidden", collapsed)
    .data("state", "open")
    .aria("label", "Introduction")
    .child(el::h1().text("Hello"))
    .child_if(show, Alert::warning("Beta"))
    .children(items.iter().map(|i| el::li().text(&i.name)))
```

Rules:

- Text and attribute values are always escaped, except text inside `<script>`
  and `<style>`, which is written as-is with `</` neutralised so the element
  cannot be closed early; script content is code, so untrusted text must not go
  there. Attribute names are case-insensitive (stored lowercased).
- Attribute names: ASCII letter first, then letters, digits, `-`, `_`, `:`, `.`.
  `data(name)` and `aria(name)` names: `[a-z0-9-]+`; `aria` names are checked against
  the WAI-ARIA 1.2 attribute list in debug builds.
- `attr()` refuses executable or HTML-bearing attributes: names starting with `on`
  and `srcdoc`. URL-bearing attributes (`href`, `src`, `action`, `formaction`,
  `poster`, `cite`, `data` on `<object>`, `xlink:href`, `ping`, `srcset`) are checked
  per URL through `Href` rules; a rejected URL renders as `#` (or is dropped from a
  list).
- Invalid or refused names panic in debug builds and are dropped in release builds.
- The escape hatches are `trusted_attr(name, value)` (any valid name, no URL check,
  value still escaped) and `Raw::trusted(html)`. Both are named to be found in review;
  they are the trust boundary.
- `Href::new` accepts relative URLs and the schemes `http`, `https`, `mailto`, `tel`
  after browser-style normalisation (leading whitespace/control characters stripped,
  tab/newline removed); `Href::trusted` bypasses the check.
- `.class()` always appends and de-duplicates. `.id()` replaces. Setting any other
  attribute twice keeps the last value.

### 4.5 Component conventions

**Composition levels.** All levels implement `Render`.

1. **Primitives** — one layout, semantic, visual or interaction concern (Button,
   Stack, Input).
2. **Compositions** — recurring patterns with named slots and shared accessibility
   wiring (Field, Panel, PageHeader, DataTable).
3. **Shells and recipes** — page structure (AppShell, DocsShell, MarketingShell) and
   gallery recipes assembled only from public components.

**Builders.** `Component::new(required…)` plus chained setters taking
`impl Into<…>`. Variants and sizes are enums rendered as `data-variant` /
`data-size`.

**Slots.** Standard slot names, used wherever they apply: `heading`, `description`,
`media`, `actions`, `body`, `meta`, `footer`. Each accepts `impl Render + 'a`.
Compositions collect assets by rendering their children; no composition requests
assets on a child's behalf.

**Passthrough attributes.** Every component exposes `.class()`, `.id()`, `.attr()`,
`.data()`, `.aria()` via a shared `Attrs` applied to its root element. Each component
documents its **reserved attributes** (ids it wires, `role`, behaviour `data-*`
state); setting a reserved attribute through passthrough panics in debug builds and
is ignored in release builds (the component's value wins).

**Landmarks.** Only shells emit `<main>` (with `id="main"`, the `SkipLink` target),
top-level `<header>`, `<footer>` and primary `<nav>`. Panels, cards and sections never
emit `main`; a `Section` or `Panel` with a heading renders `<section
aria-labelledby>`, without a heading a `<div>`.

**Headings.** Semantic level and visual size are independent:
`Heading::new(2, "Orders").size(Size::Xl)`. Compositions with a heading slot take
`.level(n)` (default 2); `PageHeader` defaults to 1.

**Public types.** A constituent gets its own public type only if it is used
standalone or has two or more independent options (`Feature`, `PricingTier`,
`AccordionItem`, `MenuItem`). Otherwise it is a builder method on its parent
(`Tabs::tab(label, panel)`, `List::item(..)`). Names in the catalogue (§8) are
reserved even when not yet implemented.

### 4.6 Assets

```rust
pub struct Asset {
    pub name: &'static str,
    pub css: Option<&'static str>,
    pub behavior: Option<Behavior>,
    pub deps: &'static [&'static Asset],
}
#[non_exhaustive]
pub enum Behavior { Js(&'static str) /* Wasm reserved */ }
```

Components call `cx.require(&TABS)` while rendering; requirements resolve
dependencies first, each asset once, cycles terminating. Assets register themselves
with `register_asset!(TABS)` (via `inventory`) so `Bundle` includes every compiled-in
asset without manual lists. Unregistered assets work on full pages (scripts inlined)
but not in fragments: requiring one in a fragment panics in debug builds and is
omitted in release builds.

The `behavior` module defines the client contract: behaviour names, custom-element
tags (`st-tabs`), `data-*` names, header names and the theme storage key
(`stucco-theme`). A test generates the matching JS constants and fails on drift.

### 4.7 Bundle, Page and delivery

```rust
let bundle = Bundle::new(Preset::Slate)                          // infallible
    .with_theme("marketing", Theme::preset(Preset::Iris).radius(Radius::Round).build()?);
bundle.get(path) -> Option<AssetFile>   // bytes, MIME, ETag, immutable flag
```

`Bundle::new` and `with_theme` take `impl Into<BuiltTheme>`, implemented for
`Preset` (pre-validated in CI) and `BuiltTheme`. A `Theme` — including a modified
preset — must go through `Theme::build()`, the only place validation happens; there
is no path that skips it.

The bundle builds one stylesheet (layers, reset, theme tokens, named themes, base,
every registered asset's CSS), the client runtime, and one module per behaviour, all
content-hashed under a configurable prefix (default `/_stucco/`, must be same-origin
for fragments).

```rust
Page::new(&bundle, "Orders")
    .lang("en").meta(Meta::description("…"))
    .head(extra).body(app)
    .csp_nonce(nonce)
    .enhanced()            // include the runtime even if no behaviour is used yet
    .delivery(Delivery::Linked)
    .render()
```

- **Linked (default):** the site-wide stylesheet link (identical on every page);
  the runtime module if the page uses any behaviour or `.enhanced()` is set; one
  module per used behaviour. Fragments inserted later can load further modules
  (§4.8).
- **Inline:** the CSS chunks actually used and the used behaviours inline, for
  single-file documents, embeds and offline pages. Inline pages do not support
  dynamic fragment insertion; the runtime is not included.

The theme-flash prevention script is inlined in `<head>`; when a CSP nonce is set,
every `<script>` (inline or `src`), `<style>` and stylesheet `<link>` carries it, and
inline content has `</` written as `<\/` so it cannot close its element early.
Unregistered assets' CSS is inlined in a `<style>` on linked pages. Named theme names
match `[a-z0-9-]+`; font family names are CSS-escaped.

### 4.8 Client runtime and fragment protocol

The runtime is one small ES module (`stucco-runtime`, budget 6 KB gzip) providing:

1. **Requirement loading.** `<st-require modules="…">` (emitted at the start of
   fragment responses) imports each listed module once: a map from URL to the import
   promise de-duplicates concurrent fragments; the browser module map de-duplicates
   further. On failure it dispatches `stucco:asset-error`, sets `data-state="error"`
   on the enclosing enhanced region and shows that region's `RequestStatus` message
   with a reload link. Only same-origin hashed URLs from the bundle are accepted; no
   inline script is ever injected, so a strict CSP (`script-src 'self'` or
   nonce + `'strict-dynamic'`) holds.
2. **Self-enhancement.** Behaviours are custom elements; their
   `connectedCallback` runs when inserted by any mechanism, so swapped content
   enhances itself once its module has loaded (the element upgrades on definition).
3. **Enhanced requests.** Links and forms marked by an `Enhance` description (§6)
   are submitted with `fetch`, sending `Stucco-Request: fragment` and
   `Stucco-Target: <id>`. The response replaces (or appends to) the target.
   - Stale and out-of-order responses: one in-flight request per target; a new
     request aborts the previous (`AbortController`); responses for superseded
     requests are discarded.
   - Status handling: 200 and 422 and 409 swap; 204 with `Stucco-Location` navigates;
     anything else leaves content in place and shows the region's failure state.
   - Focus: if focus was inside the target and an element with the same id exists
     after the swap, it is focused; after a 422 the `ErrorSummary` is focused;
     otherwise focus moves to the target's heading if focus was lost.
   - Announcements: result counts and status changes go to a polite `LiveRegion`.
   - URL: GET enhancements update the address bar with `history.replaceState` when
     `push_url` is set, so state remains shareable.
4. **Invoker fallback.** If `command`/`commandfor` are unsupported, a polyfill
   behaviour is imported on demand.

**Icons** render as inline `<svg>` at each use (no sprite, no shared ids), so
fragments never depend on a page-level sprite; repetition compresses well. A sprite
optimisation may be added later without API change.

**htmx.** Supported and smoke-tested: content swapped in by htmx enhances itself and
its `<st-require>` loads modules, provided the page includes the runtime
(`.enhanced()`). stucco has no htmx-specific API and does not integrate with htmx
history or focus handling.

### 4.9 Template adapters

Foreign templates cannot thread `Cx` through, so adapters use an **ambient render
context**: while stucco renders, a thread-local points at the active `Cx` (rendering
is synchronous).

- `askama`: `Askama(template)` implements `Render`; inside it, stucco values embedded
  with `{{ value|safe }}` implement `Display` by rendering into the ambient `Cx`, so
  their asset requirements and ids are recorded. Template errors render an HTML
  comment in release builds and panic in debug builds.
- `maud`: `maud::Markup` implements `Render`; stucco values implement `maud::Render`
  through the same ambient context.
- Rendering a stucco value through `Display` with no ambient context falls back to a
  standalone render; if that value required any asset, debug builds panic with a
  message pointing to `Askama(..)`/`Page`, release builds drop the requirement.
- Each adapter ships runnable examples (doctests) proving a component embedded in a
  template contributes its behaviour module to the page.

## 5. Theme and CSS (`stucco-theme`)

### 5.1 Theme API

```rust
Theme::from_seed(250.0)
    .neutral_tint(0.01)
    .accent(Color::oklch(0.65, 0.18, 145.0))
    .fonts(Fonts::system().mono("JetBrains Mono"))
    .type_scale(TypeScale::new(16.0, 1.2))
    .space(4.0)
    .radius(Radius::Soft)
    .density(Density::Comfortable)
    .build()?          // Result<BuiltTheme, ContrastReport>
```

- 12-step OKLCH scales for neutral, accent, success, warning, danger, info, each
  generated for light and dark; out-of-gamut colours are chroma-reduced.
- Semantic roles: `--st-bg`, `--st-surface`, `--st-surface-raised`, `--st-hover`,
  `--st-text`, `--st-text-muted`, `--st-border`, `--st-border-strong`, `--st-accent`,
  `--st-accent-hover`, `--st-accent-text`, `--st-accent-soft`, `--st-on-accent`,
  `--st-focus`, and `--st-{success,warning,danger,info}` with `-text` and `-soft`.
  Diagram tones add their own roles in the diagram feature.
- Other tokens: type steps, spacing steps, radii, elevation, motion, z-index layers,
  control height (density).
- `build()` checks text/background role pairs against WCAG AA (4.5:1 text, 3:1 UI)
  in both schemes, including `on-accent` on `accent-hover` (the hover step moves away
  from the label colour), and rejects non-finite or non-positive options; the
  `ContrastReport` lists every failing pair and invalid option. `border-strong` and
  `focus` use step 9 so UI boundaries reach 3:1. `Theme::high_contrast()` steepens
  steps 9–11 for AAA.

### 5.2 Presets

`Theme::preset(p)` returns a customisable `Theme`; `Preset` itself converts to
`BuiltTheme` infallibly because every preset is validated in CI in both schemes.

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
| Contrast | pure black/white | strong blue | WCAG AAA text |

### 5.3 Light and dark

Roles are declared once with `light-dark()`. Without `data-theme` the OS decides;
`data-theme="light|dark"` on any element forces a scheme for its subtree; named
themes apply with `data-st-theme="<name>"`. `ThemeScope` (layout family) renders a
wrapper with those attributes.

### 5.4 CSS architecture

```css
@layer stucco.reset, stucco.tokens, stucco.base, stucco.layout,
       stucco.components, stucco.utilities;
```

Unlayered user CSS always wins. Classes use the `st-` prefix; variants and states are
data attributes shared with the behaviour contract. Components use container queries
and logical properties. `prefers-reduced-motion: reduce` disables non-essential
animation.

Enforced by tests: no colour literals outside generated tokens; component CSS uses
semantic roles only, never palette steps; every role defined for both schemes in
every preset; all component CSS inside a `stucco.*` layer; reduced motion honoured.

## 6. Backend-aware presentation

The library renders presentation state and describes endpoints; it never executes
them (execution: integration spec).

**Modes**, documented per component:

- **Server** — links, GET/POST forms, full-page responses. Always works.
- **Enhanced** — the same endpoints requested asynchronously with partial
  replacement (§4.8). Same operation semantics as Server.
- **Live** — polling or streaming updates over an initial server-rendered snapshot.
  **Later**; components designed for it render a usable snapshot plus a manual
  refresh link without it.

**Presentation types** (in `stucco-core`, no HTTP dependency):

- `Action { method: Method, href: Href, enhance: Option<Enhance> }` with
  `Method::{Get, Post}` (what HTML forms support) and
  `Enhance { target: String, swap: Swap::{Replace, Append}, push_url: bool }`.
- `FormState` — submitted values per field, field errors, form-level errors.
  Fields marked sensitive (`PasswordInput`, `.sensitive()`) never redisplay values.
  `Field::new("Email", Input::email("email")).bind(&state)` fills value, error and
  `aria-invalid`, keyed by the control's name (`Input::password` is sensitive by
  default). `FormState` is built with `with_value`/`with_error`/`with_form_error`.
- `CollectionQuery` — shareable query state for collections: sort, direction,
  search, filters per column, and a `Window`: `Offset { page, per_page }` or
  `After(Cursor)` / `Before(Cursor)` for key-ordered stores. `CollectionQuery::parse`
  (unknown or malformed parameters ignored) and `to_query_string` are the query codec;
  parameter names `sort`, `dir`, `q`, `f.<column>`, `page`, `per`, `after`, `before`.
- `CollectionPage<T> { rows, next: Option<Cursor>, prev: Option<Cursor>, total: Option<u64> }`
  and `Capabilities { sortable, filterable, searchable, total_count }`, so a
  collection renders only the controls its source supports. `Cursor` is an opaque,
  URL-safe string.
- `ViewState` for regions: `Ready`, `Loading`, `Empty`, `Failed { message, retry: Option<Action> }`,
  `Conflict { message }`; validation lives in `FormState`.

**Fallback rules.** Without JavaScript: infinite loading keeps a next-page link;
autosave keeps an explicit submit button; remote suggestions keep a native
`<datalist>` or a server search submit; live regions render a snapshot and a refresh
link.

## 7. Composition and catalogue rules

- Constituents that matter standalone are public (§4.5): `Feature`/`FeatureGrid`,
  `PricingTier`/`PricingTiers`, `Testimonial`/`TestimonialGroup`,
  `AccordionItem`/`Accordion`, `FaqItem`/`Faq`, `TimelineItem`/`Timeline`.
- Consolidations: `FileInput` (no separate `File`); `Accordion` and `Faq` are built
  on `Disclosure`; `Callout` and `Banner` are `Alert` variants
  (`Alert::callout`, `Alert::banner`); `ConfirmAction` is a button that opens a
  `ConfirmDialog` (one public type each, the dialog usable alone); `RemoteSearch` is
  `SearchForm` plus a results region with an `Enhance`; `TableRow`/`TableCell` are
  `Table::row` / `Row::cell` builder methods; `Tab`/`TabPanel` are `Tabs::tab`;
  `ListItem` is `List::item`; `MenuSeparator` is `DropdownMenu::separator`.
- Complete pages (collection, detail, edit, settings, dashboard, search, sign-in,
  file manager, job monitor, docs, landing, pricing, error) are gallery **recipes**,
  not public types.

## 8. Catalogue

Columns: **L** level (P primitive, C composition, S shell); **Step** roadmap step
(§14) where it lands, or **later**; **Fallback / behaviour** (— = static HTML/CSS,
**JS** = behaviour module); **Feature**.

### 8.1 Layout and foundations (`layout`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Stack, Cluster, Grid, Sidebar, Switcher, Center, Container | P | 2 | — |
| Cover, Frame, Reel | P | 7 | — |
| Separator, VisuallyHidden, SkipLink, Surface, ThemeScope | P | 2 | — |
| Spacer | P | 7 | — |
| Image (width/height required, lazy by default) | P | 7 | — |
| Icon (type always available; set via `icons`) | P | 2 | — |

### 8.2 Typography and content (`typography`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Heading, Text, Link, Code, Kbd | P | 2 | — |
| List (`List::item`), DescriptionList, Caption, Time | P | 7 | — |
| Blockquote, Prose (`markdown`: `Prose::markdown`) | P | 7 | — |
| CodeBlock | C | 7 | copy button **JS** (absent without JS) |

### 8.3 Actions (`actions`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Button, ButtonLink, IconButton (label required) | P | 2 | — |
| ButtonGroup, ActionGroup | C | 4 | — |
| ActionForm (one-button POST form) | C | 5 | — |
| CopyButton | P | 7 | **JS**; hidden without JS |
| DownloadLink | P | 7 | — |
| ConfirmAction | C | 7 | opens ConfirmDialog; without invokers/JS submits a confirm page via `fallback_href` |
| AsyncAction (ActionForm + RequestStatus + Enhance) | C | later | — |

### 8.4 Form primitives (`forms`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Form, Field, FieldHint, FieldError, Fieldset, Legend | P/C | 2 | — |
| Input, Textarea, Select, Checkbox, RadioGroup, HiddenInput, CsrfToken | P | 2 | — |
| ErrorSummary (links to fields; focused after 422) | C | 5 | — |
| CheckboxGroup, Switch, Range, FileInput | P | 7 | — |
| PasswordInput | P | 7 | reveal toggle **JS**; plain password field without |
| Combobox | C | 7 | **JS**; native `<input list>` / server search without |

### 8.5 Composed forms (`forms`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| FormSection, FormActions | C | 5 | — |
| ValidatedForm (Form + FormState + ErrorSummary) | C | 5 | — |
| SearchForm, FilterBar | C | 4 | GET form |
| SettingsSection | C | 7 | — |
| InlineEdit, UploadField, RepeatableFields, FormWizard, AutosaveForm | C | later | — |

### 8.6 Feedback and state (`feedback`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Alert (`callout`, `banner` variants), Badge, Tag, StatusDot | P | 4 | — |
| EmptyState, ErrorState, Spinner, Skeleton | P | 4 | — |
| RequestStatus (renders a `ViewState`), LiveRegion | C | 6 | — |
| FlashMessages (server flash → Alerts or ToastRegion) | C | 5 | inline Alerts |
| Progress, Meter | P | 7 | — |
| Toast, ToastRegion | C | 7 | **JS**; inline Alerts without |
| ConnectionStatus, JobProgress | C | later | — |

### 8.7 Navigation (`navigation`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Navbar (mobile menu via `<details>`), NavGroup, NavItem, SidebarNav | C | 4 | — |
| Breadcrumbs, Pagination (numbered or cursor) | C | 4 | — |
| Disclosure | P | 7 | — |
| Tabs (`Tabs::tab`) | C | 7 | **JS**; anchor list with all panels visible |
| Steps, TableOfContents | C | 7 | — |
| AccountMenu, WorkspaceSwitcher | C | 7 | DropdownMenu |
| SearchNavigation | C | later | — |

### 8.8 Overlays (`overlay`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Dialog, Drawer | C | 7 | native `<dialog>` + invokers; polyfill **JS** if unsupported; `fallback_href` to a server-rendered page |
| ConfirmDialog, FormDialog | C | 7 | as Dialog |
| Popover | P | 7 | native popover API |
| Tooltip | P | 7 | CSS anchor positioning; `@supports` fallback places it below |
| DropdownMenu, MenuItem (`DropdownMenu::separator`) | C | 7 | **JS** keyboard navigation; popover with plain links/buttons without |
| RemoteDialog (Dialog loading a fragment) | C | later | link to the page |

### 8.9 Data display (`data`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| Card, Panel, Table (`Table::row`, `Row::cell`), ResultCount | P/C | 4 | — |
| Stat, MetricGrid | C | 7 | — |
| Avatar, AvatarGroup | P | 7 | — |
| Timeline, TimelineItem, Accordion, AccordionItem | C | 7 | `<details>` |
| ResourceList, ResourceListItem, DetailPanel | C | 7 | — |
| SortControl, PageSizeSelect | P | 4 | links / GET select |
| ActivityFeed, ActivityItem, FileList, FileItem | C | later | — |

### 8.10 Backend-aware collections (`collections`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| CollectionToolbar (SearchForm + FilterBar + SortControl + ResultCount) | C | 4 | GET form |
| DataTable (Table + CollectionToolbar + EmptyState + Pagination + LiveRegion) | C | 4 server, 6 enhanced | **JS** client mode for fully loaded rows |
| DataList (ResourceList with the same query model) | C | 7 | as DataTable |
| LoadMoreList | C | 7 | next-page link; Enhance with `Swap::Append` |
| BulkActionBar | C | 7 | checkboxes + POST form |
| DependentFields | C | later | full-page reload on change |
| RemoteSearch, RemoteSelect | C | later | SearchForm / native select |
| InfiniteList, NotificationCenter, JobList | C | later | next-page link / snapshot |

DataTable columns are typed (`Col::text`, `Col::number`, `Col::date`,
`Col::enumeration`, `Col::custom`); the column kind sets sort order and filter
control. Controls appear only where `Capabilities` allow. Server mode works on any
data size without JS; **client mode** (`.client_side()`, **JS**) sorts, filters and
searches rows already on the page, announces counts and keeps URL parameters in sync.

### 8.11 Application composition (`app`)

| Component | L | Step | Fallback / behaviour |
|---|---|---|---|
| AppShell, PageHeader, SectionHeader, Footer | S/C | 4 | — |
| ThemeSwitcher | C | 7 | **JS**; hidden without JS (OS preference applies) |
| DocsShell, MarketingShell | S | 9 | — |
| SplitPane, MasterDetail | C | later | stacked layout |

### 8.12 Marketing (`marketing`, step 9)

Section, Hero, Feature, FeatureGrid, CtaBand, ComparisonTable, PricingTier,
PricingTiers, Testimonial, TestimonialGroup, LogoCloud, Faq, FaqItem — all static.
NewsletterForm, ContactForm — ValidatedForm recipes with public builders.

### 8.13 Diagrams (`diagram`, step 9) — see §9.

## 9. Diagrams (`stucco-diagram`)

```rust
pub trait NodeKind {
    fn label(&self) -> &str;
    fn icon(&self) -> Option<Icon>;
    fn tone(&self) -> Tone;
}
Node::new(kinds::DATABASE).label("Orders").sub("db.prod:5432").status(Status::Healthy)
Node::new(Kind::new("Ledger").tone(Tone::Structure))          // iconless: text-only node
```

- Starter kinds (service, database, queue, user, client, api, cloud, server,
  function, storage, external, gateway, process, decision) use Lucide icons; this is
  why `diagram` requires `icons`. Custom kinds may have no icon.
- Tones: Primary, Structure, Edge, Neutral, Success, Danger, Info, Warning.
- Connections: Data, Control, Active, Potential, Denied, each with a label and a
  visually hidden meaning.
- Containers: Flow, Branch, Layers, Sequence, Compare, Boundary, Topology; each
  requires a caption (`DiagramCaption`), and `DiagramLegend` explains tones and
  connection kinds.
- Width safety in types: containers implement `Block`, nodes `Inline`; content-sized
  slots accept only `Inline`.
- Motion: `Reveal` (static order under reduced motion) and **JS** `Walkthrough`
  (fallback: all stages listed).
- **Later:** DiagramInspector, LiveDiagram, automatic layout.

Rendering is HTML/CSS, so diagrams reflow, follow the theme and remain readable by
screen readers.

## 10. Behaviours

Each behaviour is a dependency-free ES module defining one light-DOM custom element
wrapping native markup, configured only by `data-*` attributes from
`core::behavior`. Behaviours in v1: runtime (§4.8), invoker polyfill, theme-switcher,
tabs, dropdown-menu, combobox, password-reveal, toast, copy, data-table (client
mode), walkthrough. Budget: 4 KB gzip per behaviour, checked in CI.

## 11. Browser support

Supported: the current and previous major versions of Chrome, Edge, Firefox and
Safari (desktop and iOS), tested with Playwright's Chromium, Firefox and WebKit.

| Platform feature | Use | Support at time of writing | Fallback |
|---|---|---|---|
| `light-dark()`, container queries, `:has()`, `<dialog>`, popover API | everywhere | Baseline widely available | none needed |
| Invoker commands (`command`/`commandfor`) | Dialog, Drawer, Popover triggers | Chrome 135, Firefox 144, Safari 26.2 | polyfill behaviour; `fallback_href` without JS |
| CSS anchor positioning | Tooltip, DropdownMenu placement | Chrome 125, Firefox 147, Safari 26 | `@supports not (anchor-name: --a)` places below the trigger |

Versions are re-verified when each feature's step begins; the Playwright suite runs
each feature's fallback path with the native feature disabled where Playwright allows,
and with JavaScript disabled.

## 12. Error handling

- Rendering cannot fail; invalid input degrades safely (§4.4), unknown query
  parameters are ignored.
- `Theme::build()` is the one fallible public API (`ContrastReport`).
- `Bundle::get` returns `None` for unknown paths; the integration maps it to 404.
- Client asset load failures are visible (§4.8.1).

## 13. Quality

- Rust unit tests per component: markup contract, escaping, attribute refusal, aria
  wiring, id policy, reserved attributes, declared assets.
- `insta` snapshots of every gallery example.
- CSS rule tests and contrast tests for every preset in both schemes.
- Behaviour contract test (Rust constants ↔ JS).
- Feature matrix in CI: no default features, each feature alone, the defaults, the
  integration-slice combination, and all features (`cargo hack` with
  `--each-feature` plus the named combinations).
- Playwright (Chromium, Firefox, WebKit) over the gallery and the slice examples:
  keyboard interaction for every behaviour, axe-core in both schemes, a
  JavaScript-disabled pass, fragment insertion introducing a new behaviour and icon,
  concurrent fragments requesting the same module, stale-response discard, focus
  restoration, and an htmx smoke test.
- Docs: `#![deny(missing_docs)]`, runnable examples on every public item, the gallery
  (components, recipes, palette page, theme switcher), and `examples/`.
- Tooling: edition 2024, MSRV 1.85, clippy `-D warnings`, rustfmt, `cargo-deny`,
  `cargo-semver-checks` before releases, GitHub Actions; MIT OR Apache-2.0; local git
  only until the user asks otherwise.

## 14. Roadmap

Each step ships a working example, a no-JavaScript path, keyboard and accessibility
checks, and feature-build checks.

1. Core: rendering, safe markup and attribute policy, assets, fragments
   (`RenderedFragment`, `<st-require>`), identity policy, Bundle, Page, runtime
   requirement loading.
2. Theme generation, presets and contrast validation; base CSS; minimal layout,
   typography, actions and form primitives (step-2 rows in §8).
3. Thin Tower integration serving Bundle assets and full pages (integration spec).
4. AppShell + PageHeader + server-mode DataTable with GET sort/filter/search and
   numbered **and** cursor pagination, backed by a k/v store example.
5. Edit form: submitted-value redisplay, server validation, ErrorSummary, CSRF,
   authorization, conflict on concurrent edit.
6. Enhanced mode on the same endpoints; a fragment inserts a component whose
   behaviour and icon were absent from the initial page.
7. Catalogue expansion (step-7 rows) and gallery coverage, in family order.
8. Uploads, job observation and one live-update path — **only after a separate
   framework scope review**.
9. Marketing, diagrams, DocsShell/MarketingShell, adapters, documentation and the
   v1 release.

Committed v1 = steps 1–7 and 9. Step 8 and every **later** row are outside v1.

## 15. Decisions for review

1. Crate name `stucco-tower` for the integration, kept out of the `stucco` facade
   (no `server` feature). Framework facade decided in the framework spec.
2. Icons inline per use (no sprite) — simplest fragment story; costs repeated bytes
   before compression.
3. Fragment namespaces required and caller-chosen (target id or record key) as the
   id-collision policy.
4. `Slot<'a>` and lifetime-carrying components to allow borrowed data.
5. Ambient thread-local render context for askama/maud embedding.
6. Reserved attributes: debug panic, release ignore.
7. Browser matrix: current + previous major of Chrome, Edge, Firefox, Safari.
8. Inline delivery excludes dynamic fragments; email is out of scope.
9. Live mode, uploads and jobs deferred to step 8 behind a scope review.
