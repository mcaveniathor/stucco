import { test, expect, type Page } from "@playwright/test";

async function insert(page: Page, urls: string[]) {
  await page.evaluate(async (urls) => {
    const htmls = await Promise.all(urls.map((u) => fetch(u).then((r) => r.text())));
    for (const h of htmls) document.getElementById("target")!.insertAdjacentHTML("beforeend", h);
  }, urls);
}

test("concurrent fragments load a new module once and both enhance", async ({ page }) => {
  const requests: string[] = [];
  page.on("request", (r) => {
    if (r.url().includes("/_stucco/probe.")) requests.push(r.url());
  });
  await page.goto("/fixtures/enhanced.html");
  await insert(page, ["/fixtures/probe-a.html", "/fixtures/probe-b.html"]);
  await expect(page.locator("st-probe[data-ready]")).toHaveCount(2);
  expect(requests).toHaveLength(1);
  await expect(page.locator("st-require")).toHaveCount(0);
});

test("a missing module marks the region as failed", async ({ page }) => {
  await page.goto("/fixtures/enhanced.html");
  const event = page.evaluate(
    () =>
      new Promise<string>((resolve) =>
        document.addEventListener(
          "stucco:asset-error",
          (e) => resolve((e as CustomEvent).detail.url),
          { once: true },
        ),
      ),
  );
  await page.evaluate(() =>
    document
      .getElementById("target")!
      .insertAdjacentHTML(
        "beforeend",
        '<st-require modules="/_stucco/missing.0000000000.js"></st-require>',
      ),
  );
  expect(await event).toContain("/_stucco/missing.0000000000.js");
  await expect(page.locator("#target")).toHaveAttribute("data-state", "error");
});

test("foreign and out-of-bundle URLs are never requested", async ({ page }) => {
  const bad: string[] = [];
  page.on("request", (r) => {
    if (r.url().includes("example.com") || r.url().includes("/evil.js")) bad.push(r.url());
  });
  await page.goto("/fixtures/enhanced.html");
  await page.evaluate(() =>
    document
      .getElementById("target")!
      .insertAdjacentHTML(
        "beforeend",
        '<st-require modules="https://example.com/x.js /evil.js"></st-require>',
      ),
  );
  await page.waitForTimeout(200);
  expect(bad).toHaveLength(0);
  await expect(page.locator("st-require")).toHaveCount(0);
});
