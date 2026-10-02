# stucco First Components Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build roadmap step 2's components — layout primitives, the `Icon` type and Lucide set, typography, actions and form primitives — in a new `stucco-ui` crate, wired to facade features, with gallery pages and browser checks.

**Architecture:** `stucco-ui` depends only on `stucco-core`. Every component is a struct with a builder, an `Attrs` passthrough with documented reserved attributes, and a registered `Asset` holding its CSS. Components take parameters as data attributes and enum values — never inline `style` attributes — so pages stay valid under a strict CSP. `FormState` (presentation state) lives in `stucco-core`; `Field` binds controls to it. Facade features forward to `stucco-ui` features.

**Tech Stack:** Rust edition 2024 (MSRV 1.85), `stucco-core`, `inventory`, `insta` (dev); Node 24 + `lucide-static` 1.49.0 (code generation only); Playwright + axe (dev); `cargo-hack` 0.6 (installed locally; CI installs it).

**Spec:** `docs/superpowers/specs/2026-10-01-stucco-design.md` §3 (features), §4.4–4.5 (attribute policy, component conventions), §6 (`FormState`), §8.1–8.4 step-2 rows, §13 (feature matrix). Previous plan: `2026-10-01-stucco-foundations.md` (merged).

## Global Constraints

- Edition 2024, `rust-version = "1.85"`; no let-chains or other post-1.85 syntax.
- `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`; every public component has a rustdoc example that compiles.
- `stucco-ui` depends only on `stucco-core` (+ `inventory` via its macro). No HTTP/async/I-O crates.
- Component CSS: one file per component family under `crates/stucco-ui/css/`, inside `@layer stucco.layout` (layout family) or `@layer stucco.components` (everything else), passing `check_component_css`. Class prefix `st-`.
- No component emits a `style` attribute. Parameters are `data-*` attributes with a closed set of values.
- Every component: `new(required…)`, chained setters taking `impl Into<…>`, `Debug`, passthrough `.class() .id() .attr() .data() .aria()`, and a `RESERVED: &[&str]` list (passthrough of a reserved name: debug panic `"reserved attribute"`, release ignored).
- Generated ids use prefixes matching `[a-z0-9]+`.
- Lucide icon data is generated, committed, ISC-attributed (`crates/stucco-ui/LICENSE-LUCIDE`).
- Commits: plain messages, no AI attribution lines.
- Gate after every task: `cargo +stable fmt --all --check`, `cargo +stable clippy --workspace --all-targets -- -D warnings`, `cargo +stable test --workspace`; from Task 2 on also `cargo +stable hack check -p stucco --each-feature --no-dev-deps`; plus `cargo +stable test --workspace --release` at Tasks 1, 9 and 11.

## Review Focus

1. **Sensitive values** — a password `Input` bound to a `FormState` holding a submitted password never renders a `value` attribute; `.sensitive()` does the same for any `Input`/`Textarea`. Test in Task 9.
2. **Field id wiring** — two `Field`s without ids on one page get distinct ids; a control with an explicit id gives `{id}-hint` / `{id}-error`; `aria-describedby` lists hint then error; `aria-invalid="true"` only with errors. Tests in Task 9.
3. **Reserved passthrough** — `Field … .attr("aria-describedby", "x")` panics in debug and is ignored in release; `Button::new("x").attr("type", "submit")` likewise (type is set by `.submit()`). Tests in Tasks 2 and 7.
4. **User text everywhere is escaped** — labels, hints, errors, options, placeholder and legend text containing `<`, `"` and `&`. Each component test file includes one escaping case.
5. **Minimal feature builds** — `stucco` with `--no-default-features` and with each feature alone compiles and tests pass; no text-only component needs `icons`. Tests: `cargo hack` in Task 2 and Task 11, locally and in CI.

---

### Task 1: Core prerequisites

Folds in two deferred review minors that these components depend on.

**Files:**
- Modify: `crates/stucco-core/src/attrs.rs`, `crates/stucco-core/src/el.rs`, `crates/stucco-core/src/render.rs`, `crates/stucco-core/src/behavior.rs`, `crates/stucco-core/src/lib.rs`
- Create: `crates/stucco-core/src/form_state.rs`
- Modify: `docs/superpowers/specs/2026-10-01-stucco-design.md` §4.4 (raw-text rule), §4.3 (generated vs explicit ids note)

