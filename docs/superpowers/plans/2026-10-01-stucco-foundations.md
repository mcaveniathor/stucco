# stucco Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build roadmap step 1 and the theme engine: rendering core with safe markup and attribute policy, identity policy, assets, fragments, `Bundle`, `Page`, the client runtime's requirement loading, the theme engine with all 14 presets, base CSS, and a gallery that doubles as the Playwright test bed.

**Architecture:** Cargo workspace. `stucco-theme` (no dependencies) produces scales, roles, token CSS and contrast reports. `stucco-core` (depends on `stucco-theme`, `inventory`) renders into a `Cx`, collects `Asset`s, renders pages and fragments, and serves hashed CSS/JS via `Bundle`; its runtime module loads behaviour modules required by inserted fragments. `stucco` is the facade. `gallery` (unpublished) writes a static site; `browser/` holds the Playwright suite that runs against it.

**Tech Stack:** Rust edition 2024 (MSRV 1.85), `inventory` 0.3, `insta` 1 (dev); Node 24, `@playwright/test`, `@axe-core/playwright` (dev only); GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-01-stucco-design.md` (revision 2): §4.1–4.8, §5, §10 (runtime only), §11, §13. Integration spec: `2026-10-01-stucco-integration-design.md` (not implemented here). Later plans: (2) step 2 components (layout, typography, actions, form primitives); (3) step 3 `stucco-tower`; (4) step 4 `stucco-redb` + collections slice; (5) step 5 edit slice; (6) step 6 enhanced mode; (7) step 7 catalogue expansion; (8) step 9 marketing, diagrams, adapters (incl. the ambient render context, LS §4.9), release.

## Global Constraints

- Edition 2024, `rust-version = "1.85"` in every crate; no let-chains or other post-1.85 syntax.
- License `MIT OR Apache-2.0`; `LICENSE-MIT` and `LICENSE-APACHE` at the root.
- `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` in every published crate.
- `stucco-theme`: zero dependencies. `stucco-core`: only `stucco-theme` and `inventory`. No HTTP, async or I/O crate in either.
- CSS class prefix `st-`; custom properties `--st-`; layers exactly `stucco.reset, stucco.tokens, stucco.base, stucco.layout, stucco.components, stucco.utilities`.
- Colour literals appear only in generated token CSS.
- Default asset prefix `/_stucco/`. Theme storage key `stucco-theme`. Header names `Stucco-Request`, `Stucco-Target`, `Stucco-Location`, `Stucco-Csrf`. Requirement element `st-require`.
- JS modules are dependency-free ES modules; runtime ≤ 6 KB gzip.
- Commits: plain messages, no AI attribution lines.
- Gate at the end of every task: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

## Review Focus

1. **Event-handler and HTML-bearing attributes in any case** — `attr("OnClick", …)`, `attr("ONLOAD", …)`, `attr("srcdoc", …)` are refused (debug panic, release drop); `trusted_attr("onclick", …)` is the only way through. Test in Task 4.
2. **Obfuscated URLs, including in lists** — `" JaVaScRiPt:x"`, `"java\tscript:x"`, `"\u{0}javascript:x"`, `"data:text/html,…"` become `#`; `srcset="a.png 1x, javascript:x 2x"` keeps only `a.png 1x`; relative URLs with `:` after `/ ? #` stay valid. Tests in Tasks 3–4.
3. **Fragment namespaces** — `render_fragment("", …)` and `render_fragment("Row 7", …)` panic (invalid namespace); two fragments rendered with namespaces `a` and `b` never share a generated id. Test in Task 12.
4. **Concurrent fragments requiring the same module** — two fragments inserted at once load the module exactly once and both enhance; a module URL outside the bundle directory or another origin is never requested. Test in Task 15.
5. **`Bundle::get` with odd paths** — query strings ignored; missing prefix, `..`, stale hash, bare prefix → `None`; custom prefix without trailing slash normalised. Test in Task 11.

---

### Task 1: Workspace scaffold and HTML escaping

**Files:**
- Create: `Cargo.toml`, `.gitignore`, `rustfmt.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `.github/workflows/ci.yml`
- Create: `crates/stucco-theme/{Cargo.toml,src/lib.rs}`, `crates/stucco-core/{Cargo.toml,src/lib.rs,src/escape.rs}`, `crates/stucco/{Cargo.toml,src/lib.rs}`, `gallery/{Cargo.toml,src/main.rs}`

**Interfaces:**
- Produces: `stucco_core::escape::{escape_text(s: &str, out: &mut String), escape_attr(s: &str, out: &mut String)}`.

- [ ] **Step 1: Create the workspace.** `members = ["crates/*", "gallery"]`, `resolver = "3"`, `[workspace.package]` (version `0.1.0`, edition `2024`, rust-version `1.85`, license), `[workspace.dependencies]` for internal crates, `inventory = "0.3"`, `insta = "1"`. `gallery` has `publish = false`. CI jobs: `fmt`, `clippy`, `test`, `msrv` (`cargo +1.85 check --workspace`), `browser` (added in Task 15). `.gitignore`: `target/`, `node_modules/`, `browser/test-results/`, `browser/playwright-report/`.

- [ ] **Step 2: Write the failing tests** in `escape.rs`:

```rust
#[test]
fn text_escapes_markup_characters() {
    let mut s = String::new();
    escape_text("<a href=\"x\">&'</a>", &mut s);
    assert_eq!(s, "&lt;a href=\"x\"&gt;&amp;'&lt;/a&gt;");
}

#[test]
fn attributes_also_escape_quotes() {
    let mut s = String::new();
    escape_attr("a\"b'c<d>&", &mut s);
    assert_eq!(s, "a&quot;b&#39;c&lt;d&gt;&amp;");
}
```

- [ ] **Step 3: Run** `cargo test -p stucco-core escape` — expected FAIL (not defined).
- [ ] **Step 4: Implement** both (append to `out`, copying unescaped runs as slices).
- [ ] **Step 5: Run the gate** — expected PASS. Commit `"Scaffold workspace and HTML escaping"`.

---

### Task 2: Render, Cx, Slot, Raw and the identity policy

**Files:**
- Create: `crates/stucco-core/src/render.rs`, `crates/stucco-core/src/identity.rs`

**Interfaces:**
- Consumes: escaping (Task 1).
- Produces:
  - `pub trait Render { fn render(&self, cx: &mut Cx); }`
  - `pub struct Cx`: `Cx::new() -> Cx` (empty namespace), `pub(crate) fn with_namespace(ns: &str) -> Cx`, `cx.id(&mut self, prefix: &str) -> String` → `"{ns}-{prefix}-{n}"` when a namespace is set, `"{prefix}-{n}"` otherwise (n from 1, per prefix); `cx.claim_id(&mut self, id: &str)` records an explicit id (duplicate within one `Cx`: `debug_assert!` panic "duplicate id"); `cx.id` also records what it returns; `cx.text(&str)` (escaped); `pub(crate) fn raw(&str)`; `cx.finish(self) -> (String, AssetRequirements)` (`AssetRequirements` defined in Task 5; until then return `()` and change the signature in Task 5).
  - `Render` impls: `str`, `String`, `Cow<'_, str>`, `char`, integers, `f32`, `f64` (non-finite → empty), `Option<T>`, `Vec<T>`, `[T]`, `&T`, `Box<T>`, `Rc<T>`, `Arc<T>`, tuples arity 1–12 (macro). No impl for `bool`.
  - `pub fn render_fn<F: Fn(&mut Cx)>(f: F) -> RenderFn<F>`.
  - `pub struct Slot<'a>` holding `Box<dyn Render + 'a>` and `type_name: &'static str`; `Slot::new(r: impl Render + 'a) -> Slot<'a>`; `impl Render for Slot<'_>`; `impl Debug` prints `Slot(<type name>)`.
  - `pub fn to_html(r: &(impl Render + ?Sized)) -> String` (discards requirements; doc says so and points to `render_fragment`).
  - `pub struct Raw(String)`; `Raw::trusted(s: impl Into<String>) -> Raw` renders unescaped.

