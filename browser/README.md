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

Accessibility audits wait for loaded fonts, layout frames and finite transitions
before running Axe; scheme tests also wait for the requested media preference.
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
