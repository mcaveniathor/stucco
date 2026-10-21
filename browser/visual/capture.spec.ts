import { test, expect } from "@playwright/test";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

const label = process.env.VISUAL_LABEL ?? "current";
if (!/^[a-z0-9-]+$/.test(label)) throw new Error("VISUAL_LABEL must use lowercase letters, digits and hyphens");
const output = resolve("../target/visual-design", label);
for (const scheme of ["light", "dark"] as const) {
  for (const width of [360, 768, 1440]) {
    test(`${scheme} ${width}`, async ({ page }) => {
      await mkdir(output, { recursive: true });
      await page.setViewportSize({ width, height: 1000 });
      await page.emulateMedia({ colorScheme: scheme, reducedMotion: "reduce" });
      const captures: object[] = [];
      const capture = async (name: string) => {
        await page.evaluate(() => document.fonts.ready);
        const file = `${name}-${scheme}-${width}.png`;
        await page.screenshot({ path: resolve(output, file), fullPage: true });
        await page.screenshot({ path: resolve(output, `viewport-${file}`) });
        captures.push({ file, url: page.url(), width, scheme, overflow: await page.evaluate(() => document.documentElement.scrollWidth > innerWidth) });
      };
      await page.goto("http://localhost:4182/");
      await capture("site");
      await page.goto("http://localhost:4173/");
      await capture("gallery");
      await page.goto("http://localhost:4173/showcase.html");
      await capture("showcase");
      await page.goto("http://localhost:4181/orders");
      await capture("orders-default");
      await page.goto("http://localhost:4181/orders?per=10");
      await capture("orders");
      const row = page.locator("tbody tr").first();
      await row.getByRole("button", { name: /^Actions/ }).click();
      await capture("menu");
      await row.getByRole("link", { name: "Delete…" }).click();
      await expect(page.getByRole("dialog")).toBeVisible();
      await capture("dialog");
      await page.keyboard.press("Escape");
      await page.goto("http://localhost:4181/orders?q=DoesNotExist");
      await capture("empty");
      await page.goto("http://localhost:4181/orders/new");
      await page.getByLabel("Customer").fill("Alexandra Montgomery-Sutherland & Partners");
      await page.getByLabel("Status").selectOption("paid");
      await page.getByLabel("Total").fill("abc");
      await page.getByLabel("Created").fill("2026-10-01");
      await page.getByRole("button", { name: "Create order" }).click();
      await expect(page.getByRole("group", { name: "There is a problem" })).toBeVisible();
      await capture("errors");
      await writeFile(resolve(output, `manifest-${scheme}-${width}.json`), JSON.stringify(captures, null, 2));
    });
  }
}