- [ ] **Step 1: Write the failing tests** in `render.rs`:

```rust
#[test]
fn values_render_escaped_and_compose() {
    let none: Option<&str> = None;
    assert_eq!(to_html(&("a<b", none, Some(3u8), vec!['&', 'x'], 1.5f64)), "a&lt;b3&amp;x1.5");
}

#[test]
fn raw_is_the_only_unescaped_path() {
    assert_eq!(to_html(&Raw::trusted("<b>x</b>")), "<b>x</b>");
    assert_eq!(to_html(&"<b>x</b>"), "&lt;b&gt;x&lt;/b&gt;");
}

#[test]
fn generated_ids_are_per_prefix_and_namespaced() {
    let mut cx = Cx::new();
    assert_eq!((cx.id("tabs"), cx.id("tabs"), cx.id("menu")), ("tabs-1".into(), "tabs-2".into(), "menu-1".into()));
    let mut ns = Cx::with_namespace("order-42");
    assert_eq!(ns.id("tabs"), "order-42-tabs-1");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "duplicate id")]
fn duplicate_explicit_ids_panic_in_debug() {
    let mut cx = Cx::new();
    cx.claim_id("orders");
    cx.claim_id("orders");
}

#[test]
fn slots_borrow_and_debug_print_their_type() {
    let owned = String::from("<hi>");
    let s = Slot::new(&owned);
    assert_eq!(to_html(&s), "&lt;hi&gt;");
    assert_eq!(format!("{s:?}"), "Slot(&alloc::string::String)");
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core render` — expected FAIL.
- [ ] **Step 3: Implement** `render.rs` and `identity.rs` (id counters and the claimed-id set).
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add Render, Cx, Slot and identity policy"`.

---

### Task 3: Href

**Files:**
- Create: `crates/stucco-core/src/href.rs`

**Interfaces:**
- Produces: `pub struct Href`: `Href::new(impl Into<String>)`, `Href::trusted(impl Into<String>)`, `Href::invalid()`, `is_valid() -> bool`, `as_str() -> &str` (`"#"` when invalid); `From<&str>`, `From<String>` via `new`. `pub(crate) fn filter_url_list(value: &str, kind: UrlList) -> String` with `enum UrlList { Srcset, SpaceSeparated }` (srcset: comma-separated candidates `url [descriptor]`; ping: whitespace-separated), keeping only valid URLs.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn safe_urls_are_kept() {
    for ok in ["/a", "a/b", "../x", "?q=1", "#top", "/a:b", "?x=y:z", "https://e.com",
               "HTTP://e.com", "mailto:a@b.c", "tel:+1", "//cdn.example/x.js"] {
        assert_eq!(Href::new(ok).as_str(), ok, "{ok}");
    }
}

#[test]
fn unsafe_schemes_become_hash() {
    for bad in ["javascript:alert(1)", " JaVaScRiPt:alert(1)", "java\tscript:x",
                "\u{0}javascript:x", "data:text/html,<b>", "vbscript:x", "file:///etc"] {
        assert_eq!(Href::new(bad).as_str(), "#", "{bad:?}");
    }
    assert_eq!(Href::trusted("javascript:void(0)").as_str(), "javascript:void(0)");
}

