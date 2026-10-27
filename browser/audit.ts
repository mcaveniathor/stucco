import { test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

// Let navigation and theme changes finish painting before Axe caches computed
// styles. WebKit can update inherited light-dark() colors after the root has
// already painted its new scheme, even without an authored CSS transition.
// Wait for observed style convergence, never for a passing contrast ratio.
export async function audit(page: Page) {
  const readiness = await page.evaluate(async () => {
    const paint = () => new Promise<void>(resolve => {
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
    });
    let stopped = false;
    let timeout: number | undefined;
    let samples = 0;
    let changes = 0;
    const colors = () => {
      // An indefinite animation must not hold up the audit. Its subtree is
      // excluded only from readiness waiting, and is still audited by Axe.
      const indefinite = document.getAnimations()
        .filter(animation => animation.effect && !Number.isFinite(animation.effect.getComputedTiming().endTime))
        .map(animation => animation.effect instanceof KeyframeEffect ? animation.effect.target : null)
        .filter((element): element is Element => element !== null);
      // Read ancestors before descendants to flush pending inherited styles.
      // Include text-fill because Axe uses it ahead of color in WebKit.
      return [...document.querySelectorAll("*")]
        .filter(element => !indefinite.some(target => target.contains(element)) && element.getClientRects().length > 0)
        .map(element => {
          const css = getComputedStyle(element);
          return { element, value: [css.color, css.backgroundColor, css.getPropertyValue("-webkit-text-fill-color")].join("|") };
        });
    };
    const settle = async () => {
      await document.fonts.ready;
      await paint();
      await Promise.all(document.getAnimations()
        .filter(animation => animation.effect && Number.isFinite(animation.effect.getComputedTiming().endTime))
        .map(animation => animation.finished.catch(() => {})));
      let previous = colors();
      let stable = 0;
      while (!stopped) {
        await paint();
        const current = colors();
        samples += 1;
        const unchanged = current.length === previous.length && current.every((sample, index) =>
          sample.element === previous[index].element && sample.value === previous[index].value,
        );
        if (unchanged) stable += 1;
        else { stable = 0; changes += 1; }
        if (stable === 2) return { samples, changes };
        previous = current;
      }
      throw new Error("Accessibility readiness was interrupted");
    };
    try {
      return await Promise.race([
        settle(),
        new Promise<never>((_, reject) => {
          timeout = window.setTimeout(() => reject(new Error("Page styles did not settle before the accessibility audit")), 5_000);
        }),
      ]);
    } finally {
      stopped = true;
      window.clearTimeout(timeout);
    }
  });
  const results = await new AxeBuilder({ page }).analyze();
  if (results.violations.length) {
    const info = test.info();
    await info.attach("axe-violations", {
      body: JSON.stringify(results.violations, null, 2),
      contentType: "application/json",
    });
    await info.attach("page-colors", {
      body: JSON.stringify(await page.evaluate(({ readiness, selectors }) => ({
        readiness,
        dark: matchMedia("(prefers-color-scheme: dark)").matches,
        forced: matchMedia("(forced-colors: active)").matches,
        elements: [...new Set([document.documentElement, document.body, document.querySelector("h1"),
          ...selectors.flatMap(selector => {
            try { return [...document.querySelectorAll(selector)]; }
            catch { return []; }
          }),
        ])]
          .filter(el => el !== null).map(el => {
            const css = getComputedStyle(el);
            return {
              tag: el.tagName, text: el.textContent?.slice(0, 100),
              color: css.color, textFill: css.getPropertyValue("-webkit-text-fill-color"), background: css.backgroundColor,
              image: css.backgroundImage, opacity: css.opacity,
            };
          }),
      }), {
        readiness,
        selectors: results.violations.flatMap(violation => violation.nodes.flatMap(node =>
          node.target.filter((selector): selector is string => typeof selector === "string"),
        )),
      }), null, 2),
      contentType: "application/json",
    });
    await info.attach("page", { body: await page.screenshot(), contentType: "image/png" });
  }
  return results;
}
