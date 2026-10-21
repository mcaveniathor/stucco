# Visual design implementation and review

This completes the implementation described by the 21 October visual-design
plan. The opening baseline is recorded in `visual-design-baseline.md`; it is
retained as historical evidence, not the current status.

## Implemented scope

| Workstream | Result |
| --- | --- |
| Typography | Opt-in responsive display titles; bounded reading measure; system fallback; optional licensed Newsreader/DM Sans asset |
| Materials | Atelier warm rules, Precision compact alignment, Prism focal frost, Play heavy outlines/offset press; quiet tables and reading surfaces |
| Orders | Shared masthead, honest page metrics, identities/statuses, native secondary filters, narrow cards, bounded forms and existing mutation flows |
| Landing | Shared orders composition with four direction links enhanced in place; usable destinations without scripts |
| Complete screens | Orders, editorial reading and project overview in every direction and scheme |
| Data presentation | Semantic progress; finite-data sparkline with text equivalents; ordered activity timeline with timestamps and optional icons |
| Interaction | Existing motion tokens retained; ghost press flattened; disabled/busy hover and press suppressed; menu fade and immediate native dismissal; movement removed under reduced motion |
| Discovery | Versioned bounded generation, three lock groups, six local favorites, same-screen comparisons, scheme/narrow controls, complete recipe exports through shared Rust/CLI/WASM |

No dependency framework was added. Shared showcase recipes are workspace-only.
The theme engine and data components are public APIs. Legacy presets and seed
mapping tests are unchanged. Optional fonts remain outside default builds and
are separately documented in `examples/showcase/fonts/README.md`.

## Interaction inventory

| State/family | Behavior and evidence |
| --- | --- |
| Primary/secondary/danger buttons | Existing shadow compression and offset movement; disabled/busy states retain text and no longer react to hover/press |
| Ghost actions | Background feedback and focus outline; no inherited raised/offset movement |
| Links, tabs, pagination, navigation | Existing underline/background/current-page tokens and immediate keyboard focus; native link destinations preserved |
| Form controls and errors | Native inputs/selects; immediate focus ring; visible labels/hints/errors and linked error summary; no animation delays validation |
| Dialogs | Existing short vertical opening; native immediate close; repeated opening/Escape retains focus return |
| Menus | Token-duration opacity opening; existing runtime anchors to trigger and keeps popover inside viewport; immediate native dismissal |
| Busy | Label plus aria-busy; spinner becomes stationary dotted indicator under reduced motion |
| Success | Existing flash/status text persists after redirect, including without scripts; toast dismissal remains optional enhancement |

Existing fast/normal/slow timings remain: Smooth 120/200/320ms, Snappy
70/120/200ms, Gentle 200/320/500ms, Springy 160/260/420ms. No new motion token or
animation runtime was needed. Reduced motion removes button translation/scale
and overlay entrance translation, and the global reduced-motion rule suppresses
transitions. Closing does not wait for animation before restoring focus.

## Visual review

The direction matrix includes 72 captures: four directions × three complete
screens × two schemes × three widths. Captures live in
`target/visual-design/directions/`; manifests assert no page overflow. The
baseline/Atelier workflow additionally covers actual orders errors, empty
results, expanded filters, open menus and dialogs. Desktop, tablet and phone
captures were inspected for hierarchy, alignment, density, wrapping and surface
contrast. Review led to explicit recipe-link spacing and using the direction's
line-weight token for the table frame.

Optional font captures are in `target/visual-design/fonts/`. Both loaded and
CSP-blocked paths remain readable at 360px. Font programs are local data URLs,
with no external font requests. `font-display: optional` avoids a late swap;
the workflow checks layout shift below 0.1. These are local rendering checks,
not a claim about real-user network performance.

The browser suite checks 320px stress cases, 200% text and CSS zoom, native
forms, visible error/selection/busy states, keyboard dismissal, repeated opens,
and preference behavior. Reduced transparency is exercised through Chromium
CDP; Firefox/WebKit do not expose that emulation path here. Increased contrast,
forced colors and reduced motion are checked across all three engines. WebKit
preference changes are asynchronous, so assertions wait for the computed state.

## Validation evidence

- After the version correction, cargo-semver-checks 0.51.0 accepted the 0.4.0
  workspace against published 0.3.0. Workspace compilation, CLI tests and the
  all-feature workspace publish dry run passed with the updated dependency
  requirements. No crates were published.
- Formatting, workspace Clippy (`--all-targets -D warnings`) and full workspace
  tests passed. Focused integration tests passed after the final recipe asset fix.
- All-feature Rust documentation passed with warnings denied. The new data
  family passes its tests with only the `data` feature enabled.
- Rust 1.85 checked the workspace excluding the existing newer-MSRV storage
  crates, and separately checked the showcase with its optional fonts enabled.
- Full browser run: 406 passed, two explicit CDP-only skips. Three additional
  touch checks passed across Chromium, Firefox and WebKit.
- Dedicated workflow configuration: 54 passed.
- Final visual run: nine passed, covering 54 application states (108 full/viewport
  PNGs) plus the 72-image direction matrix and landing/playground captures.
  All 126 manifest entries report no horizontal page overflow.
- Optional font run: two passed, loaded and blocked, including layout-shift,
  fallback and network-request assertions.

Logs are `target/design-*.log`. The primary final browser log is
`target/design-browser-final.log`; workflow and capture logs have corresponding
`-final` names. Original gallery snapshots and legacy theme mapping tests were
not rewritten.

Measured local output, before HTTP transfer (gzip uses Python's default level):

| Asset | Raw bytes | Gzip bytes |
| --- | ---: | ---: |
| Site enhancement script | 17,022 | 5,105 |
| Shared theme WASM | 249,702 | 89,632 |
| Site bundle CSS, including gallery's named themes | 496,984 | 37,927 |
| Optional local font CSS, excluded by default | 127,288 | 96,336 |

Compatibility: public `spec::Base` and `SpecError` have new variants;
downstream exhaustive matches need corresponding arms. The workspace targets
0.4.0 to account for this break from published 0.3.0. JSON retains the color
maps and adds a recipe field. No remote font loading, backend chart dependency,
or change to legacy preset/seed behavior is introduced.

## Additional directions — 0.5.0

Grove, Relay and Marquee extend the registry to seven signatures. Each has
bounded version 1 variations and appears in the landing switcher, playground,
CLI exports and all three showcase compositions. Existing recipe mappings are
unchanged. The public enum additions and array length change require 0.5.0.

Visual inspection exposed body typography inherited from the outer page in
named scopes. Named `ThemeScope`s now apply their own font, size and leading;
scheme-only scopes retain inherited typography. A browser regression checks
Relay's monospace body and Grove's airy leading in both schemes, plus isolation
from the surrounding page.

- Workspace tests, affected-crate tests after the scope fix, formatting,
  Clippy and all-feature documentation checks pass.
- Rust 1.85.0 checks the workspace excluding the existing newer-MSRV storage
  crates. The semver checker accepts 0.5.0 against the `v0.4.0` tag.
- The final browser suite passes 442 tests across Chromium, Firefox and WebKit;
  the two existing CDP-only reduced-transparency checks remain skipped.
- The direction capture workflow passes at 360, 768 and 1440px: 126 scope
  captures, covering seven directions, three screens and both schemes, with
  zero horizontal page overflows. Representative new-direction captures were
  inspected for typography, wrapping, hierarchy and surface treatment.

Logs are `target/additional-directions-*.log`; captures and manifests are in
`target/visual-design/directions/`. These are local checks; release CI verifies the merged implementation.