#[test]
fn url_lists_drop_unsafe_entries() {
    assert_eq!(filter_url_list("a.png 1x, javascript:x 2x, b.png 3x", UrlList::Srcset), "a.png 1x, b.png 3x");
    assert_eq!(filter_url_list("/p1 javascript:x /p2", UrlList::SpaceSeparated), "/p1 /p2");
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core href` — expected FAIL.
- [ ] **Step 3: Implement.** Normalise a copy: strip leading ASCII whitespace and C0 controls, remove tab/LF/CR anywhere; find the first of `: / ? #`; if `:`, the prefix is a scheme allowed only if it case-insensitively equals `http`, `https`, `mailto`, `tel`; otherwise relative and valid. Keep the original string when valid.
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add Href and URL list filtering"`.

---

### Task 4: Element builder, attribute policy and Attrs

**Files:**
- Create: `crates/stucco-core/src/el.rs`, `crates/stucco-core/src/el/tags.rs`, `crates/stucco-core/src/attrs.rs`, `crates/stucco-core/src/aria.rs` (ARIA 1.2 attribute name list)

**Interfaces:**
- Consumes: `Render`, `Cx`, `Href`, `filter_url_list`, escaping.
- Produces:
  - `pub struct Element`, `pub struct VoidElement`; module `el` with one fn per standard HTML element (void set: `area base br col embed hr img input link meta source track wbr`) and `el::custom(tag: &str) -> Element` (lowercase ASCII, starts with a letter, contains `-`; invalid → debug panic, release renders `div`).
  - Shared methods: `.class(impl AsRef<str>)` (space-separated, appends, de-duplicates, order kept), `.id(impl Into<String>)` (replaces; claimed via `cx.claim_id` at render), `.attr(name, value)`, `.trusted_attr(name, value)`, `.bool_attr(name, bool)`, `.data(name, value)`, `.aria(name, value)`, `.href(impl Into<Href>)`, `.src(impl Into<Href>)`, `.action(impl Into<Href>)`, `.attrs(&Attrs)`.
  - `Element` only: `.child(impl Render + 'a)`, `.child_if(bool, impl Render + 'a)`, `.children(impl IntoIterator<Item = impl Render + 'a>)`, `.text(impl Into<String>)`. (`Element<'a>` / `VoidElement` carry the slot lifetime.)
  - `pub struct Attrs` (`Default, Clone, Debug`) with the same attribute methods; `Attrs::reserved_conflicts(&self, reserved: &[&str]) -> Vec<String>` for components (used by later plans).
- Policy (LS §4.4):
  - Valid attribute name: ASCII letter first, then letters/digits/`- _ : .`. `data`/`aria` names `[a-z0-9-]+`; `aria` names not in the ARIA 1.2 list → debug panic.
  - `attr()` refuses names that case-insensitively start with `on` or equal `srcdoc`.
  - `href src action formaction poster cite xlink:href` (and `data` on `object`) go through `Href::new`; `srcset` through `UrlList::Srcset`; `ping` through `UrlList::SpaceSeparated`.
  - Refused or invalid → `debug_assert!(false, "refused attribute: {name}")` / `"invalid attribute name: {name}"`, then dropped.
  - Output order: `id`, `class`, then insertion order; repeated non-class attributes keep the last value.

- [ ] **Step 1: Write the failing tests** in `el.rs`:

```rust
#[test]
fn builds_nested_markup_with_escaping() {
    let html = to_html(&el::section().class("hero").id("intro").data("state", "open")
        .child(el::h1().text("A & B"))
        .child_if(false, el::p().text("hidden"))
        .children(["x", "y"].map(|s| el::li().text(s))));
    assert_eq!(html, r#"<section id="intro" class="hero" data-state="open"><h1>A &amp; B</h1><li>x</li><li>y</li></section>"#);
}

#[test]
fn class_appends_and_attrs_merge() {
    let attrs = Attrs::default().class("b c").attr("hx-get", "/more");
    let html = to_html(&el::div().class("a b").attrs(&attrs).bool_attr("hidden", true).bool_attr("inert", false));
    assert_eq!(html, r#"<div class="a b c" hx-get="/more" hidden></div>"#);
}

#[test]
fn url_attributes_are_checked_including_lists() {
    assert_eq!(to_html(&el::img().src("javascript:x").attr("srcset", "a.png 1x, javascript:x 2x")),
               r#"<img src="#" srcset="a.png 1x">"#);
}

#[test]
fn trusted_attr_is_the_only_way_to_set_handlers() {
    assert_eq!(to_html(&el::button().trusted_attr("onclick", "go()")), r#"<button onclick="go()"></button>"#);
}

#[test]
fn raw_text_elements_stay_escaped() {
    assert_eq!(to_html(&el::script().text("</script><b>")), "<script>&lt;/script&gt;&lt;b&gt;</script>");
}

#[test]
fn custom_elements_are_validated() {
    assert_eq!(to_html(&el::custom("st-tabs")), "<st-tabs></st-tabs>");
}

#[cfg(debug_assertions)]
mod refusals {
    use super::*;
    #[test] #[should_panic(expected = "refused attribute")] fn mixed_case_handler() { let _ = el::div().attr("OnClick", "x"); }
    #[test] #[should_panic(expected = "refused attribute")] fn upper_case_handler() { let _ = el::img().attr("ONLOAD", "x"); }
    #[test] #[should_panic(expected = "refused attribute")] fn srcdoc() { let _ = el::iframe().attr("srcdoc", "<b>"); }
    #[test] #[should_panic(expected = "invalid attribute name")] fn spaces() { let _ = el::div().attr("a b", "x"); }
    #[test] #[should_panic(expected = "unknown aria attribute")] fn aria_typo() { let _ = el::div().aria("lable", "x"); }
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core el` — expected FAIL.
- [ ] **Step 3: Implement** (`tags.rs` via a `macro_rules!` list).
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add element builder and attribute policy"`.

---

### Task 5: Assets, requirements and the behaviour vocabulary

**Files:**
- Create: `crates/stucco-core/src/asset.rs`, `crates/stucco-core/src/behavior.rs`
- Modify: `crates/stucco-core/src/render.rs` (`Cx::require`, `Cx::finish` returns `AssetRequirements`)

**Interfaces:**
- Produces:
  - `pub struct Asset { pub name: &'static str, pub css: Option<&'static str>, pub behavior: Option<Behavior>, pub deps: &'static [&'static Asset] }`; `#[non_exhaustive] pub enum Behavior { Js(&'static str) }`.
  - `cx.require(&mut self, asset: &'static Asset)`.
  - `#[derive(Clone, Debug, Default)] pub struct AssetRequirements` — resolved list: `iter() -> impl Iterator<Item = &'static Asset>`, `behaviors()` (assets with a behaviour), `is_empty()`. Resolution: depth-first post-order over `deps` in declaration order, from requirements in first-require order; each asset once (pointer identity); cycles terminate.
  - `pub struct AssetRef(pub &'static Asset)`; `inventory::collect!(AssetRef)`; `#[macro_export] register_asset!($a:path)`; `#[doc(hidden)] pub use inventory`; `pub fn registered_assets() -> Vec<&'static Asset>` sorted by name; `pub fn is_registered(a: &'static Asset) -> bool`.
  - `behavior` constants: `THEME_STORAGE_KEY = "stucco-theme"`, `THEME_ATTR = "data-theme"`, `NAMED_THEME_ATTR = "data-st-theme"`, `REQUIRE_TAG = "st-require"`, `REGION_ATTR = "data-st-region"`, `STATE_ATTR = "data-state"`, `ASSET_ERROR_EVENT = "stucco:asset-error"`, `HEADER_REQUEST = "Stucco-Request"`, `HEADER_TARGET = "Stucco-Target"`, `HEADER_LOCATION = "Stucco-Location"`, `HEADER_CSRF = "Stucco-Csrf"`; and `pub fn js_placeholders() -> &'static [(&'static str, &'static str)]` mapping `"__STUCCO_REQUIRE_TAG__"` etc. (one per constant, `__STUCCO_<CONST_NAME>__`) to values.

- [ ] **Step 1: Write the failing tests** in `asset.rs`:

```rust
static D: Asset = Asset { name: "d", css: Some(".d{}"), behavior: None, deps: &[] };
static B: Asset = Asset { name: "b", css: None, behavior: None, deps: &[&D] };
static C: Asset = Asset { name: "c", css: None, behavior: None, deps: &[&D] };
static A: Asset = Asset { name: "a", css: None, behavior: None, deps: &[&B, &C] };
static X: Asset = Asset { name: "x", css: None, behavior: None, deps: &[&Y] };
static Y: Asset = Asset { name: "y", css: None, behavior: None, deps: &[&X] };
crate::register_asset!(D);

fn names(r: &AssetRequirements) -> Vec<&'static str> { r.iter().map(|a| a.name).collect() }

#[test]
fn diamonds_resolve_once_dependencies_first() {
    let mut cx = Cx::new();
    cx.require(&A);
    cx.require(&D);
    assert_eq!(names(&cx.finish().1), ["d", "b", "c", "a"]);
}

#[test]
fn cycles_terminate() {
    let mut cx = Cx::new();
    cx.require(&X);
    assert_eq!(names(&cx.finish().1), ["y", "x"]);
}

#[test]
fn registry_finds_registered_assets_only() {
    assert!(is_registered(&D) && !is_registered(&A));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core asset` — expected FAIL.
- [ ] **Step 3: Implement** (the macro uses `$crate` paths so it works inside and outside the crate).
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add assets, requirements and behaviour vocabulary"`.

---

### Task 6: Colour — OKLCH, gamut mapping, contrast

**Files:**
- Create: `crates/stucco-theme/src/color.rs`

**Interfaces:**
- Produces: `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Color { pub l: f64, pub c: f64, pub h: f64 }`; `Color::oklch(l, c, h)` (clamp `l` to 0..=1, `c` ≥ 0, hue into `[0, 360)`, `NaN` → 0); `gamut_mapped(self) -> Color`; `to_srgb(self) -> [f64; 3]`; `css(self) -> String` as `oklch(L% C H)` (L percent 2 dp, C 4 dp, H 2 dp, of the gamut-mapped colour); `luminance(self) -> f64` (WCAG); `pub fn contrast(a: Color, b: Color) -> f64`.

- [ ] **Step 1: Write the failing tests:**

```rust
fn close(a: f64, b: f64) -> bool { (a - b).abs() < 0.01 }

#[test]
fn known_colours_convert() {
    assert!(Color::oklch(1.0, 0.0, 0.0).to_srgb().iter().all(|v| close(*v, 1.0)));
    let red = Color::oklch(0.62796, 0.25768, 29.2339).to_srgb();
    assert!(close(red[0], 1.0) && close(red[1], 0.0) && close(red[2], 0.0), "{red:?}");
}

#[test]
fn contrast_matches_wcag() {
    let (k, w) = (Color::oklch(0.0, 0.0, 0.0), Color::oklch(1.0, 0.0, 0.0));
    assert!(close(contrast(k, w), 21.0) && close(contrast(w, k), 21.0));
}

#[test]
fn inputs_are_normalised() {
    assert_eq!(Color::oklch(0.5, 0.1, -30.0).h, 330.0);
    assert!(close(Color::oklch(0.5, 0.1, 725.0).h, 5.0));
    assert_eq!(Color::oklch(0.5, 0.1, f64::NAN).h, 0.0);
}

#[test]
fn out_of_gamut_chroma_is_reduced() {
    let m = Color::oklch(0.7, 0.5, 145.0).gamut_mapped();
    assert!(close(m.l, 0.7) && close(m.h, 145.0) && m.c < 0.5);
    assert!(m.to_srgb().iter().all(|v| (0.0..=1.0).contains(v)));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme color` — expected FAIL.
- [ ] **Step 3: Implement.** OKLCH → OKLab → linear sRGB (Ottosson's matrices) → sRGB transfer. In gamut = every linear channel within `[-1e-4, 1 + 1e-4]`; otherwise binary-search chroma in `[0, c]` for 20 iterations keeping the largest in-gamut value. `to_srgb` clamps the final channels.
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add OKLCH colour, gamut mapping and contrast"`.

---

### Task 7: Scales

**Files:**
- Create: `crates/stucco-theme/src/scale.rs`

**Interfaces:**
- Produces: `pub struct Scale { pub light: [Color; 12], pub dark: [Color; 12] }`; `Scale::new(hue, chroma)`; `step(scheme, n: usize) -> Color` (1-based; panics outside 1..=12); `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Scheme { Light, Dark }`, `Scheme::BOTH`.

Step semantics: 1–2 app backgrounds, 3–5 component backgrounds, 6–8 borders, 9–10 solid fills, 11 low-contrast text, 12 high-contrast text.

| step | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| light L | .99 | .975 | .95 | .92 | .885 | .845 | .79 | .71 | .60 | .55 | .45 | .24 |
| dark L | .16 | .19 | .23 | .26 | .30 | .34 | .40 | .48 | .60 | .66 | .80 | .95 |
| chroma × | .10 | .15 | .25 | .35 | .45 | .55 | .65 | .80 | 1.0 | 1.0 | .85 | .45 |

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn steps_follow_the_curves() {
    let s = Scale::new(250.0, 0.15);
    assert_eq!(s.step(Scheme::Light, 1).l, 0.99);
    assert_eq!(s.step(Scheme::Dark, 12).l, 0.95);
    assert_eq!(s.step(Scheme::Light, 9).c, 0.15);
    assert!((s.step(Scheme::Light, 1).c - 0.015).abs() < 1e-9);
}

#[test]
fn text_steps_contrast_with_backgrounds() {
    for hue in [0.0, 60.0, 145.0, 250.0, 320.0] {
        let s = Scale::new(hue, 0.15);
        for scheme in Scheme::BOTH {
            assert!(contrast(s.step(scheme, 12), s.step(scheme, 1)) >= 7.0, "{hue} {scheme:?}");
            assert!(contrast(s.step(scheme, 11), s.step(scheme, 2)) >= 4.5, "{hue} {scheme:?}");
        }
    }
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme scale` — expected FAIL.
- [ ] **Step 3: Implement** with the table. If a hue fails the step-11 check, lower light L11 / raise dark L11 in 0.02 increments until all pass, and record the new values in this table.
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add 12-step OKLCH scales"`.

---

### Task 8: Theme builder, roles, contrast report and token CSS

**Files:**
- Create: `crates/stucco-theme/src/theme.rs`, `crates/stucco-theme/src/roles.rs`, `crates/stucco-theme/src/css.rs`

**Interfaces:**
- Produces:
  - `#[derive(Clone, Debug)] pub struct Theme`: `from_seed(hue)`; `.neutral_tint(chroma)` (default 0.01), `.neutral_hue(hue)` (default = accent hue), `.accent(Color)` (default chroma 0.15), `.accent_from_neutral()`, `.text_lightness(light, dark)` (overrides neutral step 12 L), `.fonts(Fonts)`, `.type_scale(TypeScale)`, `.space(px)` (default 4.0), `.radius(Radius)`, `.density(Density)`, `.min_contrast(ratio)` (default 4.5), `.build(self) -> Result<BuiltTheme, ContrastReport>`.
  - `Fonts::system()` (sans `system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`; mono `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`), `.sans(&str)`, `.mono(&str)` (prepended, quoted).
  - `TypeScale::new(base_px, ratio)` (default 16, 1.2) → `--st-text-xs` (step −2), `-sm`, `-base`, `-lg`, `-xl`, `-2xl`, `-3xl` (+5), each `clamp(<0.9×size>rem, <a>rem + <b>vw, <size>rem)` interpolating between viewport 360px and 1280px.
  - `enum Radius { Sharp, Soft, Round }` → `--st-radius-sm/md/lg` Sharp `0` each, Soft `4px 8px 12px`, Round `8px 14px 20px`; `--st-radius-full: 9999px` always.
  - `enum Density { Compact, Comfortable }` → `--st-control-h` 32px / 40px; `--st-pad-scale` 0.75 / 1.
  - Spacing `--st-space-1..12` = unit × [1,2,3,4,5,6,8,10,12,16,20,24]; motion `--st-duration-fast: 120ms; --st-duration: 200ms; --st-duration-slow: 320ms; --st-ease: cubic-bezier(.2,0,0,1)`; z `--st-z-dropdown: 100; --st-z-sticky: 200; --st-z-overlay: 300; --st-z-toast: 400`; shadows `--st-shadow-1..3` via `color-mix(in oklch, var(--st-neutral-12) <8|12|18>%, transparent)`.
  - `pub struct BuiltTheme`: `css(&self, scope: Scope<'_>) -> String`, `role(&self, scheme, name: &str) -> Option<Color>`; `pub enum Scope<'a> { Root, Named(&'a str) }`.
  - `pub struct ContrastReport { pub failures: Vec<ContrastFailure> }` (`Display`: one line per failure, `"{fg} on {bg} ({scheme:?}): {ratio:.2} < {required}"`); `pub struct ContrastFailure { pub fg: &'static str, pub bg: &'static str, pub scheme: Scheme, pub ratio: f64, pub required: f64 }`.

Roles (`roles.rs`):

| role | source | role | source |
|---|---|---|---|
| `bg` | neutral 1 | `accent` | accent 9 |
| `surface` | neutral 2 | `accent-hover` | accent 10 |
| `surface-raised` | neutral 3 | `accent-text` | accent 11 |
| `hover` | neutral 4 | `accent-soft` | accent 3 |
| `text` | neutral 12 | `on-accent` | black or white, higher contrast with `accent` |
| `text-muted` | neutral 11 | `focus` | accent 8 |
| `border` | neutral 6 | `{s}` / `{s}-text` / `{s}-soft` | {s} 9 / 11 / 3 |
| `border-strong` | neutral 8 | | |

`{s}`: success (hue 150), warning (85), danger (25), info (240), chroma 0.14.

Contrast pairs — text (`min_contrast`): `text`/`bg`, `text`/`surface`, `text`/`surface-raised`, `text-muted`/`bg`, `text-muted`/`surface`, `accent-text`/`bg`, `accent-text`/`surface`, `on-accent`/`accent`, `{s}-text`/`bg`, `{s}-text`/`{s}-soft`; UI (3.0): `border-strong`/`bg`, `focus`/`bg`.

CSS (`Root` → `:root`, `Named(n)` → `[data-st-theme="n"]`):

```css
@layer stucco.tokens {
  :root { color-scheme: light dark;
    --st-neutral-1: light-dark(oklch(…), oklch(…)); …   /* 6 scales × 12 */
    --st-bg: var(--st-neutral-1); …                      /* roles */
    --st-on-accent: light-dark(oklch(…), oklch(…));
    --st-font-sans: …; --st-text-base: …; --st-space-1: 4px; … }
  [data-theme="light"] { color-scheme: light; }
  [data-theme="dark"] { color-scheme: dark; }
}
```

The two `[data-theme]` rules only for `Scope::Root`.

- [ ] **Step 1: Write the failing tests** in `theme.rs`:

```rust
#[test]
fn default_seed_meets_aa_in_both_schemes() {
    let built = Theme::from_seed(250.0).build().expect("valid");
    for scheme in Scheme::BOTH {
        assert!(contrast(built.role(scheme, "text").unwrap(), built.role(scheme, "bg").unwrap()) >= 4.5);
    }
}

#[test]
fn an_impossible_minimum_reports_failing_pairs() {
    let err = Theme::from_seed(250.0).min_contrast(30.0).build().unwrap_err();
    assert!(err.failures.iter().any(|f| f.fg == "text" && f.bg == "bg" && f.scheme == Scheme::Dark));
    assert!(err.to_string().contains("text on bg"));
}

#[test]
fn root_css_has_layer_scales_roles_and_scheme_switches() {
    let css = Theme::from_seed(250.0).build().unwrap().css(Scope::Root);
    assert!(css.starts_with("@layer stucco.tokens {"));
    assert!(css.contains("--st-neutral-1: light-dark(oklch("));
    assert!(css.contains("--st-bg: var(--st-neutral-1);"));
    assert!(css.contains("[data-theme=\"dark\"] { color-scheme: dark; }"));
    assert!(css.contains("--st-space-3: 12px;"));
}

#[test]
fn named_scope_has_no_scheme_switches() {
    let css = Theme::from_seed(300.0).build().unwrap().css(Scope::Named("brand"));
    assert!(css.contains("[data-st-theme=\"brand\"] {") && !css.contains("[data-theme="));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme theme` — expected FAIL.
- [ ] **Step 3: Implement** per the tables.
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add theme builder, roles, contrast report and token CSS"`.

---

### Task 9: Presets

**Files:**
- Create: `crates/stucco-theme/src/presets.rs`

**Interfaces:**
- Produces: `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Preset { Slate, Graphite, Stone, Sand, Sepia, Pine, Fjord, Ocean, Iris, Rose, Ember, Sol, Terminal, Contrast }`; `Preset::ALL`; `name() -> &'static str` (lowercase); `Theme::preset(p) -> Theme`; `impl From<Preset> for BuiltTheme` (`build().expect("presets are validated in CI")`). There is **no** `From<Theme> for BuiltTheme`.

| preset | accent hue / chroma | neutral hue / tint | other |
|---|---|---|---|
| Slate | 250 / .15 | 250 / .012 | — |
| Graphite | 0 / 0 | 0 / 0 | `accent_from_neutral()` |
| Stone | 70 / .14 | 60 / .010 | Radius::Round |
| Sand | 40 / .12 | 75 / .020 | sans `"Iowan Old Style", Georgia, serif` |
| Sepia | 55 / .09 | 70 / .030 | fonts as Sand |
| Pine | 160 / .11 | 165 / .015 | — |
| Fjord | 230 / .08 | 225 / .018 | — |
| Ocean | 205 / .13 | 215 / .012 | Radius::Sharp |
| Iris | 290 / .17 | 290 / .014 | — |
| Rose | 355 / .14 | 350 / .012 | Radius::Round |
| Ember | 35 / .19 | 40 / .008 | Radius::Sharp |
| Sol | 90 / .15 | 85 / .020 | — |
| Terminal | 145 / .18 | 145 / .010 | sans = mono stack, Radius::Sharp |
| Contrast | 255 / .20 | 0 / 0 | `min_contrast(7.0)`, `text_lightness(0.10, 1.0)` |

Values are starting points; tune in OKLCH if a test fails and record the final values in the file.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn every_preset_builds_in_both_schemes() {
    for p in Preset::ALL {
        if let Err(report) = Theme::preset(p).build() { panic!("{} fails:\n{report}", p.name()); }
    }
}

#[test]
fn contrast_preset_meets_aaa_for_text() {
    let built = Theme::preset(Preset::Contrast).build().unwrap();
    for scheme in Scheme::BOTH {
        for (fg, bg) in [("text", "bg"), ("text-muted", "bg"), ("accent-text", "bg")] {
            let r = contrast(built.role(scheme, fg).unwrap(), built.role(scheme, bg).unwrap());
            assert!(r >= 7.0, "{fg} on {bg} {scheme:?}: {r}");
        }
    }
}

#[test]
fn modified_presets_must_be_built() {
    let built: BuiltTheme = Theme::preset(Preset::Sand).radius(Radius::Sharp).build().unwrap();
    assert!(built.css(Scope::Root).contains("--st-radius-md: 0;"));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme presets` — expected FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add the 14 theme presets"`.

---

### Task 10: Base CSS layers and CSS rule checks

**Files:**
- Create: `crates/stucco-core/css/{layers.css,reset.css,base.css}`, `crates/stucco-core/src/base_css.rs`, `crates/stucco-core/tests/css_rules.rs`

**Interfaces:**
- Produces: `pub const LAYERS_CSS, RESET_CSS, BASE_CSS: &str`; `pub fn check_component_css(name: &str, css: &str) -> Result<(), String>` (every later plan runs it on each `Asset::css`).

Content: `layers.css` is the single `@layer` order statement. `reset.css` (`@layer stucco.reset`): border-box, margin reset, `img, svg, video { display: block; max-inline-size: 100% }`, inherited fonts on controls, `text-wrap: pretty` on `p`, `balance` on headings. `base.css` (`@layer stucco.base`): body uses `--st-bg`, `--st-text`, `--st-font-sans`, `--st-text-base`, line-height 1.5; `:focus-visible { outline: 2px solid var(--st-focus); outline-offset: 2px }`; `.st-sr-only`; links `--st-accent-text`; `code, kbd, pre` `--st-font-mono`; reduced motion sets `animation-duration` and `transition-duration` to `0.01ms` and `scroll-behavior: auto` on `*, ::before, ::after`.

Checker rules: (1) no `oklch(`, `rgb(`, `rgba(`, `hsl(`, `hsla(`, or `#` + 3/4/6/8 hex digits followed by a non-identifier character; (2) no `var(--st-<palette>-<n>)` with palette ∈ neutral, accent, success, warning, danger, info; (3) all top-level content inside `@layer stucco.<layer> { … }` (brace-depth scan, no parser dependency).

- [ ] **Step 1: Write the failing tests:**

```rust
use stucco_core::{check_component_css, BASE_CSS, RESET_CSS};

#[test]
fn base_layers_obey_the_rules() {
    check_component_css("reset", RESET_CSS).unwrap();
    check_component_css("base", BASE_CSS).unwrap();
}

#[test]
fn the_checker_rejects_violations() {
    assert!(check_component_css("x", "@layer stucco.components { .a { color: #fff; } }").is_err());
    assert!(check_component_css("x", "@layer stucco.components { .a { color: var(--st-accent-9); } }").is_err());
    assert!(check_component_css("x", ".a { color: var(--st-text); }").is_err());
    assert!(check_component_css("x", "@layer stucco.components { .a { color: var(--st-text); } }").is_ok());
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core --test css_rules` — expected FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add base CSS layers and component CSS rules"`.

---

### Task 11: Bundle and the runtime file

**Files:**
- Create: `crates/stucco-core/src/bundle.rs`, `crates/stucco-core/src/hash.rs`, `crates/stucco-core/js/runtime.js` (stub this task: `// stucco runtime` plus one placeholder use; filled in Task 15)
- Create: `crates/stucco-core/tests/js_contract.rs`

**Interfaces:**
- Consumes: `BuiltTheme`, `Scope`, `registered_assets`, `js_placeholders`, base CSS.
- Produces:
  - `pub struct Bundle`: `new(theme: impl Into<BuiltTheme>)`, `.with_theme(name: &str, theme: impl Into<BuiltTheme>)`, `.prefix(p: &str)` (normalised to leading and trailing `/`), `stylesheet_url() -> &str`, `runtime_url() -> &str`, `script_url(asset: &Asset) -> Option<&str>` (`None` if unregistered or no JS), `css() -> &str`, `css_for(&AssetRequirements) -> String` (layers + reset + all theme tokens + base + only the required assets' CSS; used by Inline delivery), `get(path) -> Option<AssetFile>`.
  - `#[derive(Clone, Debug)] pub struct AssetFile { pub bytes: Arc<[u8]>, pub mime: &'static str, pub etag: String, pub immutable: bool }`.
  - `pub(crate) fn fnv1a64(&[u8]) -> u64`; file names use 10 lowercase hex digits: `stucco.<h>.css`, `stucco-runtime.<h>.js`, `<asset>.<h>.js`.
  - `pub(crate) fn substitute_placeholders(js: &str) -> String` — replaces every `__STUCCO_*__` from `js_placeholders()`; applied to the runtime and every behaviour script when the bundle is built.
- Stylesheet order: layers, reset, root tokens, named themes (call order), base, registered assets' CSS (by name). Built eagerly; `with_theme`/`prefix` rebuild.
- `get`: drop `?…`; require the prefix; remainder must equal a known file name; else `None`. MIME `text/css; charset=utf-8` / `text/javascript; charset=utf-8`; etag `"\"<h>\""`; `immutable: true`.

- [ ] **Step 1: Write the failing tests** — in `bundle.rs`:

```rust
use stucco_theme::Preset;

#[test]
fn stylesheet_order() {
    let b = Bundle::new(Preset::Slate).with_theme("brand", Preset::Iris);
    let at = |s: &str| b.css().find(s).unwrap_or_else(|| panic!("missing {s}"));
    assert!(at("@layer stucco.reset, stucco.tokens") < at(":root { color-scheme"));
    assert!(at(":root { color-scheme") < at("[data-st-theme=\"brand\"]"));
    assert!(at("[data-st-theme=\"brand\"]") < at("@layer stucco.base"));
}

#[test]
fn serves_hashed_files_ignoring_queries() {
    let b = Bundle::new(Preset::Slate);
    let url = b.stylesheet_url().to_owned();
    assert!(url.starts_with("/_stucco/stucco.") && url.ends_with(".css"));
    let f = b.get(&url).unwrap();
    assert_eq!((f.mime, f.immutable), ("text/css; charset=utf-8", true));
    assert_eq!(&*f.bytes, b.css().as_bytes());
    assert!(b.get(&format!("{url}?v=1")).is_some());
    assert_eq!(b.get(b.runtime_url()).unwrap().mime, "text/javascript; charset=utf-8");
}

#[test]
fn rejects_unknown_paths() {
    let b = Bundle::new(Preset::Slate).prefix("/assets");
    let name = b.stylesheet_url().rsplit('/').next().unwrap().to_owned();
    assert!(b.stylesheet_url().starts_with("/assets/"));
    for bad in [name.clone(), format!("/_stucco/{name}"), format!("/assets/../{name}"),
                "/assets/stucco.0000000000.css".to_owned(), "/assets/".to_owned()] {
        assert!(b.get(&bad).is_none(), "{bad}");
    }
}

#[test]
fn theme_changes_change_the_url() {
    assert_ne!(Bundle::new(Preset::Slate).stylesheet_url(), Bundle::new(Preset::Iris).stylesheet_url());
}
```

and in `tests/js_contract.rs` (the drift test, LS §4.6):

```rust
#[test]
fn every_js_placeholder_is_known_and_substituted() {
    let b = stucco_core::Bundle::new(stucco_theme::Preset::Slate);
    let runtime = String::from_utf8(b.get(b.runtime_url()).unwrap().bytes.to_vec()).unwrap();
    assert!(!runtime.contains("__STUCCO_"), "unsubstituted placeholder in runtime");
    let source = include_str!("../js/runtime.js");
    for word in source.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if word.starts_with("__STUCCO_") {
            assert!(stucco_core::behavior::js_placeholders().iter().any(|(k, _)| *k == word), "unknown {word}");
        }
    }
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core bundle --test js_contract` — expected FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add Bundle, hashed serving and JS contract substitution"`.

---

### Task 12: Fragments

**Files:**
- Create: `crates/stucco-core/src/fragment.rs`

**Interfaces:**
- Consumes: `Cx::with_namespace`, `AssetRequirements`, `is_registered`, `Bundle::script_url`, `behavior::REQUIRE_TAG`.
- Produces:
  - `pub struct RenderedFragment { pub html: String, pub assets: AssetRequirements }`.
  - `pub fn render_fragment(namespace: &str, r: &(impl Render + ?Sized)) -> RenderedFragment` — namespace must match `[a-z0-9-]+` (else panic `"invalid fragment namespace"` in all builds: it is a programming error with collision consequences). Required unregistered assets with a behaviour: `debug_assert!` panic `"unregistered asset in fragment: {name}"`, release removes them from `assets`.
  - `fragment.to_response_html(&self, bundle: &Bundle) -> String`: if any required asset has a behaviour, prefix `<st-require modules="{url1} {url2}"></st-require>` (dependency order, attribute-escaped); then `html`.

- [ ] **Step 1: Write the failing tests:**

```rust
use stucco_theme::Preset;

static PROBE: Asset = Asset { name: "probe", css: None, behavior: Some(Behavior::Js("/*probe*/")), deps: &[] };
crate::register_asset!(PROBE);
struct Probe;
impl Render for Probe {
    fn render(&self, cx: &mut Cx) { cx.require(&PROBE); let id = cx.id("probe"); el::custom("st-probe").id(id).render(cx); }
}

#[test]
fn fragments_namespace_ids_and_keep_requirements() {
    let a = render_fragment("a", &Probe);
    let b = render_fragment("b", &Probe);
    assert_eq!(a.html, r#"<st-probe id="a-probe-1"></st-probe>"#);
    assert_eq!(b.html, r#"<st-probe id="b-probe-1"></st-probe>"#);
    assert_eq!(a.assets.iter().map(|x| x.name).collect::<Vec<_>>(), ["probe"]);
}

#[test]
fn response_html_announces_modules() {
    let bundle = Bundle::new(Preset::Slate);
    let html = render_fragment("a", &Probe).to_response_html(&bundle);
    let url = bundle.script_url(&PROBE).unwrap();
    assert!(html.starts_with(&format!(r#"<st-require modules="{url}"></st-require>"#)));
    assert_eq!(render_fragment("p", &"plain").to_response_html(&bundle), "plain");
}

#[test]
#[should_panic(expected = "invalid fragment namespace")]
fn empty_namespace_panics() { render_fragment("", &"x"); }

#[test]
#[should_panic(expected = "invalid fragment namespace")]
fn spaced_namespace_panics() { render_fragment("Row 7", &"x"); }
```

- [ ] **Step 2: Run** `cargo test -p stucco-core fragment` — expected FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add fragments with requirement announcements"`.

---

### Task 13: Page

**Files:**
- Create: `crates/stucco-core/src/page.rs`, `crates/stucco-core/js/theme_init.js`

**Interfaces:**
- Consumes: `Bundle`, `Cx`, `Attrs`, `Href`, behaviour constants, `substitute_placeholders`.
- Produces:
  - `pub struct Page<'b, 'a>`: `Page::new(bundle: &'b Bundle, title: impl Into<String>)`, `.lang(&str)` (default `"en"`), `.meta(Meta)`, `.head(impl Render + 'a)`, `.body(impl Render + 'a)`, `.body_attrs(Attrs)`, `.csp_nonce(impl Into<String>)`, `.enhanced()`, `.delivery(Delivery)`, `.render(self) -> String`.
  - `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum Delivery { #[default] Linked, Inline }`.
  - `#[derive(Clone, Debug, Default)] pub struct Meta`: `Meta::description(&str)`, `.og_title(&str)`, `.og_image(impl Into<Href>)`, `.canonical(impl Into<Href>)`.
- `theme_init.js`: inside `try`, reads `localStorage.getItem("__STUCCO_THEME_STORAGE_KEY__")`; if `"light"` or `"dark"`, sets `document.documentElement.dataset.theme`. Inlined after placeholder substitution.
- Output: `<!doctype html><html lang="…"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>…</title>` meta tags, theme init `<script>`, then:
  - Linked: `<link rel="stylesheet" href="{stylesheet_url}">`; if any required behaviour or `.enhanced()`: `<script type="module" src="{runtime_url}"></script>`; per required behaviour with a bundle URL `<script type="module" src="…"></script>`; unregistered behaviours inline as `<script type="module">…</script>`.
  - Inline: `<style>{bundle.css_for(&requirements)}</style>`; behaviours inline; **no runtime** (`.enhanced()` is ignored with a debug panic "Inline pages do not support fragments").
  - user head, `</head><body{body_attrs}>…</body></html>`.
  - Every inline `<script>`/`<style>` carries `nonce="…"` when set.
- The body renders first into `Cx::new()`; the head is written from `cx.finish()`.

- [ ] **Step 1: Write the failing tests:**

```rust
use stucco_theme::Preset;

static WIDGET: Asset = Asset { name: "widget", css: Some("@layer stucco.components { .st-widget { color: var(--st-text); } }"),
    behavior: Some(Behavior::Js("/*widget*/")), deps: &[] };
crate::register_asset!(WIDGET);
static LOOSE: Asset = Asset { name: "loose", css: None, behavior: Some(Behavior::Js("/*loose*/")), deps: &[] };

struct Widget;
impl Render for Widget { fn render(&self, cx: &mut Cx) { cx.require(&WIDGET); el::div().class("st-widget").render(cx); } }
struct Loose;
impl Render for Loose { fn render(&self, cx: &mut Cx) { cx.require(&LOOSE); } }

#[test]
fn linked_page_links_css_runtime_and_used_modules() {
    let b = Bundle::new(Preset::Slate);
    let html = Page::new(&b, "A <b> title").meta(Meta::description("d")).body(Widget).render();
    assert!(html.starts_with("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">"));
    assert!(html.contains("<title>A &lt;b&gt; title</title>"));
    assert!(html.contains(&format!("<link rel=\"stylesheet\" href=\"{}\">", b.stylesheet_url())));
    assert!(html.contains(&format!("<script type=\"module\" src=\"{}\"></script>", b.runtime_url())));
    assert!(html.contains(&format!("<script type=\"module\" src=\"{}\"></script>", b.script_url(&WIDGET).unwrap())));
    assert!(html.ends_with("<body><div class=\"st-widget\"></div></body></html>"));
}

#[test]
fn plain_pages_have_no_runtime_unless_enhanced() {
    let b = Bundle::new(Preset::Slate);
    assert!(!Page::new(&b, "t").body("plain").render().contains("type=\"module\""));
    assert!(Page::new(&b, "t").enhanced().body("plain").render().contains(b.runtime_url()));
}

#[test]
fn unregistered_behaviours_are_inlined() {
    let b = Bundle::new(Preset::Slate);
    assert!(Page::new(&b, "t").body(Loose).render().contains("<script type=\"module\">/*loose*/</script>"));
}

#[test]
fn inline_page_ships_only_used_css_and_nonces_everything() {
    let b = Bundle::new(Preset::Slate);
    let html = Page::new(&b, "t").delivery(Delivery::Inline).csp_nonce("n0nce").body(Widget).render();
    assert!(html.contains(".st-widget") && !html.contains("<link rel=\"stylesheet\"") && !html.contains(b.runtime_url()));
    let inline_tags = html.matches("<script").count() + html.matches("<style").count();
    assert_eq!(html.matches("nonce=\"n0nce\"").count(), inline_tags);
    assert!(html.contains("localStorage") && html.contains("stucco-theme"));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core page` — expected FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add Page with linked and inline delivery"`.

---

### Task 14: Facade and gallery (palette page and fragment fixtures)

**Files:**
- Modify: `crates/stucco/{Cargo.toml,src/lib.rs}`
- Create: `gallery/src/{lib.rs,index.rs,palette.rs,fixtures.rs}`, `gallery/js/probe.js`; Modify: `gallery/src/main.rs`
- Create: `gallery/tests/snapshots.rs`

**Interfaces:**
- Produces:
  - Facade re-exports: `stucco::{Render, Cx, Slot, render_fn, to_html, render_fragment, RenderedFragment, Raw, Href, Attrs, el, Asset, Behavior, AssetRequirements, Bundle, AssetFile, Page, Meta, Delivery, register_asset, behavior}`, `stucco::theme::{Theme, Preset, Color, Scheme, Scope, Radius, Density, Fonts, TypeScale, BuiltTheme, ContrastReport}`, `stucco::prelude::{Render, Cx, el, Page, Bundle, Theme, Preset}`. Facade features from LS §3 declared empty (filled by later plans), defaults as in the spec.
  - `gallery` lib: `bundle() -> Bundle` (Slate root theme plus every preset as a named theme), `write_site(dir: &Path) -> io::Result<()>` writing `index.html`, `palette.html`, `fixtures/enhanced.html`, `fixtures/probe-a.html`, `fixtures/probe-b.html`, and every bundle file at its URL path under `dir`. `main.rs`: `cargo run -p gallery -- [dir]` (default `target/gallery`).
  - `palette::palette_page(&Bundle) -> String`: per preset a `<section data-st-theme="<name>">` with two panels (`data-theme="light"`, `data-theme="dark"`), each showing the 6 scales as 12 swatches (inline `style="background: var(--st-<scale>-<n>)"`) and a role sample (text on bg, muted text, accent button, four status colours, border). Gallery CSS is a registered asset named `gallery` that passes `check_component_css`.
  - `fixtures.rs`: registered asset `PROBE` (`js/probe.js`: defines `st-probe`, which sets `data-ready=""` in `connectedCallback`); `enhanced.html` = `Page::new(..).enhanced()` with `<div id="target" data-st-region></div>` and no probe on the page; `probe-a.html` / `probe-b.html` = `render_fragment("probe-a" | "probe-b", &Probe).to_response_html(&bundle)`.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn palette_page_shows_every_preset_in_both_schemes() {
    let html = gallery::palette::palette_page(&gallery::bundle());
    for p in stucco::theme::Preset::ALL {
        assert!(html.contains(&format!("data-st-theme=\"{}\"", p.name())), "{}", p.name());
    }
    assert_eq!(html.matches("data-theme=\"dark\"").count(), 14);
    insta::assert_snapshot!(html);
}

#[test]
fn enhanced_fixture_lacks_the_probe_module_that_fragments_require() {
    let dir = std::env::temp_dir().join(format!("stucco-gallery-{}", std::process::id()));
    gallery::write_site(&dir).unwrap();
    let page = std::fs::read_to_string(dir.join("fixtures/enhanced.html")).unwrap();
    let frag = std::fs::read_to_string(dir.join("fixtures/probe-a.html")).unwrap();
    assert!(frag.starts_with("<st-require modules=\"/_stucco/probe."));
    let probe_url = frag.split('"').nth(1).unwrap();
    assert!(!page.contains(probe_url));
    assert!(dir.join(probe_url.trim_start_matches('/')).exists());
    std::fs::remove_dir_all(dir).unwrap();
}
```

- [ ] **Step 2: Run** `cargo test -p gallery` — expected FAIL.
- [ ] **Step 3: Implement**; review the snapshot once, then `cargo insta accept`.
- [ ] **Step 4: Run the gate** — expected PASS. Commit `"Add facade and gallery with palette page and fixtures"`.

---

### Task 15: Runtime requirement loading and the Playwright harness

**Files:**
- Modify: `crates/stucco-core/js/runtime.js`
- Create: `browser/package.json`, `browser/playwright.config.ts`, `browser/serve.mjs`, `browser/tests/runtime.spec.ts`, `browser/tests/a11y.spec.ts`
- Modify: `.github/workflows/ci.yml` (job `browser`)

**Interfaces:**
- Consumes: gallery fixtures (Task 14), behaviour constants via placeholders.
- Produces: runtime behaviour (LS §4.8.1–2):
  - Defines `__STUCCO_REQUIRE_TAG__`. In `connectedCallback`: find `region = this.parentElement?.closest("[__STUCCO_REGION_ATTR__]") ?? this.parentElement`; for each whitespace-separated URL in `modules`: resolve against `location.href`; accept only if same origin as `location` and `pathname` starts with the directory of `import.meta.url`; otherwise `console.error` and skip. Accepted URLs go through a module-level `Map<string, Promise>` so each URL is imported once; on rejection set `region.setAttribute("__STUCCO_STATE_ATTR__", "error")` and dispatch `new CustomEvent("__STUCCO_ASSET_ERROR_EVENT__", { bubbles: true, detail: { url } })` on `region`. Then `this.remove()`.
  - Size budget: ≤ 6 KB gzip (checked in the browser job with `gzip -c | wc -c`).
- `serve.mjs`: `node serve.mjs <dir> <port>` static server (node:http, MIME by extension, 404 otherwise, ~40 lines).
- `playwright.config.ts`: projects chromium, firefox, webkit; `webServer.command`: `cargo run -p gallery -- ../target/gallery && node serve.mjs ../target/gallery 4173`, `url: http://localhost:4173/index.html`.

- [ ] **Step 1: Write the failing browser tests** in `browser/tests/runtime.spec.ts`:

```ts
import { test, expect } from "@playwright/test";

async function insert(page, urls: string[]) {
  await page.evaluate(async (urls) => {
    const htmls = await Promise.all(urls.map((u) => fetch(u).then((r) => r.text())));
    for (const h of htmls) document.getElementById("target")!.insertAdjacentHTML("beforeend", h);
  }, urls);
}

test("concurrent fragments load a new module once and both enhance", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (r) => r.url().includes("/_stucco/probe.") && requests.push(r.url()));
  await page.goto("/fixtures/enhanced.html");
  await insert(page, ["/fixtures/probe-a.html", "/fixtures/probe-b.html"]);
  await expect(page.locator("st-probe[data-ready]")).toHaveCount(2);
  expect(requests).toHaveLength(1);
  await expect(page.locator("st-require")).toHaveCount(0);
});

test("a missing module marks the region as failed", async ({ page }) => {
  await page.goto("/fixtures/enhanced.html");
  const event = page.evaluate(() => new Promise((res) =>
    document.addEventListener("stucco:asset-error", (e: any) => res(e.detail.url), { once: true })));
  await page.evaluate(() => document.getElementById("target")!
    .insertAdjacentHTML("beforeend", '<st-require modules="/_stucco/missing.0000000000.js"></st-require>'));
  expect(await event).toContain("/_stucco/missing.0000000000.js");
  await expect(page.locator("#target")).toHaveAttribute("data-state", "error");
});

test("foreign and out-of-bundle URLs are never requested", async ({ page }) => {
  const bad: string[] = [];
  page.on("request", (r) => (r.url().includes("example.com") || r.url().includes("/evil.js")) && bad.push(r.url()));
  await page.goto("/fixtures/enhanced.html");
  await page.evaluate(() => document.getElementById("target")!
    .insertAdjacentHTML("beforeend", '<st-require modules="https://example.com/x.js /evil.js"></st-require>'));
  await page.waitForTimeout(200);
  expect(bad).toHaveLength(0);
});
```

and `browser/tests/a11y.spec.ts`: for `palette.html` and `index.html`, with `colorScheme` `light` and `dark`, `new AxeBuilder({ page }).analyze()` has `violations` equal to `[]`; and with `javaScriptEnabled: false` the palette page shows 14 sections.

- [ ] **Step 2: Run** `cd browser && npm install && npx playwright install chromium firefox webkit && npx playwright test` — expected FAIL (runtime is a stub).
- [ ] **Step 3: Implement** `runtime.js`; add the CI `browser` job (Node 24, `npm ci`, `npx playwright install --with-deps`, `npx playwright test`, gzip budget check).
- [ ] **Step 4: Run** `npx playwright test` (all three engines) and the Rust gate — expected PASS.
- [ ] **Step 5: Visual check:** serve `target/gallery`, open `palette.html` in the browser pane, confirm every preset is legible in light and dark, and take a screenshot for the review.
- [ ] **Step 6: Commit** `"Add runtime requirement loading and Playwright harness"`.
