import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.use({ baseURL: "http://localhost:4180" });

test("the page renders with its linked stylesheet and works without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false, baseURL: "http://localhost:4180" });
  const page = await context.newPage();
  await page.goto("/");
  const href = await page.locator('link[rel="stylesheet"]').getAttribute("href");
  expect(href).toMatch(/^\/_stucco\/stucco\.[0-9a-f]{10}\.css$/);
  await page.getByLabel("Name").fill("Ada <3");
  await page.getByRole("button", { name: "Greet" }).click();
  await expect(page.locator("#greeting")).toHaveText("Hello, Ada <3!");
  await context.close();
});

test("assets answer 200, 304, 404 and 405", async ({ request, page }) => {
  await page.goto("/");
  const href = (await page.locator('link[rel="stylesheet"]').getAttribute("href"))!;
  const ok = await request.get(href);
  expect(ok.status()).toBe(200);
  expect(ok.headers()["cache-control"]).toBe("public, max-age=31536000, immutable");
  expect((await request.get(href, { headers: { "if-none-match": ok.headers()["etag"] } })).status()).toBe(304);
  expect((await request.get("/_stucco/nope.css")).status()).toBe(404);
  expect((await request.post(href)).status()).toBe(405);
});

test("fragment requests get only the target, uncached, varying on the request kind", async ({ request }) => {
  const res = await request.get("/?q=Grace", {
    headers: { "Stucco-Request": "fragment", "Stucco-Target": "greeting" },
  });
  expect(res.headers()["vary"]).toBe("Stucco-Request, Stucco-Target");
  expect(res.headers()["cache-control"]).toBe("no-store");
  const html = await res.text();
  expect(html).toContain("Hello, Grace!");
  expect(html).not.toContain("<html");
});

test("the page has no axe violations", async ({ page }) => {
  await page.goto("/?q=Ada");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});
