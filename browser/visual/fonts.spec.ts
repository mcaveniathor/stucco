import { test, expect } from "@playwright/test";
import { mkdir } from "node:fs/promises";
const output = "../target/visual-design/fonts";
for (const blocked of [false, true]) {
  test(`optional faces with fonts ${blocked ? "blocked" : "loaded"}`, async ({ page }) => {
    await mkdir(output, { recursive: true });
    await page.setViewportSize({ width: 360, height: 900 });
    const external: string[] = [];
    page.on("request", (request) => { if (request.url().startsWith("http") && !request.url().startsWith("http://localhost:4184/")) external.push(request.url()); });
    if (blocked) {
      await page.route("**/showcase-fonts.html", async (route) => {
        const response = await route.fetch();
        await route.fulfill({ response, body: (await response.text()).replace("<head>", '<head><meta http-equiv="Content-Security-Policy" content="font-src \'none\'">') });
      });
    }
    await page.addInitScript(() => {
      (window as any).fontLayoutShift = 0;
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries() as any[]) if (!entry.hadRecentInput) (window as any).fontLayoutShift += entry.value;
      }).observe({ type: "layout-shift", buffered: true });
    });
    await page.goto("/showcase-fonts.html");
    await page.evaluate(() => document.fonts.ready);
    expect(await page.evaluate(() => document.fonts.check('400 20px "Newsreader"'))).toBe(!blocked);
    expect(await page.evaluate(() => document.fonts.check('400 16px "DM Sans"'))).toBe(!blocked);
    expect(external).toEqual([]);
    expect(await page.evaluate(() => (window as any).fontLayoutShift)).toBeLessThan(0.1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(360);
    await page.locator(".showcase-fonts").screenshot({ path: `${output}/${blocked ? "fallback" : "loaded"}-360.png` });
  });
}