**Interfaces:**
- Produces:
  - Attribute names are lowercased on insert everywhere in `Attrs` (`attr`, `trusted_attr`, `bool_attr`, `reserved_conflicts` compares lowercased).
  - `Attrs::without(&self, names: &[&str]) -> Attrs` (a copy with those names, and `id`/`class` if listed, removed).
  - `el::script()` / `el::style()`: `.text()` writes the text **unescaped except** that `</` becomes `<\/` (via a crate-private `RawText` slot). Other elements unchanged.
  - Doc on `Cx::id`: generated ids can equal an explicit id such as `tabs-1`; keep explicit ids distinct from `{prefix}-{n}`.
  - `behavior::CSRF_FIELD: &str = "_csrf"` (+ placeholder entry).
  - `form_state.rs`: `#[derive(Clone, Debug, Default, PartialEq, Eq)] pub struct FormState`; `FormState::new()`; builders `.value(name, v)` (appends), `.error(name, msg)`, `.form_error(msg)`; getters `value(&self, name) -> Option<&str>` (first), `values(&self, name) -> &[String]`, `errors(&self, name) -> &[String]`, `form_errors() -> &[String]`, `has_errors() -> bool`. Backed by `BTreeMap<String, Vec<String>>`.

- [ ] **Step 1: Write the failing tests** (in the existing test modules / new `form_state.rs`):

```rust
// el/tests.rs
#[test]
fn attribute_names_are_case_insensitive() {
    assert_eq!(to_html(&el::div().attr("title", "a").attr("TITLE", "b")), r#"<div title="b"></div>"#);
    assert_eq!(to_html(&el::div().bool_attr("hidden", true).bool_attr("Hidden", false)), "<div></div>");
    assert_eq!(Attrs::default().attr("ROLE", "x").reserved_conflicts(&["role"]), ["role"]);
}

#[test]
fn script_and_style_text_is_raw_but_cannot_close_the_element() {
    assert_eq!(to_html(&el::style().text("ul > li {}")), "<style>ul > li {}</style>");
    assert_eq!(to_html(&el::script().text("a < b && c</script>")), r"<script>a < b && c<\/script></script>");
}

#[test]
fn without_removes_names_including_id_and_class() {
    let a = Attrs::default().id("x").class("c").attr("role", "r").attr("title", "t");
    assert_eq!(to_html(&el::div().attrs(&a.without(&["id", "role"]))), r#"<div class="c" title="t"></div>"#);
}

// form_state.rs
#[test]
fn form_state_collects_values_and_errors() {
    let s = FormState::new().value("tags", "a").value("tags", "b").error("email", "Required").form_error("Try again");
    assert_eq!(s.value("tags"), Some("a"));
    assert_eq!(s.values("tags"), ["a", "b"]);
    assert_eq!(s.errors("email"), ["Required"]);
    assert!(s.errors("name").is_empty() && s.has_errors());
    assert_eq!(s.form_errors(), ["Try again"]);
}
```

Also update the existing `raw_text_elements_stay_escaped` test to the new expectation (it encoded the old rule).

- [ ] **Step 2: Run** `cargo +stable test -p stucco-core` — expected FAIL (new tests).
- [ ] **Step 3: Implement**; update spec §4.4: "Text is escaped, except inside `<script>`/`<style>` where it is written raw with `</` neutralised as `<\/`; content there is code, so untrusted text must not be placed in a script."
- [ ] **Step 4: Run the gate (debug and release)** — expected PASS. Commit `"Lowercase attribute names, raw-text script/style, FormState"`.

---

### Task 2: `stucco-ui` crate, shared vocabulary and feature wiring

**Files:**
- Create: `crates/stucco-ui/{Cargo.toml, src/lib.rs, src/common.rs, src/passthrough.rs}`, `crates/stucco-ui/tests/css.rs`
- Modify: `Cargo.toml` (workspace dep), `crates/stucco/{Cargo.toml, src/lib.rs}`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces:
  - `stucco-ui` features: `layout`, `typography`, `actions`, `forms = ["actions"]`, `icons`; default none. Facade features forward (`layout = ["stucco-ui/layout"]`, …; facade `forms = ["actions", "stucco-ui/forms"]`); facade re-exports `stucco_ui::*` at the root and `stucco::icon` (feature `icons`).
  - `common.rs` (always compiled), each with `fn as_str(self) -> &'static str` used as the data-attribute value:
    - `enum Space { S0, S1, S2, S3, S4, S5, S6, S8, S10, S12 }` → `"0".."12"`.
    - `enum Size { Xs, Sm, Md, Lg, Xl, Xl2, Xl3 }` → `"xs" "sm" "md" "lg" "xl" "2xl" "3xl"`.
    - `enum Measure { Xs, Sm, Md, Lg, Xl, Prose }` → `"xs".."xl" "prose"` (20, 30, 40, 60, 75rem, 65ch).
    - `enum Variant { Primary, Secondary, Ghost, Danger }`; `enum Tone { Default, Muted, Accent, Success, Warning, Danger, Info }`; `enum ColorScheme { Light, Dark }`.
  - `passthrough.rs`: `macro_rules! passthrough` generating `.class .id .attr .data .aria` on a struct with an `attrs: Attrs` field; `pub(crate) fn apply(el: Element<'a>, attrs: &Attrs, reserved: &[&str]) -> Element<'a>` — `debug_assert!(conflicts.is_empty(), "reserved attribute: {…}")`, merges `attrs.without(reserved)`.
  - `pub fn ui_assets() -> Vec<&'static Asset>` (every asset the enabled features register; used by the CSS test).
  - CI: job `features` installing cargo-hack (`taiki-e/install-action@cargo-hack`) and running `cargo hack check -p stucco --each-feature --no-dev-deps` and `cargo hack test -p stucco-ui --each-feature`.

