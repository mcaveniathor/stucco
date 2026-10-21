# Visual design baseline — 21 October 2026

Baseline source: `574e117`, before the visual-design changes. Captures are retained
under `target/visual-design/baseline/`; the repeatable workflow is documented in
`browser/README.md`. The original capture matrix has 42 full-page images:
seven states × three widths × two schemes. Later captures also include the
shared showcase, default orders and viewport images.

## Observed gaps

| Observation | Evidence | Existing capability / first change |
| --- | --- | --- |
| The desktop New order action fills the entire header; its edge does not follow the table's content edge. | `orders-light-1440.png` | Compose title and bounded action together inside the main column. |
| On a phone, filters consume nearly a screen before any records; amount and actions need horizontal table scrolling. | `orders-dark-360.png` | Enable existing DataTable card layout; add opt-in native secondary-filter disclosure. |
| Headline, section headings and card titles have little scale separation. | `site-light-1440.png`, `gallery-light-1440.png` | Add opt-in responsive display sizing, leaving body/forms/tables unchanged. |
| The landing page describes components but never shows an application. | `site-light-1440.png` | Share a real component composition between landing and gallery. |
| Form and list views lack a common masthead, and wide forms have no clear reading boundary. | `errors-light-768.png`, `orders-light-1440.png` | Shared masthead and bounded form surface. |
| Existing badges, error summaries and overlays communicate state well. | `dialog-dark-1440.png`, `errors-light-768.png` | Preserve behavior and accessible naming; compose a coordinated theme. |

## Foundation decisions

Sand already supplies a suitable warm neutral and terracotta palette. Combine
it with `Fonts::system`, serif headings, a fine navigation bar, quiet ruled
panels, sand finish, raised primary buttons and gentle motion. Theme validation
checks both schemes. Keep decorative relief and translucent materials out of
dense data regions. `ThemeScope` already supplies the bounded preview; no new
surface API is needed.

The default orders data remains unchanged to preserve storage and query
examples. The shared illustrative fixture adds long and accented names, amounts
from $74.25 to $12,840.00, and all three existing statuses. Filtered-empty and
validation-error states are captured through the real application. The baseline
manifests reported no horizontal *page* overflow; table scrolling was still
required on narrow screens.

## First Atelier pass

The live orders view now uses a shared masthead, display title, bounded primary
action, clearly page-scoped summary, identity cells and existing responsive
cards. Secondary filters use native details and stay in the same GET form.
Active ranges, sorting or a nondefault page size reopen the details after
navigation. Clearing filters preserves the pre-existing sorting behavior.

The site defaults to Atelier, retains preset/custom theme switching, and links
its default theme to the recipe. The landing and gallery share a static orders
preview with explicit illustrative labeling and real recipe links. Existing
presets and deterministic generation are unchanged. System fonts keep the
screen usable without downloading fonts.

At the end of this opening pass, optional bundled fonts,
all four art directions, direction switching, editorial/project showcases,
richer data presentations, the full interaction audit and guided discovery
were still outstanding. They are now implemented; see `visual-design-validation.md`
for the continuation and final evidence.

## Validation of this implementation

- `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace` passed.
- `npm test`: 363 passed across Chromium, Firefox and WebKit. After the header and preview
  spacing refinements, the affected site/orders/accessibility suites passed
  again (216 checks). The dedicated `npm run test:workflows` passed 54 checks.
- Rust 1.85 checked the standalone showcase crate and its selected component
  families with `--locked --offline`.
- Final captures: `target/visual-design/atelier-final/`, 54 states with full-page
  and viewport images (108 PNGs). The manifests report no horizontal page
  overflow. Reviewed representative desktop, tablet and phone views in both
  schemes, including validation, confirmation and filtered-empty states.
- New behavioral coverage exercises compact native filters with scripts on/off,
  retained sorting after clearing, long names and two maximum u64-cent orders.
  Their page sum is exact and the 320px layout has no page overflow.

The host needed the Rust WASM target installed. WebKit's missing Linux libraries
were downloaded and extracted under `/tmp/stucco-webkit-libs`; tests used
`LD_LIBRARY_PATH` for validation and absolute `LD_PRELOAD` paths for libevent,
libhidapi-hidraw and libmanette because the bundled WebKit launcher replaces its
library path. No system packages were installed. A normal development machine
should use Playwright's documented `install-deps` setup.

Preference, enlarged-text/CSS-zoom and optional font-loading checks were completed
in the continuation documented in `visual-design-validation.md`.
