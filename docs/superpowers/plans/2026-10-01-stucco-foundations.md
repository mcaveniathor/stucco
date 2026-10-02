# stucco Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the stucco workspace foundation: the rendering core (`Render`, `Cx`, element builder, `Href`, `Attrs`, assets, `Page`, `Bundle`), the theme engine with all 14 presets, the base CSS layers, and a static gallery skeleton with the palette page.

**Architecture:** A Cargo workspace. `stucco-theme` (no dependencies) turns OKLCH seeds into 12-step scales, semantic roles and token CSS, and checks contrast. `stucco-core` depends on `stucco-theme` and `inventory`; it renders components into a `Cx`, collects declared `Asset`s, and serves a content-hashed stylesheet and scripts through `Bundle`. `stucco` is the facade crate; `gallery` (unpublished) writes a static site to `target/gallery/`.

**Tech Stack:** Rust edition 2024 (MSRV 1.85), `inventory` 0.3, `insta` 1 (dev), GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-10-01-stucco-design.md` — this plan implements build-order steps 1 and 2, plus the base CSS layers of step 3 (needed by `Page`). Later plans cover: (2) icons, layout, typography; (3) forms; (4) feedback, navigation, overlay, behaviour runtime and the Playwright harness; (5) data display and DataTable; (6) shells and the theme switcher; (7) marketing; (8) diagrams; (9) adapters, examples, release.

**Deviations from the spec (update the spec in Task 1):**
- Dependency direction: `stucco-theme` has no dependencies and `stucco-core` depends on it (the spec's table had theme → core, which would cycle with `Bundle` taking a theme).
- `Bundle::new` takes `impl Into<BuiltTheme>` (implemented for `Preset`, `Theme`, `BuiltTheme`); the spec's `ThemeSource` name is dropped. `From<Theme>` panics with the contrast report if the theme fails; users who want to handle that call `.build()` first.
- Assets register themselves with `stucco_core::register_asset!` (via `inventory`), so `Bundle::new` includes every compiled-in component's CSS without manual lists.

## Global Constraints

- Edition 2024, `rust-version = "1.85"` in every crate; no let-chains or other post-1.85 syntax.
- License `MIT OR Apache-2.0`; `LICENSE-MIT` and `LICENSE-APACHE` at the root.
- `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` in every published crate.
- `stucco-theme`: zero dependencies. `stucco-core`: only `stucco-theme` and `inventory`.
- CSS class prefix `st-`; custom properties prefix `--st-`; cascade layers exactly `stucco.reset, stucco.tokens, stucco.base, stucco.layout, stucco.components, stucco.utilities`.
- Colour literals (`oklch(`, `#hex`, `rgb(`, `hsl(`) appear only in generated token CSS.
- Default asset URL prefix `/_stucco/`.
- `localStorage` key for the theme choice: `stucco-theme`.
- Commits: plain messages, no AI attribution lines.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass at the end of every task.

## Review Focus

1. **Obfuscated URL schemes** — `" JaVaScRiPt:alert(1)"`, `"java\tscript:x"`, `"\u{0}javascript:x"`, `"data:text/html,…"`, `"vbscript:x"` must all become invalid `Href`s rendering `#`; relative URLs containing `:` after a `/`, `?` or `#` (`"/a:b"`, `"?x=y:z"`) stay valid. Test in Task 3.
2. **Text inside raw-text elements** — `el::script().text("</script><b>")` and `el::style().text(..)` must stay escaped (safe, even if not useful); only `Raw::trusted` emits unescaped markup. Test in Task 4.
3. **Asset graphs with diamonds and cycles** — A→B, A→C, B→D, C→D resolves to D, B, C, A with D once; a cycle A→B→A terminates and includes each once. Test in Task 5.
4. **`Bundle::get` with odd paths** — query strings (`/_stucco/stucco.<hash>.css?v=1`), a missing prefix, `..` segments, a stale hash, and a custom prefix without trailing slash (`/assets`) behave predictably: query ignored, everything else unknown → `None`. Test in Task 9.
5. **Out-of-range theme inputs** — hue `-30.0` equals `330.0`, hue `725.0` equals `5.0`, chroma `0.5` (out of sRGB gamut) is gamut-mapped rather than clipped per channel, and `NaN` hue is treated as `0.0`. Test in Task 6.

---

### Task 1: Workspace scaffold and HTML escaping

**Files:**
- Create: `Cargo.toml` (workspace), `.gitignore`, `rustfmt.toml`, `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `.github/workflows/ci.yml`
- Create: `crates/stucco-theme/Cargo.toml`, `crates/stucco-theme/src/lib.rs`
- Create: `crates/stucco-core/Cargo.toml`, `crates/stucco-core/src/lib.rs`, `crates/stucco-core/src/escape.rs`
- Create: `crates/stucco/Cargo.toml`, `crates/stucco/src/lib.rs`
- Create: `gallery/Cargo.toml`, `gallery/src/main.rs`
- Modify: `docs/superpowers/specs/2026-10-01-stucco-design.md` (§3 table, §4.5 — the three deviations above)

**Interfaces:**
- Produces: `stucco_core::escape::{escape_text(s: &str, out: &mut String), escape_attr(s: &str, out: &mut String)}`.

- [ ] **Step 1: Create the workspace.** Root `Cargo.toml` with `members = ["crates/*", "gallery"]`, `resolver = "3"`, `[workspace.package]` (version `0.1.0`, edition `2024`, rust-version `1.85`, license, repository left empty), `[workspace.dependencies]` for the internal crates, `inventory = "0.3"`, `insta = "1"`. `gallery` has `publish = false`. CI workflow: jobs `fmt`, `clippy`, `test` (stable), `msrv` (`cargo +1.85 check --workspace`). `.gitignore`: `target/`, `node_modules/`.

- [ ] **Step 2: Write the failing test** in `escape.rs`:

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

- [ ] **Step 3: Run** `cargo test -p stucco-core escape` — expected FAIL (functions not defined).

- [ ] **Step 4: Implement** both functions in `escape.rs` (append to `out`; copy unescaped runs in slices, not per char).

- [ ] **Step 5: Run** `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` — expected PASS.

- [ ] **Step 6: Update the spec** with the three deviations, then commit.

```bash
git add -A && git commit -m "Scaffold workspace and HTML escaping"
```

---

### Task 2: Render, Cx, composition and Raw

**Files:**
- Create: `crates/stucco-core/src/render.rs`
- Modify: `crates/stucco-core/src/lib.rs` (re-exports)

**Interfaces:**
- Consumes: `escape_text`, `escape_attr` (Task 1).
- Produces:
  - `pub trait Render { fn render(&self, cx: &mut Cx); }`
  - `pub struct Cx` with `Cx::new() -> Cx`, `cx.id(&mut self, prefix: &str) -> String` (`"{prefix}-{n}"`, n starting at 1, one counter per prefix), `cx.text(&mut self, s: &str)` (escaped), `pub(crate) fn raw(&mut self, s: &str)`, `pub(crate) fn attr_value(&mut self, s: &str)`, `cx.finish(self) -> (String, Vec<&'static Asset>)` (assets: Task 5; return an empty Vec until then).
  - `Render` impls: `str`, `String`, `Cow<'_, str>`, `char`, all integer types, `f32`, `f64`, `bool` is **not** implemented, `Option<T>`, `Vec<T>`, `[T]`, `&T`, `Box<T>`, `Rc<T>`, `Arc<T>`, tuples arity 1–12.
  - `pub fn render_fn<F: Fn(&mut Cx)>(f: F) -> RenderFn<F>`; `pub type Slot = Box<dyn Render>`; `pub fn slot(r: impl Render + 'static) -> Slot`. Components accept `impl Render + 'static` in their setters and box internally (a blanket `From<T> for Box<dyn Render>` is impossible under the orphan rule).
  - `pub fn to_html(r: &(impl Render + ?Sized)) -> String`.
  - `pub struct Raw(String)` with `Raw::trusted(s: impl Into<String>) -> Raw`, implementing `Render` by writing unescaped.

- [ ] **Step 1: Write the failing tests** in `render.rs`:

```rust
#[test]
fn strings_render_escaped_and_compose() {
    let none: Option<&str> = None;
    let html = to_html(&("a<b", none, Some(3u8), vec!['&', 'x'], 1.5f64));
    assert_eq!(html, "a&lt;b3&amp;x1.5");
}

#[test]
fn raw_is_the_only_unescaped_path() {
    assert_eq!(to_html(&Raw::trusted("<b>x</b>")), "<b>x</b>");
    assert_eq!(to_html(&"<b>x</b>"), "&lt;b&gt;x&lt;/b&gt;");
}

#[test]
fn ids_are_unique_per_prefix_within_one_cx() {
    let mut cx = Cx::new();
    assert_eq!(cx.id("tabs"), "tabs-1");
    assert_eq!(cx.id("tabs"), "tabs-2");
    assert_eq!(cx.id("menu"), "menu-1");
    assert_eq!(Cx::new().id("tabs"), "tabs-1", "a new Cx restarts");
}

#[test]
fn closures_and_slots_render() {
    let s: Slot = slot(render_fn(|cx: &mut Cx| cx.text("<hi>")));
    assert_eq!(to_html(&s), "&lt;hi&gt;");
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core render` — expected FAIL.

- [ ] **Step 3: Implement** in `render.rs`. Floats use `Display` (`1.5`, not `1.50`); non-finite floats render as an empty string. Tuple impls via a `macro_rules!`.

- [ ] **Step 4: Run** `cargo test -p stucco-core` — expected PASS. Commit: `"Add Render, Cx and composition"`.

---

### Task 3: Href

**Files:**
- Create: `crates/stucco-core/src/href.rs`

**Interfaces:**
- Produces: `pub struct Href` with `Href::new(s: impl Into<String>) -> Href`, `Href::trusted(s: impl Into<String>) -> Href`, `Href::invalid() -> Href`, `href.is_valid() -> bool`, `href.as_str() -> &str` (returns `"#"` when invalid); `impl From<&str> for Href`, `impl From<String> for Href` (both via `new`).

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn safe_urls_are_kept() {
    for ok in ["/a", "a/b", "../x", "?q=1", "#top", "/a:b", "?x=y:z", "https://e.com",
               "HTTP://e.com", "mailto:a@b.c", "tel:+1", "//cdn.example/x.js"] {
        assert!(Href::new(ok).is_valid(), "{ok}");
        assert_eq!(Href::new(ok).as_str(), ok);
    }
}

#[test]
fn unsafe_schemes_become_hash() {
    for bad in ["javascript:alert(1)", " JaVaScRiPt:alert(1)", "java\tscript:x",
                "\u{0}javascript:x", "data:text/html,<b>", "vbscript:x", "file:///etc"] {
        let h = Href::new(bad);
        assert!(!h.is_valid(), "{bad:?}");
        assert_eq!(h.as_str(), "#");
    }
    assert_eq!(Href::trusted("javascript:void(0)").as_str(), "javascript:void(0)");
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core href` — expected FAIL.

- [ ] **Step 3: Implement.** Classification: strip leading ASCII whitespace and C0 controls, remove ASCII tab/newline/CR anywhere (as browsers do), then find the first of `:`, `/`, `?`, `#`. If it is `:`, the prefix is a scheme: allowed only if it case-insensitively equals `http`, `https`, `mailto` or `tel`. Otherwise the URL is relative and valid. Store the original string when valid.

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add Href with scheme allow-list"`.

---

### Task 4: Element builder and Attrs

**Files:**
- Create: `crates/stucco-core/src/el.rs` (builder types), `crates/stucco-core/src/el/tags.rs` (one fn per element), `crates/stucco-core/src/attrs.rs`

**Interfaces:**
- Consumes: `Render`, `Cx`, `Href`, escaping.
- Produces:
  - `pub struct Element` and `pub struct VoidElement`; module `el` with a lowercase fn per HTML element returning `Element` (normal) or `VoidElement` (`area base br col embed hr img input link meta source track wbr`). No HTML element name is a Rust keyword, so every function uses the plain tag name.
  - Shared methods on both (via a private `Base` struct both wrap): `.class(impl AsRef<str>)` (space-separated input allowed, appends, de-duplicates, order preserved), `.id(impl Into<String>)`, `.attr(name: &str, value: impl Into<String>)`, `.bool_attr(name: &str, on: bool)`, `.data(name: &str, value)` → `data-{name}`, `.aria(name: &str, value)` → `aria-{name}`, `.href(impl Into<Href>)`, `.src(impl Into<Href>)`, `.action(impl Into<Href>)`, `.attrs(&Attrs)` (merge).
  - `Element` only: `.child(impl Render + 'static)`, `.child_if(bool, impl Render + 'static)`, `.children(impl IntoIterator<Item = impl Render + 'static>)`, `.text(impl Into<String>)`.
  - `pub struct Attrs` (`Default`, `Clone`, `Debug`) with the same `.class/.id/.attr/.bool_attr/.data/.aria` methods; components store one and call `element.attrs(&self.attrs)`.
  - Attribute order in output: `id`, `class`, then insertion order. `.attr("href" | "src" | "action" | "formaction" | "poster", …)` routes through `Href::new`. Setting the same attribute twice keeps the last value (except `class`).

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
fn class_appends_and_dedupes_and_attrs_merge() {
    let attrs = Attrs::default().class("b c").attr("hx-get", "/more");
    let html = to_html(&el::div().class("a b").attrs(&attrs).bool_attr("hidden", true).bool_attr("inert", false));
    assert_eq!(html, r#"<div class="a b c" hx-get="/more" hidden></div>"#);
}

#[test]
fn void_elements_have_no_closing_tag_and_urls_are_checked() {
    assert_eq!(to_html(&el::img().src("javascript:x").attr("alt", "\"q\"")),
               r#"<img src="#" alt="&quot;q&quot;">"#);
    assert_eq!(to_html(&el::a().attr("href", "/ok").text("ok")), r#"<a href="/ok">ok</a>"#);
}

#[test]
fn raw_text_elements_stay_escaped() {
    assert_eq!(to_html(&el::script().text("</script><b>")), "<script>&lt;/script&gt;&lt;b&gt;</script>");
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "invalid attribute name")]
fn invalid_attribute_names_panic_in_debug() {
    let _ = el::div().attr("on click", "x");
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core el` — expected FAIL.

- [ ] **Step 3: Implement.** Valid attribute name: non-empty, ASCII alphanumerics plus `- _ : .`, first char a letter. Invalid: `debug_assert!(false, "invalid attribute name: {name}")`, then drop it. `tags.rs` is generated by a `macro_rules!` list of element names.

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add element builder and Attrs"`.

---

### Task 5: Assets and the behaviour vocabulary

**Files:**
- Create: `crates/stucco-core/src/asset.rs`, `crates/stucco-core/src/behavior.rs`
- Modify: `crates/stucco-core/src/render.rs` (`Cx::require`, `Cx::finish`)

**Interfaces:**
- Produces:
  - `pub struct Asset { pub name: &'static str, pub css: Option<&'static str>, pub behavior: Option<Behavior>, pub deps: &'static [&'static Asset] }`
  - `#[non_exhaustive] pub enum Behavior { Js(&'static str) }`
  - `cx.require(&mut self, asset: &'static Asset)`; `cx.finish()` returns assets resolved in dependency-first order (depth-first post-order over `deps` in declaration order, starting from requirements in first-require order; each asset once, identified by pointer; cycles terminate via a visiting set).
  - `pub struct AssetRef(pub &'static Asset)`; `inventory::collect!(AssetRef)`; `#[macro_export] macro_rules! register_asset { ($a:path) => { ::stucco_core::inventory::submit! { ::stucco_core::AssetRef(&$a) } } }` (re-export `inventory` as `#[doc(hidden)] pub use inventory`).
  - `pub fn registered_assets() -> Vec<&'static Asset>` sorted by `name` (stable bundle output).
  - `behavior` module: `pub const THEME_STORAGE_KEY: &str = "stucco-theme";`, `pub const THEME_ATTR: &str = "data-theme";`, `pub const NAMED_THEME_ATTR: &str = "data-st-theme";`.

- [ ] **Step 1: Write the failing tests** in `asset.rs`:

```rust
static D: Asset = Asset { name: "d", css: Some(".d{}"), behavior: None, deps: &[] };
static B: Asset = Asset { name: "b", css: None, behavior: None, deps: &[&D] };
static C: Asset = Asset { name: "c", css: None, behavior: None, deps: &[&D] };
static A: Asset = Asset { name: "a", css: None, behavior: None, deps: &[&B, &C] };
static X: Asset = Asset { name: "x", css: None, behavior: None, deps: &[&Y] };
static Y: Asset = Asset { name: "y", css: None, behavior: None, deps: &[&X] };

fn names(v: Vec<&'static Asset>) -> Vec<&'static str> { v.into_iter().map(|a| a.name).collect() }

#[test]
fn diamond_dependencies_resolve_once_in_dependency_order() {
    let mut cx = Cx::new();
    cx.require(&A);
    cx.require(&D);
    assert_eq!(names(cx.finish().1), ["d", "b", "c", "a"]);
}

#[test]
fn cycles_terminate() {
    let mut cx = Cx::new();
    cx.require(&X);
    assert_eq!(names(cx.finish().1), ["y", "x"]);
}

crate::register_asset!(D);

#[test]
fn registered_assets_are_discoverable() {
    assert!(registered_assets().iter().any(|a| a.name == "d"));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core asset` — expected FAIL.

- [ ] **Step 3: Implement** (the macro must work both inside `stucco-core` tests and from other crates; inside the crate use `$crate`).

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add asset declaration, resolution and registry"`.

---

### Task 6: Colour — OKLCH, gamut mapping, contrast

**Files:**
- Create: `crates/stucco-theme/src/color.rs`

**Interfaces:**
- Produces: `#[derive(Clone, Copy, Debug, PartialEq)] pub struct Color { pub l: f64, pub c: f64, pub h: f64 }`; `Color::oklch(l, c, h) -> Color` (clamps `l` to 0..=1, `c` to ≥0, normalises `h` into `[0, 360)`, `NaN` → 0); `color.to_srgb() -> [f64; 3]` (gamma-encoded, 0..=1, gamut-mapped); `color.css() -> String` formatted `oklch(L% C H)` with L as percent to 2 decimals, C and H to 4 and 2 decimals, using the **gamut-mapped** colour; `color.luminance() -> f64` (WCAG relative luminance of `to_srgb()`); `pub fn contrast(a: Color, b: Color) -> f64`.

- [ ] **Step 1: Write the failing tests:**

```rust
fn close(a: f64, b: f64) -> bool { (a - b).abs() < 0.01 }

#[test]
fn known_colours_convert() {
    let w = Color::oklch(1.0, 0.0, 0.0).to_srgb();
    assert!(w.iter().all(|v| close(*v, 1.0)));
    let red = Color::oklch(0.62796, 0.25768, 29.2339).to_srgb();
    assert!(close(red[0], 1.0) && close(red[1], 0.0) && close(red[2], 0.0), "{red:?}");
}

#[test]
fn contrast_matches_wcag_reference() {
    let black = Color::oklch(0.0, 0.0, 0.0);
    let white = Color::oklch(1.0, 0.0, 0.0);
    assert!(close(contrast(black, white), 21.0));
    assert!(close(contrast(white, black), 21.0));
}

#[test]
fn inputs_are_normalised() {
    assert_eq!(Color::oklch(0.5, 0.1, -30.0).h, 330.0);
    assert!(close(Color::oklch(0.5, 0.1, 725.0).h, 5.0));
    assert_eq!(Color::oklch(0.5, 0.1, f64::NAN).h, 0.0);
}

#[test]
fn out_of_gamut_chroma_is_reduced_preserving_hue_and_lightness() {
    let c = Color::oklch(0.7, 0.5, 145.0);
    let rgb = c.to_srgb();
    assert!(rgb.iter().all(|v| (0.0..=1.0).contains(v)));
    let mapped = c.gamut_mapped();
    assert!(close(mapped.l, 0.7) && close(mapped.h, 145.0) && mapped.c < 0.5);
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme color` — expected FAIL.

- [ ] **Step 3: Implement.** Conversion: OKLCH → OKLab → linear sRGB with Björn Ottosson's published matrices → sRGB transfer function. `pub fn gamut_mapped(self) -> Color`: if in gamut (each linear channel within `[-1e-4, 1+1e-4]`) return self; otherwise binary-search chroma in `[0, c]` for 20 iterations keeping the largest in-gamut chroma. `to_srgb` clamps the final channels to `[0, 1]`.

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add OKLCH colour, gamut mapping and contrast"`.

---

### Task 7: Scales

**Files:**
- Create: `crates/stucco-theme/src/scale.rs`

**Interfaces:**
- Consumes: `Color`.
- Produces: `pub struct Scale { pub light: [Color; 12], pub dark: [Color; 12] }`; `Scale::new(hue: f64, chroma: f64) -> Scale`; `scale.step(scheme: Scheme, n: usize) -> Color` (1-based, panics outside 1..=12); `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Scheme { Light, Dark }` with `Scheme::BOTH`.

Step semantics (documented on `Scale`): 1–2 app backgrounds, 3–5 component backgrounds, 6–8 borders, 9–10 solid fills, 11 low-contrast text, 12 high-contrast text.

Lightness and chroma curves (exact values):

| step | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| light L | .99 | .975 | .95 | .92 | .885 | .845 | .79 | .71 | .60 | .55 | .45 | .24 |
| dark L | .16 | .19 | .23 | .26 | .30 | .34 | .40 | .48 | .60 | .66 | .80 | .95 |
| chroma × | .10 | .15 | .25 | .35 | .45 | .55 | .65 | .80 | 1.0 | 1.0 | .85 | .45 |

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn steps_follow_the_lightness_curves() {
    let s = Scale::new(250.0, 0.15);
    assert_eq!(s.step(Scheme::Light, 1).l, 0.99);
    assert_eq!(s.step(Scheme::Dark, 12).l, 0.95);
    assert_eq!(s.step(Scheme::Light, 9).c, 0.15);
    assert!((s.step(Scheme::Light, 1).c - 0.015).abs() < 1e-9);
}

#[test]
fn text_steps_contrast_with_background_steps() {
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

- [ ] **Step 3: Implement** with the table above (colours stored un-mapped; mapping happens at `css()`/`to_srgb()`).

- [ ] **Step 4: Run** tests — expected PASS (if a hue fails the 11-on-2 check, lower light L11 / raise dark L11 by 0.02 until all pass, and update the table in this plan). Commit: `"Add 12-step OKLCH scales"`.

---

### Task 8: Theme builder, roles, contrast report and token CSS

**Files:**
- Create: `crates/stucco-theme/src/theme.rs` (builder + options), `crates/stucco-theme/src/roles.rs` (role table + contrast pairs), `crates/stucco-theme/src/css.rs` (token CSS)

**Interfaces:**
- Consumes: `Color`, `Scale`, `Scheme`, `contrast`.
- Produces:
  - `#[derive(Clone, Debug)] pub struct Theme`; `Theme::from_seed(hue: f64) -> Theme`; setters `.neutral_tint(chroma: f64)` (default 0.01, neutral hue = accent hue), `.neutral_hue(hue: f64)`, `.accent(Color)` (sets hue and chroma; default chroma 0.15), `.fonts(Fonts)`, `.type_scale(TypeScale)`, `.space(px: f64)` (default 4.0), `.radius(Radius)`, `.density(Density)`, `.min_contrast(ratio: f64)` (default 4.5; the Contrast preset uses 7.0), `.build(self) -> Result<BuiltTheme, ContrastReport>`.
  - `Fonts::system()` (sans: `system-ui, -apple-system, "Segoe UI", Roboto, sans-serif`; mono: `ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`), `.sans(&str)`, `.mono(&str)` (prepended to the stacks, quoted).
  - `TypeScale::new(base_px: f64, ratio: f64)` (default `16.0, 1.2`) → steps `--st-text-xs` (−2) … `--st-text-3xl` (+5), each `clamp(min, preferred, max)` where min = size × 0.9, max = size, preferred = `calc(min + (max−min) × (100vw − 360px) / 920)` written as `clamp(<min>rem, <a>rem + <b>vw, <max>rem)`.
  - `enum Radius { Sharp, Soft, Round }` → `--st-radius-sm/md/lg/full` = Sharp `0 0 0 0`(full stays `9999px`), Soft `4px 8px 12px`, Round `8px 14px 20px`.
  - `enum Density { Compact, Comfortable }` → `--st-control-h` 32px / 40px, `--st-pad-scale` 0.75 / 1.
  - Spacing `--st-space-1..12` = unit × [1,2,3,4,5,6,8,10,12,16,20,24]; motion `--st-duration-fast: 120ms; --st-duration: 200ms; --st-duration-slow: 320ms; --st-ease: cubic-bezier(.2,0,0,1)`; z `--st-z-dropdown: 100; --st-z-sticky: 200; --st-z-overlay: 300; --st-z-toast: 400`; shadows `--st-shadow-1..3` using `color-mix` of `--st-neutral-12`.
  - `pub struct BuiltTheme` with `built.css(scope: Scope<'_>) -> String`, `built.role(scheme, name: &str) -> Option<Color>`; `pub enum Scope<'a> { Root, Named(&'a str) }`.
  - `pub struct ContrastReport { pub failures: Vec<ContrastFailure> }` (`Display` lists one failure per line); `pub struct ContrastFailure { pub fg: &'static str, pub bg: &'static str, pub scheme: Scheme, pub ratio: f64, pub required: f64 }`.

Role table (`roles.rs`; scale step per role; "auto" = black or white, whichever contrasts more with `accent` in that scheme):

| role | source | role | source |
|---|---|---|---|
| `bg` | neutral 1 | `accent` | accent 9 |
| `surface` | neutral 2 | `accent-hover` | accent 10 |
| `surface-raised` | neutral 3 | `accent-text` | accent 11 |
| `hover` | neutral 4 | `accent-soft` | accent 3 |
| `text` | neutral 12 | `on-accent` | auto |
| `text-muted` | neutral 11 | `focus` | accent 8 |
| `border` | neutral 6 | `{s}` | {s} 9 |
| `border-strong` | neutral 8 | `{s}-text`, `{s}-soft` | {s} 11, {s} 3 |

where `{s}` ∈ success (hue 150), warning (hue 85), danger (hue 25), info (hue 240), each chroma 0.14.

Contrast pairs (required = `min_contrast` for text, 3.0 for UI): `text`/`bg`, `text`/`surface`, `text`/`surface-raised`, `text-muted`/`bg`, `text-muted`/`surface`, `accent-text`/`bg`, `accent-text`/`surface`, `on-accent`/`accent`, `{s}-text`/`bg`, `{s}-text`/`{s}-soft` (text); `border-strong`/`bg`, `focus`/`bg` (UI, 3.0).

Token CSS shape (`Scope::Root` → selector `:root`; `Scope::Named(n)` → `[data-st-theme="n"]`):

```css
@layer stucco.tokens {
  :root { color-scheme: light dark;
    --st-neutral-1: light-dark(oklch(…), oklch(…)); … (all 6 scales × 12)
    --st-bg: var(--st-neutral-1); … (all roles)
    --st-on-accent: light-dark(oklch(…), oklch(…));
    --st-font-sans: …; --st-text-base: …; --st-space-1: 4px; … }
  [data-theme="light"] { color-scheme: light; }
  [data-theme="dark"] { color-scheme: dark; }
}
```

The two `[data-theme]` rules are emitted only for `Scope::Root`.

- [ ] **Step 1: Write the failing tests** in `theme.rs`:

```rust
#[test]
fn default_seed_builds_and_meets_aa_in_both_schemes() {
    let built = Theme::from_seed(250.0).build().expect("valid");
    for scheme in Scheme::BOTH {
        let ratio = contrast(built.role(scheme, "text").unwrap(), built.role(scheme, "bg").unwrap());
        assert!(ratio >= 4.5);
    }
}

#[test]
fn an_impossible_minimum_reports_every_failing_pair() {
    let err = Theme::from_seed(250.0).min_contrast(30.0).build().unwrap_err();
    assert!(err.failures.iter().any(|f| f.fg == "text" && f.bg == "bg" && f.scheme == Scheme::Dark));
    assert!(err.to_string().contains("text on bg"));
}

#[test]
fn root_css_declares_layer_scales_roles_and_scheme_switches() {
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
    assert!(css.contains("[data-st-theme=\"brand\"] {"));
    assert!(!css.contains("[data-theme="));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme theme` — expected FAIL.

- [ ] **Step 3: Implement** `theme.rs`, `roles.rs`, `css.rs` per the tables above.

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add theme builder, roles, contrast report and token CSS"`.

---

### Task 9: Presets

**Files:**
- Create: `crates/stucco-theme/src/presets.rs`

**Interfaces:**
- Consumes: `Theme` and its setters.
- Produces: `#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Preset { Slate, Graphite, Stone, Sand, Sepia, Pine, Fjord, Ocean, Iris, Rose, Ember, Sol, Terminal, Contrast }`; `Preset::ALL: [Preset; 14]`; `preset.name() -> &'static str` (lowercase); `Theme::preset(p: Preset) -> Theme`; `impl From<Preset> for BuiltTheme` (`build().expect(…)`, guaranteed by the test below); `impl From<Theme> for BuiltTheme` (panics with the report's `Display`).

Starting values (tune in OKLCH during implementation if a contrast test fails; record final values in the file):

| preset | accent hue / chroma | neutral hue / tint | other |
|---|---|---|---|
| Slate | 250 / .15 | 250 / .012 | — |
| Graphite | 0 / 0 | 0 / 0 | accent = neutral (accent role maps to neutral 12/11) |
| Stone | 70 / .14 | 60 / .010 | Radius::Round |
| Sand | 40 / .12 | 75 / .020 | Fonts sans `"Iowan Old Style", Georgia, serif` |
| Sepia | 55 / .09 | 70 / .030 | Fonts as Sand |
| Pine | 160 / .11 | 165 / .015 | — |
| Fjord | 230 / .08 | 225 / .018 | — |
| Ocean | 205 / .13 | 215 / .012 | Radius::Sharp |
| Iris | 290 / .17 | 290 / .014 | — |
| Rose | 355 / .14 | 350 / .012 | Radius::Round |
| Ember | 35 / .19 | 40 / .008 | Radius::Sharp |
| Sol | 90 / .15 | 85 / .020 | — |
| Terminal | 145 / .18 | 145 / .010 | Fonts sans = mono stack, Radius::Sharp |
| Contrast | 255 / .20 | 0 / 0 | `min_contrast(7.0)`, light L12 .10, dark L12 1.0 |

Graphite and Contrast need two small hooks on `Theme`: `.accent_from_neutral()` and `.text_lightness(light: f64, dark: f64)` (overrides step 12 L of the neutral scale only). Add them to `theme.rs` with doc comments.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test]
fn every_preset_builds_in_both_schemes() {
    for p in Preset::ALL {
        if let Err(report) = Theme::preset(p).build() {
            panic!("{} fails contrast:\n{report}", p.name());
        }
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
fn presets_are_customisable() {
    let css = Theme::preset(Preset::Sand).radius(Radius::Sharp).build().unwrap().css(Scope::Root);
    assert!(css.contains("--st-radius-md: 0;"));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-theme presets` — expected FAIL.

- [ ] **Step 3: Implement** the presets and the two hooks.

- [ ] **Step 4: Run** `cargo test -p stucco-theme` — expected PASS. Commit: `"Add the 14 theme presets"`.

---

### Task 10: Base CSS layers and CSS rule tests

**Files:**
- Create: `crates/stucco-core/css/layers.css`, `crates/stucco-core/css/reset.css`, `crates/stucco-core/css/base.css`
- Create: `crates/stucco-core/src/base_css.rs`
- Create: `crates/stucco-core/tests/css_rules.rs`

**Interfaces:**
- Produces: `pub const LAYERS_CSS`, `pub const RESET_CSS`, `pub const BASE_CSS` (`include_str!`), and `pub fn check_component_css(name: &str, css: &str) -> Result<(), String>` (used by every later plan's tests on each `Asset::css`).

Content:
- `layers.css`: exactly the one `@layer` order statement from Global Constraints.
- `reset.css` (in `@layer stucco.reset`): box-sizing border-box, margin reset, `img, svg, video { display:block; max-inline-size:100% }`, inherited fonts on form controls, `text-wrap: pretty` on paragraphs, `text-wrap: balance` on headings.
- `base.css` (in `@layer stucco.base`): `body` uses `--st-bg`, `--st-text`, `--st-font-sans`, `--st-text-base`, line-height 1.5; `:focus-visible { outline: 2px solid var(--st-focus); outline-offset: 2px }`; `.st-sr-only`; `a` uses `--st-accent-text`; `code, kbd, pre` use `--st-font-mono`; `@media (prefers-reduced-motion: reduce)` sets `animation-duration`, `transition-duration` to `0.01ms` and `scroll-behavior: auto` on `*, ::before, ::after`.

`check_component_css` rules (spec §5.4): (1) no `oklch(`, `rgb(`, `rgba(`, `hsl(`, `hsla(`, or `#` followed by 3/4/6/8 hex digits and a non-identifier character; (2) every `var(--st-…)` reference names a role, type, space, radius, density, motion, z, shadow or font token — never `--st-<palette>-<n>` where palette ∈ neutral, accent, success, warning, danger, info; (3) the stylesheet's top-level content is entirely inside `@layer stucco.<layer> { … }` blocks.

- [ ] **Step 1: Write the failing tests** in `tests/css_rules.rs`:

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

- [ ] **Step 3: Implement** the CSS files and the checker (brace-depth scan for rule 3; no CSS parser dependency).

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add base CSS layers and component CSS rules"`.

---

### Task 11: Bundle

**Files:**
- Create: `crates/stucco-core/src/bundle.rs`, `crates/stucco-core/src/hash.rs`

**Interfaces:**
- Consumes: `BuiltTheme`, `Scope` (theme), `registered_assets`, `Behavior`, base CSS constants.
- Produces:
  - `pub struct Bundle`; `Bundle::new(theme: impl Into<BuiltTheme>) -> Bundle`; `.with_theme(name: &str, theme: impl Into<BuiltTheme>) -> Bundle`; `.prefix(p: &str) -> Bundle` (normalised to leading and trailing `/`); `bundle.stylesheet_url() -> &str`; `bundle.script_url(asset: &Asset) -> Option<&str>`; `bundle.css() -> &str`; `bundle.get(path: &str) -> Option<AssetFile>`.
  - `#[derive(Clone, Debug)] pub struct AssetFile { pub bytes: Arc<[u8]>, pub mime: &'static str, pub etag: String, pub immutable: bool }`.
  - `hash.rs`: `pub(crate) fn fnv1a64(bytes: &[u8]) -> u64`; file names use the first 10 lowercase hex digits: `stucco.<hash>.css`, `<asset-name>.<hash>.js`.
- Stylesheet content order: layers, reset, root theme tokens, each named theme's tokens (in `with_theme` call order), base, then every registered asset's `css` (sorted by name). Files are built eagerly in `new`; `with_theme` and `prefix` rebuild them.
- `script_url` returns `None` for assets that were not registered with `register_asset!` (Page inlines those; Task 12).
- `get`: strip any `?…` query; the remainder must start with the prefix; the rest must exactly equal a known file name; anything else → `None`. MIME `text/css; charset=utf-8` and `text/javascript; charset=utf-8`; etag `"\"<hash>\""`; `immutable: true`.

- [ ] **Step 1: Write the failing tests** in `bundle.rs`:

```rust
use stucco_theme::Preset;

#[test]
fn stylesheet_contains_layers_tokens_named_themes_and_base_in_order() {
    let b = Bundle::new(Preset::Slate).with_theme("brand", Preset::Iris);
    let css = b.css();
    let at = |s: &str| css.find(s).unwrap_or_else(|| panic!("missing {s}"));
    assert!(at("@layer stucco.reset, stucco.tokens") < at(":root { color-scheme"));
    assert!(at(":root { color-scheme") < at("[data-st-theme=\"brand\"]"));
    assert!(at("[data-st-theme=\"brand\"]") < at("@layer stucco.base"));
}

#[test]
fn get_serves_hashed_files_and_ignores_queries() {
    let b = Bundle::new(Preset::Slate);
    let url = b.stylesheet_url().to_owned();
    assert!(url.starts_with("/_stucco/stucco.") && url.ends_with(".css"));
    let f = b.get(&url).unwrap();
    assert_eq!(f.mime, "text/css; charset=utf-8");
    assert!(f.immutable);
    assert_eq!(&*f.bytes, b.css().as_bytes());
    assert!(b.get(&format!("{url}?v=1")).is_some());
}

#[test]
fn get_rejects_unknown_paths() {
    let b = Bundle::new(Preset::Slate).prefix("/assets");
    let name = b.stylesheet_url().rsplit('/').next().unwrap().to_owned();
    assert!(b.stylesheet_url().starts_with("/assets/"));
    for bad in [name.clone(), format!("/_stucco/{name}"), format!("/assets/../{name}"),
                "/assets/stucco.0000000000.css".to_owned(), "/assets/".to_owned()] {
        assert!(b.get(&bad).is_none(), "{bad}");
    }
}

#[test]
fn a_theme_change_changes_the_url() {
    assert_ne!(Bundle::new(Preset::Slate).stylesheet_url(), Bundle::new(Preset::Iris).stylesheet_url());
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core bundle` — expected FAIL.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run** tests — expected PASS. Commit: `"Add Bundle with hashed asset serving"`.

---

### Task 12: Page

**Files:**
- Create: `crates/stucco-core/src/page.rs`, `crates/stucco-core/src/theme_init.js`

**Interfaces:**
- Consumes: `Bundle`, `Cx`, `Render`, `Href`, `behavior` constants, escaping.
- Produces:
  - `pub struct Page<'b>`; `Page::new(bundle: &'b Bundle, title: impl Into<String>) -> Page<'b>`; `.lang(&str)` (default `"en"`), `.meta(Meta)`, `.head(impl Render + 'static)`, `.body(impl Render + 'static)`, `.body_attrs(Attrs)`, `.csp_nonce(impl Into<String>)`, `.delivery(Delivery)`, `.render(self) -> String`.
  - `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum Delivery { #[default] Linked, Inline }`.
  - `#[derive(Clone, Debug, Default)] pub struct Meta`; `Meta::description(&str) -> Meta`, `.og_title(&str)`, `.og_image(impl Into<Href>)`, `.canonical(impl Into<Href>)`.
- `theme_init.js` (inlined, minimal): reads `localStorage[THEME_STORAGE_KEY]` inside `try`; if `"light"` or `"dark"`, sets `document.documentElement.dataset.theme`. The storage key is substituted from `behavior::THEME_STORAGE_KEY` at render time (the file contains the placeholder `__STUCCO_THEME_KEY__`).
- A required asset whose behaviour has no bundle URL (not registered) is emitted as an inline module script in both delivery modes.
- Output order: `<!doctype html><html lang><head>` meta charset, viewport (`width=device-width, initial-scale=1`), `<title>`, meta tags, theme init script, stylesheet (`<link rel="stylesheet">` for Linked; `<style>` with layers + reset + root theme tokens + named themes + base + **only the required assets' CSS** for Inline), user head, then for each required asset with `Behavior::Js`: `<script type="module" src=…>` (Linked) or inline `<script type="module">` (Inline), then `</head><body …>`body`</body></html>`. Every inline `<script>`/`<style>` gets `nonce="…"` when set.

- [ ] **Step 1: Write the failing tests** in `page.rs`:

```rust
use stucco_theme::Preset;

static WIDGET: Asset = Asset { name: "widget", css: Some("@layer stucco.components { .st-widget { color: var(--st-text); } }"),
    behavior: Some(Behavior::Js("customElements.define('st-widget', class extends HTMLElement {});")), deps: &[] };

crate::register_asset!(WIDGET);

struct Widget;
impl Render for Widget {
    fn render(&self, cx: &mut Cx) { cx.require(&WIDGET); el::div().class("st-widget").render(cx); }
}

#[test]
fn linked_page_has_head_essentials_and_used_scripts_only() {
    let b = Bundle::new(Preset::Slate);
    let html = Page::new(&b, "A <b> title").meta(Meta::description("d")).body(Widget).render();
    assert!(html.starts_with("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">"));
    assert!(html.contains("<title>A &lt;b&gt; title</title>"));
    assert!(html.contains(&format!("<link rel=\"stylesheet\" href=\"{}\">", b.stylesheet_url())));
    assert!(html.contains(&format!("<script type=\"module\" src=\"{}\"></script>", b.script_url(&WIDGET).unwrap())));
    assert!(html.contains("<body><div class=\"st-widget\"></div></body></html>"));
    let without = Page::new(&b, "t").body("plain").render();
    assert!(!without.contains("type=\"module\""));
}

#[test]
fn inline_page_includes_only_used_css_and_nonces_everything() {
    let b = Bundle::new(Preset::Slate);
    let html = Page::new(&b, "t").delivery(Delivery::Inline).csp_nonce("n0nce").body(Widget).render();
    assert!(html.contains(".st-widget"));
    assert!(!html.contains("<link rel=\"stylesheet\""));
    let inline_tags = html.matches("<script").count() + html.matches("<style").count();
    assert_eq!(html.matches("nonce=\"n0nce\"").count(), inline_tags);
    assert!(html.contains("localStorage") && html.contains("stucco-theme"));
}

static LOOSE: Asset = Asset { name: "loose", css: None, behavior: Some(Behavior::Js("/*loose*/")), deps: &[] };
struct Loose;
impl Render for Loose { fn render(&self, cx: &mut Cx) { cx.require(&LOOSE); } }

#[test]
fn unregistered_behaviours_are_inlined_even_when_linked() {
    let b = Bundle::new(Preset::Slate);
    let html = Page::new(&b, "t").body(Loose).render();
    assert!(html.contains("<script type=\"module\">/*loose*/</script>"));
}
```

- [ ] **Step 2: Run** `cargo test -p stucco-core page` — expected FAIL.

- [ ] **Step 3: Implement.** The body renders first into a fresh `Cx`; the head is written afterwards from `cx.finish()`.

- [ ] **Step 4: Run** `cargo test -p stucco-core` — expected PASS. Commit: `"Add Page with linked and inline delivery"`.

---

### Task 13: Facade and gallery skeleton with the palette page

**Files:**
- Modify: `crates/stucco/src/lib.rs`, `crates/stucco/Cargo.toml`
- Modify: `gallery/src/main.rs`; Create: `gallery/src/palette.rs`, `gallery/src/index.rs`
- Create: `gallery/tests/snapshots.rs` (+ `insta` snapshots)
- Create: `examples/hello/` is **not** in this plan (examples are plan 9).

**Interfaces:**
- Consumes: everything above.
- Produces:
  - `stucco` re-exports: `stucco::{Render, Cx, Slot, slot, render_fn, to_html, Raw, Href, Attrs, el, Asset, Behavior, Bundle, AssetFile, Page, Meta, Delivery, register_asset}` and `stucco::theme::{Theme, Preset, Color, Scheme, Scope, Radius, Density, Fonts, TypeScale, BuiltTheme, ContrastReport}`; `stucco::prelude` re-exports `Render, Cx, el, Page, Bundle, Theme, Preset`. Facade feature list from spec §3 declared now as empty features (filled by later plans).
  - Gallery binary: `cargo run -p gallery -- [out_dir]` (default `target/gallery`) writes `index.html`, `palette.html` and every `Bundle` file under `_stucco/`. Pages use `Delivery::Linked` with relative prefix `_stucco/`.
  - `palette.rs`: `pub fn palette_page(bundle: &Bundle) -> String`. The bundle has every preset as a named theme (`with_theme(preset.name(), preset)`). For each preset: a `<section data-st-theme="<name>">` containing two panels (`data-theme="light"` and `data-theme="dark"`), each showing the 6 scales as 12 swatches (inline style `background: var(--st-<scale>-<n>)` — allowed: references, not literals) and a role sample (text on bg, muted text, accent button, the four status colours, border). Gallery CSS lives in an unregistered `Asset` named `gallery` that obeys `check_component_css`.

- [ ] **Step 1: Write the failing tests** in `gallery/tests/snapshots.rs`:

```rust
#[test]
fn palette_page_shows_every_preset_in_both_schemes() {
    let bundle = gallery::bundle();
    let html = gallery::palette::palette_page(&bundle);
    for p in stucco::theme::Preset::ALL {
        assert!(html.contains(&format!("data-st-theme=\"{}\"", p.name())), "{}", p.name());
    }
    assert_eq!(html.matches("data-theme=\"dark\"").count(), 14);
    insta::assert_snapshot!(html);
}

#[test]
fn site_writes_pages_and_assets() {
    let dir = std::env::temp_dir().join(format!("stucco-gallery-{}", std::process::id()));
    gallery::write_site(&dir).unwrap();
    for f in ["index.html", "palette.html"] { assert!(dir.join(f).exists(), "{f}"); }
    let css = std::fs::read_dir(dir.join("_stucco")).unwrap().count();
    assert!(css >= 1);
    std::fs::remove_dir_all(dir).unwrap();
}
```

(`gallery` gets a `src/lib.rs` exposing `bundle() -> Bundle`, `write_site(dir: &Path) -> io::Result<()>`, `pub mod palette`, `pub mod index`; `main.rs` calls `write_site`.)

- [ ] **Step 2: Run** `cargo test -p gallery` — expected FAIL.

- [ ] **Step 3: Implement** the facade re-exports and the gallery. Run `cargo insta accept` after reviewing the snapshot once by eye.

- [ ] **Step 4: Visual check.** Run `cargo run -p gallery`, then open `target/gallery/palette.html` in the browser pane: every preset shows distinct, legible light and dark panels; screenshot for the review.

- [ ] **Step 5: Run the full gate** `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo +1.85 check --workspace` — expected PASS (if 1.85 is not installed: `rustup toolchain install 1.85 --profile minimal`). Commit: `"Add facade re-exports and gallery palette page"`.