- [ ] **Step 1: Write the failing tests:**

```rust
// tests/css.rs
#[test]
fn every_ui_stylesheet_obeys_the_component_rules() {
    for asset in stucco_ui::ui_assets() {
        stucco_core::check_component_css(asset.name, asset.css.unwrap_or("")).unwrap();
    }
}

// passthrough.rs (unit)
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved attribute")]
fn reserved_passthrough_panics_in_debug() {
    let _ = apply(el::div(), &Attrs::default().attr("role", "x"), &["role"]);
}

#[test]
#[cfg(not(debug_assertions))]
fn reserved_passthrough_is_ignored_in_release() {
    let html = to_html(&apply(el::div().attr("role", "list"), &Attrs::default().attr("role", "x").class("c"), &["role"]));
    assert_eq!(html, r#"<div class="c" role="list"></div>"#);
}
```

- [ ] **Step 2: Run** `cargo +stable test -p stucco-ui` — expected FAIL.
- [ ] **Step 3: Implement.** (`ui_assets()` starts empty; later tasks append.)
- [ ] **Step 4: Run the gate** and `cargo +stable hack check -p stucco --each-feature --no-dev-deps` plus `cargo +stable hack test -p stucco-ui --each-feature` — expected PASS. Commit `"Add stucco-ui crate with shared vocabulary and feature wiring"`.

---

### Task 3: Icon type and the Lucide set

**Files:**
- Create: `crates/stucco-ui/src/icon.rs`, `crates/stucco-ui/src/icon/lucide.rs` (generated), `crates/stucco-ui/LICENSE-LUCIDE`, `tools/package.json`, `tools/gen-icons.mjs`

**Interfaces:**
- Produces:
  - Always compiled: `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Icon { name: &'static str, body: &'static str }`; `pub const fn Icon::custom(name: &'static str, body: &'static str) -> Icon` (body = trusted inner SVG markup for a 24×24 viewBox; doc says so); `icon.name()`; `icon.label(text) -> LabelledIcon`. `impl Render for Icon` → `<svg class="st-icon" viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">{body}</svg>`; `LabelledIcon` renders the same with `role="img" aria-label="…"` instead of `aria-hidden`.
  - Feature `icons`: `pub mod icon` (in `lucide.rs`) with one `pub const NAME: Icon` per Lucide icon (kebab → SCREAMING_SNAKE; `Icon::custom("arrow-right", "<path …/>")`) and `pub const ALL: &[Icon]`.
  - Generator: `node tools/gen-icons.mjs` reads `tools/node_modules/lucide-static/icons/*.svg`, strips the outer `<svg …>`/`</svg>` and comments, rejects any body containing `<script`, `on…=` or `href`, sorts by name, writes `lucide.rs` with a header naming the package version.
  - Icon CSS (`st-icon`: inline-block, `vertical-align: -0.125em`, `flex-shrink: 0`) as a registered asset required by both renders.

- [ ] **Step 1: Write the failing tests** (`icon.rs`):

```rust
#[test]
fn decorative_icons_are_hidden_from_assistive_technology() {
    let html = to_html(&Icon::custom("dot", "<circle cx=\"12\" cy=\"12\" r=\"2\"/>"));
    assert!(html.starts_with("<svg class=\"st-icon\" viewBox=\"0 0 24 24\""));
    assert!(html.contains("aria-hidden=\"true\"") && html.ends_with("<circle cx=\"12\" cy=\"12\" r=\"2\"/></svg>"));
}

#[test]
fn labelled_icons_are_images_with_escaped_labels() {
    let html = to_html(&Icon::custom("dot", "").label("Done & <ok>"));
    assert!(html.contains("role=\"img\" aria-label=\"Done &amp; &lt;ok&gt;\"") && !html.contains("aria-hidden"));
}

#[cfg(feature = "icons")]
#[test]
fn the_lucide_set_is_complete_and_safe() {
    assert!(icon::ALL.len() > 1500);
    assert_eq!(icon::ARROW_RIGHT.name(), "arrow-right");
    assert!(icon::ALL.iter().all(|i| !i.body().contains("<script") && !i.body().contains(" on")));
}
```

