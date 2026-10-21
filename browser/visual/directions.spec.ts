import { directions } from "../directions";
import { test, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
const output = resolve("../target/visual-design/directions");
for (const width of [360, 768, 1440]) {
  test(`complete screens at ${width}px`, async ({ page }) => {
    test.setTimeout(120_000);
    await mkdir(output, { recursive: true });
    await page.setViewportSize({ width, height: 1000 });
    await page.emulateMedia({ reducedMotion: "reduce" });
    const manifest: object[] = [];
    for (const direction of directions) {
      for (const screen of ["orders", "editorial", "project"]) {
        await page.goto(`http://localhost:4173/showcase-${direction}-${screen}.html`);
        for (const scheme of ["light", "dark"]) {
          const scope = page.locator(`.st-theme-scope[data-theme="${scheme}"]`);
          await scope.screenshot({ path: resolve(output, `${direction}-${screen}-${scheme}-${width}.png`) });
          const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
          expect(overflow).toBe(false);
          manifest.push({ direction, screen, scheme, width, overflow });
        }
      }
    }
    await page.goto("http://localhost:4182/");
    await page.screenshot({ path: resolve(output, `landing-${width}.png`), fullPage: true });
    await page.goto("http://localhost:4182/playground.html?guided=prism:1:42");
    await expect(page.locator("#pg-label")).toContainText("variation 42");
    await page.locator("#pg-screen").selectOption("project");
    await page.screenshot({ path: resolve(output, `playground-${width}.png`), fullPage: true });
    await writeFile(resolve(output, `manifest-${width}.json`), JSON.stringify(manifest, null, 2));
  });
}
