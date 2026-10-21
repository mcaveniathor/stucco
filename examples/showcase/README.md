# Shared visual recipes

This unpublished workspace crate keeps the landing preview, gallery showcase
and orders example consistent. It depends only on Stucco presentation APIs.
HTTP routes, persistence, validation and queries remain in the orders example.

`atelier()` delegates to the published `Direction::Atelier.theme()` recipe without changing presets or seeded
generation. `src/lib.rs` supplies the masthead, page heading, identity cells,
status badges and page metrics. `css/showcase.css` is a registered, hashed asset
in the normal Stucco bundle. Copy these functions and the stylesheet into an
application and adapt its content; this crate is not a published API.

The static `ORDERS` fixture is illustrative. The live app supplies its own
records and derives every metric from the loaded page. No global totals or
trends are implied.

Run `cargo run -p gallery -- target/gallery` and serve that directory to inspect
`showcase.html` in both schemes. The site build includes the same page under
`gallery/`, plus a landing preview. See `docs/guide/atelier.md` for the copyable
theme recipe and `browser/README.md` for before/after captures.

`editorial_preview` and `project_preview` extend the same system to reading and
project tracking. `screen` selects among all three. Every composition declares
its stylesheet during rendering, including for inline asset delivery.
`Direction::{Atelier,Precision,Prism,Play,Grove,Relay,Marquee}` and `guided_v1(seed)` belong to the
published theme engine. The gallery has `showcase-{direction}-{screen}.html`
pages; the landing switcher and playground use these same compositions.

The optional `atelier-fonts` feature supplies licensed local Newsreader/DM Sans
faces. Use `with_atelier_fonts(view)` to apply the pairing and declare its asset
for inline delivery. See `fonts/README.md` for costs and fallback details. This
feature is deliberately off by default.