- [ ] **Step 2: Run** `cargo +stable test -p stucco-ui --features icons icon` — expected FAIL.
- [ ] **Step 3: Implement**; run `cd tools && npm install && node gen-icons.mjs`; copy Lucide's ISC licence to `LICENSE-LUCIDE`; mention it in `README.md`.
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add Icon and the generated Lucide set"`.

---

### Task 4: Layout primitives I — Stack, Cluster, Grid, Center, Container

**Files:**
- Create: `crates/stucco-ui/src/layout/{mod.rs, stack.rs, cluster.rs, grid.rs, center.rs, container.rs}`, `crates/stucco-ui/css/layout.css`

**Interfaces:**
- Produces (feature `layout`; all accept children via `.child(impl Render + 'a)` / `.children(iter)`; all `Element` tag `div` unless `.as_tag(Tag)` with `enum Tag { Div, Section, Ul, Ol, Nav, Header, Footer, Article, Aside }`):
  - `Stack::new()` `.space(Space)` (default `S4`) → `<div class="st-stack" data-space="4">`; `.recursive()` → `data-recursive`.
  - `Cluster::new()` `.space(Space)` (default `S3`), `.justify(Justify)` `enum Justify { Start, Center, End, Between }`, `.align(Align)` `enum Align { Start, Center, End, Baseline, Stretch }` → `data-justify`, `data-align`.
  - `Grid::new()` `.min(Measure)` (default `Sm`), `.space(Space)` → auto-fit columns `minmax(min(100%, <measure>), 1fr)`.
  - `Center::new()` `.max(Measure)` (default `Prose`), `.gutters(Space)`, `.intrinsic()`.
  - `Container::new()` `.size(Measure)` (default `Xl`) — centred page width with responsive gutters.
- CSS: every data-attribute value has a rule (e.g. `.st-stack[data-space="4"] > * + * { margin-block-start: var(--st-space-4) }`); no `style` attributes.

- [ ] **Step 1: Write the failing tests** (`layout/mod.rs` tests module):

```rust
#[test]
fn stack_spaces_children_with_data_attributes() {
    let html = to_html(&Stack::new().space(Space::S6).child("a").child("<b>"));
    assert_eq!(html, r#"<div class="st-stack" data-space="6">a&lt;b&gt;</div>"#);
}

#[test]
fn layout_parameters_never_use_inline_styles() {
    let html = to_html(&(
        Cluster::new().justify(Justify::Between).align(Align::Center),
        Grid::new().min(Measure::Md).space(Space::S2),
        Center::new().max(Measure::Lg).intrinsic(),
        Container::new().size(Measure::Xl),
    ));
    assert!(!html.contains("style="), "{html}");
    for needle in [r#"data-justify="between""#, r#"data-min="md""#, r#"data-max="lg""#, "data-intrinsic", r#"data-size="xl""#] {
        assert!(html.contains(needle), "{needle}");
    }
}

#[test]
fn as_tag_changes_the_element_and_passthrough_merges() {
    let html = to_html(&Stack::new().as_tag(Tag::Ul).class("x").aria("label", "Items"));
    assert_eq!(html, r#"<ul class="st-stack x" data-space="4" aria-label="Items"></ul>"#);
}

#[test]
fn layout_stylesheet_covers_every_value() {
    let css = LAYOUT.css.unwrap();
    for s in ["0", "1", "2", "3", "4", "5", "6", "8", "10", "12"] {
        assert!(css.contains(&format!(".st-stack[data-space=\"{s}\"]")), "{s}");
    }
}
```

- [ ] **Step 2: Run** `cargo +stable test -p stucco-ui --features layout layout` — expected FAIL.
- [ ] **Step 3: Implement** (one shared `LAYOUT` asset for the family; each primitive requires it).
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add Stack, Cluster, Grid, Center and Container"`.

---

### Task 5: Layout primitives II — Sidebar, Switcher, Separator, VisuallyHidden, SkipLink, Surface, ThemeScope

**Files:**
- Create: `crates/stucco-ui/src/layout/{sidebar.rs, switcher.rs, separator.rs, visually_hidden.rs, skip_link.rs, surface.rs, theme_scope.rs}`; Modify: `css/layout.css`

**Interfaces:**
- Produces (feature `layout`):
  - `Sidebar::new(side: impl Render + 'a, main: impl Render + 'a)` `.side_width(Measure)` (default `Xs`), `.right()`, `.space(Space)`; collapses to stacked when main would be narrower than 50%.
  - `Switcher::new()` `.threshold(Measure)` (default `Lg`), `.space(Space)`, `.limit(u8)` (2–6, else clamped) → `data-limit`.
  - `Separator::new()` → `<hr class="st-separator">`; `.decorative()` → `<div class="st-separator" role="none">`.
  - `VisuallyHidden::new(child)` → `<span class="st-sr-only">`.
  - `SkipLink::new()` (target `#main`, text "Skip to main content") `.target(&str)` `.text(&str)` → `<a class="st-skip-link" href="#main">`, visible on focus.
  - `Surface::new()` `.level(Level)` `enum Level { Flat, Raised, Overlay }`, `.padding(Space)`, `.border(bool)` (default true), `.radius(Size)`.
  - `ThemeScope::new()` `.scheme(ColorScheme)` with `enum ColorScheme { Light, Dark }` (in `common.rs`, so `stucco-ui` needs no theme dependency) → `data-theme`; `.named(&'static str)` → `data-st-theme` (name validated `[a-z0-9-]+`, debug panic `"invalid theme name"`); renders a `div` with children and `color: var(--st-text); background: var(--st-bg)`.
