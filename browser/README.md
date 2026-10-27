# Browser checks and visual review

From this directory:

```sh
npm ci
npx playwright install chromium firefox webkit
rustup target add wasm32-unknown-unknown
npm test
npm run test:workflows
```

Playwright starts the repository's orders, gallery, site (including WASM) and
hello workflows. Install the Playwright host dependencies when a browser reports
missing shared libraries. Local HTTP server ports must be available.

The opt-in Leptos application suite uses the built full-stack example on port 4183:

```sh
# From the repository root; requires cargo-leptos 0.3.11 and the WASM target.
cargo leptos build -p orders-fullstack --cargo-locked
cd browser
npx playwright test --config playwright.fullstack.config.ts
```

Run the cargo-leptos build after ordinary Cargo tests, immediately before this
suite, so the server binary matches its hydration assets. The application requires
Rust 1.90 and uses embedded redb NoSQL storage by default; SQLite is an optional
server feature.

The suite provisions an isolated database and test accounts before starting the
server, then runs serially. It checks native and hydrated CRUD, URL search and
pagination, browser history, draft retention, stale revisions, delayed and lost
responses, account isolation, hydration, keyboard focus, accessibility and layouts.
It also covers native and hydrated attachments, private CSV exports, delayed export
navigation, and the component gallery's dialogs, repeated fields, suggestions, file
metadata and English/Arabic RTL layouts. Dedicated export accounts for each browser
keep quota and idempotency scenarios isolated. Recovery tests create and remove
disposable records through the application. Workflow screenshots are retained with
the test results. See [the example setup](../examples/orders-fullstack/README.md)
for provisioning, backup and maintenance prerequisites.

To test the copied optimized runtime, follow the
[deployment build/package instructions](../examples/orders-fullstack/DEPLOYING.md),
then run from this directory:

```sh
STUCCO_E2E_BUNDLE=../target/orders-release npx playwright test --config playwright.fullstack.config.ts
```

The harness runs that executable from its bundle directory and serves its copied
assets, using a new disposable database. Step 5 passes **210 cases**, **70 each in
Chromium, Firefox and WebKit**, against that release bundle. The added CSP check
verifies hydration, a saved mutation and reload without policy violations, then
confirms that an untrusted inline script is blocked. The release attachment and
Arabic RTL narrow-dark screenshots were visually inspected in WebKit.

At the Step 4 checkpoint, all **207 browser cases pass**: **69 each in Chromium,
Firefox and WebKit**, including accessibility and overflow checks. Selected
attachment, export, dialog and English/Arabic RTL gallery screenshots were visually
inspected in wide light and narrow dark layouts across the three browsers. Step 3's
144 cases and earlier Step 2 counts describe their historical checkpoints.

Native workflows run with JavaScript disabled and check form behavior and semantics.
Accessibility audits on enhanced pages wait for loaded fonts, layout frames and
finite transitions before running Axe; scheme tests also wait for the requested
media preference.
Indefinite busy animations keep running. All rules remain enabled. On a violation,
the test attaches Axe's findings, computed page colors and a viewport screenshot;
failed CI runs retain `browser/test-results/` as the `browser-test-results`
artifact for seven days.

## Repeatable visual captures

```sh
VISUAL_LABEL=before npx playwright test --config playwright.visual.config.ts
# Make the intended change, then capture under a different label.
VISUAL_LABEL=after npx playwright test --config playwright.visual.config.ts
```

The opt-in configuration uses Chromium, one worker and a fresh
`target/orders-visual.redb`. It does not reuse running servers, preventing stale
builds and mutated browser-test data from contaminating captures. Ports 4173,
4180, 4181 and 4182 must be free. It never resets the normal orders database.

Images and per-viewport JSON manifests live in `target/visual-design/<label>/`.
The label defaults to `current`; rerunning a label replaces its captures, so
use separate labels to retain before/after evidence. The directory follows the
repository's existing ignored `target` artifact convention.

Captures cover 360, 768 and 1440 CSS pixels, light and dark, with reduced motion
for repeatability. Each state has a full-page image and a `viewport-` image for
review at readable scale. States include the landing page, gallery, shared
showcase, default orders, ten-row orders with expanded secondary controls, an
open row menu, an open delete dialog, no results, and a server validation error.
The error is submitted through the real form and creates no record. The
showcase includes long customer names, accented text and large amounts.

Manifests record the URL, viewport, scheme and horizontal overflow. They are
observations, not automatic approval of the image. Inspect typography, alignment,
density, colors, focus and content wrapping before accepting changes. These
captures do not replace keyboard, accessibility, no-JavaScript or cross-browser
behavioral checks.

## Signature screens and optional fonts

The visual configuration also captures all seven directions × three screens ×
both schemes at 360/768/1440px. These 126 scope images and manifests are written
to `target/visual-design/directions/`; landing/playground captures accompany them.
Run only this matrix with:

```sh
npx playwright test --config playwright.visual.config.ts directions.spec.ts
```

The optional font workflow uses its own gallery output and port 4184:

```sh
npx playwright test --config playwright.fonts.config.ts
```

It checks local faces with fonts loaded and blocked by CSP, no external
requests, layout shift below 0.1, and a 360px fallback layout. Captures are under
`target/visual-design/fonts/`. The feature is off in normal builds.

`tests/discovery.spec.ts` covers guided locks, saved comparisons, recipe/JSON
round trips, unavailable storage, invalid seeds, no-script previews, all seven
signatures at 320px, form errors, action states, repeated overlay interruption,
touch, enlarged text, CSS zoom, and preferences. Reduced transparency uses
Chromium CDP; that one emulation check is explicitly skipped on Firefox/WebKit.
Contrast, reduced motion, forced colors and functional checks run in all three.
