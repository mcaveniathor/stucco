import { test, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

// Let navigation and theme changes finish painting before Axe caches computed
// styles. Wait for finite transitions; an indefinite busy spinner must not
// prevent auditing the page. Keep every accessibility rule enabled.
export async function audit(page: Page) {
  await page.evaluate(async () => {
    const paint = () => new Promise<void>(resolve => {
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
    });
    await document.fonts.ready;
    await paint();
    await Promise.all(document.getAnimations()
      .filter(animation => animation.effect && Number.isFinite(animation.effect.getComputedTiming().endTime))
      .map(animation => animation.finished.catch(() => {})));
    await paint();
  });
  const results = await new AxeBuilder({ page }).analyze();
  if (results.violations.length) {
    const info = test.info();
    await info.attach("axe-violations", {
      body: JSON.stringify(results.violations, null, 2),
      contentType: "application/json",
    });
    await info.attach("page-colors", {
      body: JSON.stringify(await page.evaluate(() => ({
        dark: matchMedia("(prefers-color-scheme: dark)").matches,
        forced: matchMedia("(forced-colors: active)").matches,
        elements: [document.documentElement, document.body, document.querySelector("h1")]
          .filter(el => el !== null).map(el => {
            const css = getComputedStyle(el);
            return {
              tag: el.tagName, color: css.color, background: css.backgroundColor,
              image: css.backgroundImage, opacity: css.opacity,
            };
          }),
      })), null, 2),
      contentType: "application/json",
    });
    await info.attach("page", { body: await page.screenshot(), contentType: "image/png" });
  }
  return results;
}