- `RESERVED`: `ThemeScope` reserves `data-theme`, `data-st-theme`; `SkipLink` reserves `href`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn sidebar_orders_side_and_main() {
    let html = to_html(&Sidebar::new("nav", "content").right());
    assert_eq!(html, r#"<div class="st-sidebar" data-side="right" data-side-width="xs" data-space="4"><div>nav</div><div>content</div></div>"#);
}

#[test]
fn separators_have_the_right_semantics() {
    assert_eq!(to_html(&Separator::new()), r#"<hr class="st-separator">"#);
    assert_eq!(to_html(&Separator::new().decorative()), r#"<div class="st-separator" role="none"></div>"#);
}

#[test]
fn skip_link_targets_main_by_default() {
    assert_eq!(to_html(&SkipLink::new()), r##"<a class="st-skip-link" href="#main">Skip to main content</a>"##);
}

#[test]
fn theme_scope_sets_scheme_and_named_theme() {
    let html = to_html(&ThemeScope::new().scheme(ColorScheme::Dark).named("brand").child("x"));
    assert_eq!(html, r#"<div class="st-theme-scope" data-theme="dark" data-st-theme="brand">x</div>"#);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "invalid theme name")]
fn theme_scope_validates_names() { let _ = ThemeScope::new().named("Bad Name"); }

#[test]
fn switcher_limit_is_clamped() {
    assert!(to_html(&Switcher::new().limit(9)).contains(r#"data-limit="6""#));
}
```

- [ ] **Step 2: Run** — expected FAIL. **Step 3: Implement.** **Step 4: Run the gate** — expected PASS. Commit `"Add Sidebar, Switcher, Separator, VisuallyHidden, SkipLink, Surface and ThemeScope"`.

---

### Task 6: Typography — Heading, Text, Link, Code, Kbd

**Files:**
- Create: `crates/stucco-ui/src/typography/{mod.rs, heading.rs, text.rs, link.rs, code.rs}`, `crates/stucco-ui/css/typography.css`

**Interfaces:**
- Produces (feature `typography`):
  - `Heading::new(level: u8, content: impl Render + 'a)` (level clamped 1–6; debug panic `"heading level"` outside), `.size(Size)` (default per level: 1→`Xl3`, 2→`Xl2`, 3→`Xl`, 4→`Lg`, 5→`Md`, 6→`Sm`) → `<h2 class="st-heading" data-size="2xl">`.
  - `Text::new(content)` `.size(Size)` (default `Md`), `.tone(Tone)` (default `Default`, omitted), `.weight(Weight)` `enum Weight { Normal, Medium, Bold }`, `.inline()` (`span` instead of `p`).
  - `Link::new(content, href: impl Into<Href>)` `.external()` → `target="_blank" rel="noopener noreferrer"` plus a visually hidden " (opens in a new tab)".
  - `Code::new(text)` → `<code class="st-code">`; `Kbd::new(keys: &[&str])` → `<kbd class="st-kbd"><kbd>Ctrl</kbd>+<kbd>K</kbd></kbd>`.
- `RESERVED`: `Link` reserves `href`, `target`, `rel`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn heading_level_and_size_are_independent() {
    assert_eq!(to_html(&Heading::new(2, "Orders").size(Size::Xl)), r#"<h2 class="st-heading" data-size="xl">Orders</h2>"#);
    assert_eq!(to_html(&Heading::new(1, "A & B")), r#"<h1 class="st-heading" data-size="3xl">A &amp; B</h1>"#);
}

#[test]
fn text_renders_tone_and_inline() {
    assert_eq!(to_html(&Text::new("x").tone(Tone::Muted).inline()), r#"<span class="st-text" data-size="md" data-tone="muted">x</span>"#);
}

#[test]
fn external_links_are_safe_and_announced() {
    let html = to_html(&Link::new("Docs", "https://e.com").external());
    assert!(html.contains(r#"target="_blank" rel="noopener noreferrer""#));
    assert!(html.contains(r#"<span class="st-sr-only"> (opens in a new tab)</span>"#));
    assert!(to_html(&Link::new("x", "javascript:alert(1)")).contains(r##"href="#""##));
}

#[test]
fn kbd_joins_keys() {
    assert_eq!(to_html(&Kbd::new(&["Ctrl", "K"])), r#"<kbd class="st-kbd"><kbd>Ctrl</kbd>+<kbd>K</kbd></kbd>"#);
}
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS). Commit `"Add Heading, Text, Link, Code and Kbd"`.

---

### Task 7: Actions — Button, ButtonLink, IconButton

**Files:**
- Create: `crates/stucco-ui/src/actions/{mod.rs, button.rs}`, `crates/stucco-ui/css/actions.css`

**Interfaces:**
- Produces (feature `actions`):
  - `Button::new(label: impl Render + 'a)` `.variant(Variant)` (default `Secondary`), `.size(Size)` (`Sm`/`Md`/`Lg`; default `Md`), `.submit()` / `.reset()` (default `type="button"`), `.name(&str)`, `.value(&str)`, `.icon(Icon)` (leading, decorative), `.disabled()`, `.loading()` (`aria-busy="true"`, `aria-disabled="true"`, keeps focusability; spinner via CSS) .
  - `ButtonLink::new(label, href: impl Into<Href>)` — `<a class="st-button">` with the same variant/size/icon options.
  - `IconButton::new(icon: Icon, label: &str)` — `<button class="st-button" data-icon-only aria-label="…">` with the icon; label required (empty label: debug panic `"icon button label"`).
  - Height uses `--st-control-h`; variants use roles only (`--st-accent`, `--st-on-accent`, `--st-accent-hover`, `--st-danger`, …); focus ring from base CSS.
- `RESERVED`: `Button`/`IconButton` reserve `type`, `aria-busy`, `aria-disabled`, `aria-label` (IconButton); `ButtonLink` reserves `href`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn buttons_default_to_type_button() {
    assert_eq!(to_html(&Button::new("Save")),
        r#"<button class="st-button" type="button" data-variant="secondary" data-size="md">Save</button>"#);
    assert!(to_html(&Button::new("Go").submit().variant(Variant::Primary)).contains(r#"type="submit" data-variant="primary""#));
}

#[test]
fn loading_buttons_stay_focusable_and_announce_busy() {
    let html = to_html(&Button::new("Save").loading());
    assert!(html.contains(r#"aria-busy="true" aria-disabled="true""#) && !html.contains(" disabled"));
}

#[test]
fn icon_buttons_have_accessible_names() {
    let html = to_html(&IconButton::new(Icon::custom("x", ""), "Close <dialog>"));
    assert!(html.contains(r#"aria-label="Close &lt;dialog&gt;""#) && html.contains("aria-hidden=\"true\""));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved attribute")]
fn type_cannot_be_overridden_by_passthrough() { let _ = to_html(&Button::new("x").attr("type", "submit")); }
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS). Commit `"Add Button, ButtonLink and IconButton"`.

---

### Task 8: Form structure — Form, Fieldset, Legend, FieldHint, FieldError, HiddenInput, CsrfToken

**Files:**
- Create: `crates/stucco-ui/src/forms/{mod.rs, form.rs, fieldset.rs, messages.rs, hidden.rs}`, `crates/stucco-ui/css/forms.css`

**Interfaces:**
- Produces (feature `forms`):
  - `Form::post(action: impl Into<Href>)` / `Form::get(action)`; `.csrf(token: &str)` (adds `CsrfToken`), `.child`, `.children`, `.multipart()` (`enctype="multipart/form-data"`, POST only; debug panic on GET). Renders `<form class="st-form" method="post" action="…">`.
  - `Fieldset::new(legend: impl Render + 'a)` `.child`/`.children` → `<fieldset class="st-fieldset"><legend class="st-legend">…</legend>…</fieldset>`; `Legend::new(content)` for standalone use.
  - `FieldHint::new(id: &str, text)` → `<p class="st-field-hint" id="…">`; `FieldError::new(id: &str, messages: &[String])` → `<p class="st-field-error" id="…">` with an error icon-free prefix "Error: " in `st-sr-only`, messages joined by `<br>`; renders nothing if `messages` is empty.
  - `HiddenInput::new(name, value)`; `CsrfToken::new(token)` → `<input type="hidden" name="_csrf" value="…">` (name from `behavior::CSRF_FIELD`).
- `RESERVED`: `Form` reserves `method`, `action`, `enctype`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn post_forms_carry_csrf_tokens() {
    let html = to_html(&Form::post("/orders").csrf("t\"k").child("x"));
    assert_eq!(html, r#"<form class="st-form" method="post" action="/orders"><input type="hidden" name="_csrf" value="t&quot;k">x</form>"#);
}

#[test]
fn field_errors_render_only_when_present() {
    assert_eq!(to_html(&FieldError::new("e", &[])), "");
    assert_eq!(to_html(&FieldError::new("e", &["Too <short>".into(), "Required".into()])),
        r#"<p class="st-field-error" id="e"><span class="st-sr-only">Error: </span>Too &lt;short&gt;<br>Required</p>"#);
}

#[test]
fn fieldsets_have_legends() {
    assert_eq!(to_html(&Fieldset::new("Shipping").child("x")),
        r#"<fieldset class="st-fieldset"><legend class="st-legend">Shipping</legend>x</fieldset>"#);
}
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS). Commit `"Add Form, Fieldset, messages, HiddenInput and CsrfToken"`.

---

### Task 9: Controls and Field binding — Input, Textarea, Select, Field

**Files:**
- Create: `crates/stucco-ui/src/forms/{control.rs, input.rs, textarea.rs, select.rs, field.rs}`; Modify: `css/forms.css`

**Interfaces:**
- Produces (feature `forms`):
  - `pub trait Control: Render` with `fn name(&self) -> &str`, `fn explicit_id(&self) -> Option<&str>`, `fn is_sensitive(&self) -> bool`, `fn wire(&mut self, wiring: Wiring)`; `pub struct Wiring { pub id: String, pub described_by: Option<String>, pub invalid: bool, pub value: Option<String> }`.
  - `Input::text(name)`, `::email`, `::password` (sensitive), `::number`, `::search`, `::tel`, `::url`, `::date`; `.value`, `.placeholder`, `.required`, `.disabled`, `.readonly`, `.autocomplete(&str)`, `.min/.max/.step(&str)`, `.sensitive()`, `.prefix(impl Render + 'a)`, `.suffix(impl Render + 'a)` (adornments render `<div class="st-input-group">…</div>`); `.id(&str)` sets the explicit id.
  - `Textarea::new(name)` `.rows(u8)`, `.value`, `.placeholder`, `.required`, `.sensitive()`.
  - `Select::new(name)` `.option(value, label)`, `.options(iter of (value, label))`, `.placeholder(label)` (first `<option value="" disabled>`; selected when no value), `.selected(value)`, `.required`.
  - `Field::new(label: impl Render + 'a, control: C)` where `C: Control + 'a`; `.hint(text)`, `.error(msg)` (appends), `.bind(&FormState)` (adds the state's errors for `control.name()` and, unless sensitive, its first value), `.required()` (adds a visible "required" marker and forwards `required`).
  - Field wiring at render: `id = control.explicit_id()` or `cx.id("field")`; hint id `{id}-hint`, error id `{id}-error`; `described_by` = present ids in order hint, error; `invalid` = any errors. Output: `<div class="st-field"><label class="st-label" for="{id}">…</label>{control}{hint}{error}</div>`.
- `RESERVED`: controls reserve `id` (use `.id()`), `name`, `type`, `aria-describedby`, `aria-invalid`; `Field` reserves the same on its root.

- [ ] **Step 1: Write the failing tests** (`field.rs` tests):

```rust
#[test]
fn field_wires_label_hint_error_and_invalid_state() {
    let html = to_html(&Field::new("Email", Input::email("email")).hint("Work address").error("Required"));
    assert!(html.contains(r#"<label class="st-label" for="field-1">Email</label>"#), "{html}");
    assert!(html.contains(r#"<input class="st-input" type="email" name="email" id="field-1" aria-describedby="field-1-hint field-1-error" aria-invalid="true">"#), "{html}");
    assert!(html.contains(r#"<p class="st-field-hint" id="field-1-hint">Work address</p>"#));
}

#[test]
fn two_fields_get_distinct_ids_and_explicit_ids_are_kept() {
    let html = to_html(&(
        Field::new("A", Input::text("a")),
        Field::new("B", Input::text("b").id("bee")).hint("h"),
    ));
    assert!(html.contains(r#"for="field-1""#) && html.contains(r#"for="bee""#) && html.contains(r#"id="bee-hint""#));
    assert!(!html.contains("aria-invalid"));
}

#[test]
fn binding_redisplays_values_but_never_sensitive_ones() {
    let state = FormState::new().value("email", "a@b.c\"").value("password", "hunter2").error("password", "Too short");
    let email = to_html(&Field::new("Email", Input::email("email")).bind(&state));
    assert!(email.contains(r#"value="a@b.c&quot;""#));
    let password = to_html(&Field::new("Password", Input::password("password")).bind(&state));
    assert!(!password.contains("hunter2") && !password.contains("value=") && password.contains("Too short"));
    let notes = to_html(&Field::new("Notes", Textarea::new("password").sensitive()).bind(&state));
    assert!(!notes.contains("hunter2"));
}

#[test]
fn selects_mark_the_selected_option_and_escape_labels() {
    let html = to_html(&Select::new("plan").placeholder("Choose…").option("free", "Free <tier>").option("pro", "Pro").selected("pro"));
    assert!(html.contains(r#"<option value="" disabled>Choose…</option>"#));
    assert!(html.contains(r#"<option value="free">Free &lt;tier&gt;</option><option value="pro" selected>Pro</option>"#));
}

#[test]
fn adornments_wrap_the_input() {
    let html = to_html(&Input::url("site").prefix("https://"));
    assert!(html.starts_with(r#"<div class="st-input-group"><span class="st-input-adornment">https://</span><input"#));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved attribute")]
fn field_wiring_cannot_be_overridden() { let _ = to_html(&Field::new("x", Input::text("x")).attr("aria-describedby", "y")); }
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS, debug and release). Commit `"Add Input, Textarea, Select and Field binding"`.

---

### Task 10: Checkbox and RadioGroup

**Files:**
- Create: `crates/stucco-ui/src/forms/{checkbox.rs, radio_group.rs}`; Modify: `css/forms.css`

**Interfaces:**
- Produces (feature `forms`):
  - `Checkbox::new(name, label: impl Render + 'a)` `.value(&str)` (default `"on"`), `.checked(bool)`, `.disabled()`, `.bind(&FormState)` (checked when the state's values for `name` contain this value) → `<label class="st-checkbox"><input type="checkbox" name="…" value="…" checked>…</label>`.
  - `RadioGroup::new(name, legend: impl Render + 'a)` `.option(value, label)`, `.selected(&str)`, `.hint(text)`, `.error(msg)`, `.bind(&FormState)` (selected value + errors), `.required()`. Renders a `fieldset` with `legend`, one `label.st-radio` per option, hint/error with ids derived from `cx.id("radios")` and `aria-describedby`/`aria-invalid` on the fieldset.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn checkboxes_bind_by_value() {
    let state = FormState::new().value("tags", "a").value("tags", "c");
    assert!(to_html(&Checkbox::new("tags", "A").value("a").bind(&state)).contains(" checked"));
    assert!(!to_html(&Checkbox::new("tags", "B").value("b").bind(&state)).contains(" checked"));
}

#[test]
fn radio_groups_are_fieldsets_with_described_errors() {
    let state = FormState::new().value("plan", "pro").error("plan", "Pick one");
    let html = to_html(&RadioGroup::new("plan", "Plan").option("free", "Free").option("pro", "Pro").bind(&state));
    assert!(html.starts_with(r#"<fieldset class="st-radio-group" aria-describedby="radios-1-error" aria-invalid="true"><legend class="st-legend">Plan</legend>"#), "{html}");
    assert!(html.contains(r#"<input type="radio" name="plan" value="pro" checked>"#));
    assert!(html.contains(r#"id="radios-1-error""#) && html.contains("Pick one"));
}
```

- [ ] **Step 2–4:** run (FAIL), implement, gate (PASS). Commit `"Add Checkbox and RadioGroup"`.

---

### Task 11: Gallery pages and browser checks

**Files:**
- Create: `gallery/src/{layout_page.rs, typography_page.rs, actions_page.rs, forms_page.rs}`; Modify: `gallery/src/{lib.rs, index.rs}`, `gallery/Cargo.toml` (enable facade features `layout typography actions forms icons`), `gallery/tests/snapshots.rs`
- Create: `browser/tests/forms.spec.ts`; Modify: `browser/tests/a11y.spec.ts`

**Interfaces:**
- Produces: `gallery::{layout_page, typography_page, actions_page, forms_page}::page(&Bundle) -> String`, written as `layout.html`, `typography.html`, `actions.html`, `forms.html` and linked from `index.html`. Each page wraps content in `SkipLink` + `<main id="main">` + `Container`, shows every component and variant with a heading per component, and (forms page) a `Form::post("#")` containing every control, one `Field` bound to a `FormState` with errors and a submitted password.
- Browser: `forms.spec.ts` asserts every input is reachable by `page.getByLabel(...)` (label association), the invalid field has `aria-invalid="true"` and its error text is in its accessible description (`toHaveAccessibleDescription`), Tab moves from the skip link to the first form control, and the password field has an empty value. `a11y.spec.ts` adds the four new pages to the axe sweep (light and dark).

- [ ] **Step 1: Write the failing tests:** gallery snapshot tests per page (`insta::assert_snapshot!`), each also asserting `!html.contains("style=")` except the palette page; `forms.spec.ts` as above; extend the axe page list.
- [ ] **Step 2: Run** `cargo +stable test -p gallery` and `cd browser && npx playwright test` — expected FAIL.
- [ ] **Step 3: Implement**; review snapshots once, accept.
- [ ] **Step 4: Visual check:** screenshot each new page in light and dark with Playwright; fix anything illegible or misaligned.
- [ ] **Step 5: Run the full gate** (debug, release, the two `cargo hack` commands from Task 2, `npx playwright test`) — expected PASS. Commit `"Add gallery pages for layout, typography, actions and forms"`.
