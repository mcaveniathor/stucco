import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test.use({ baseURL: "http://localhost:4181" });

test("GET filters work without JavaScript", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false, baseURL: "http://localhost:4181" });
  try {
    const page = await context.newPage();
    await page.goto("/orders");
    await page.getByLabel("Search orders", { exact: true }).fill("Ada");
    await page.getByLabel("Status", { exact: true }).selectOption("paid");
    await page.getByRole("button", { name: "Apply filters" }).click();
    await expect(page).toHaveURL(/q=Ada/);
    await expect(page).toHaveURL(/f\.status=paid/);
    const rows = page.getByRole("table", { name: "Orders", exact: true }).locator("tbody tr");
    expect(await rows.count()).toBeGreaterThan(0);
    for (const row of await rows.all()) { await expect(row).toContainText("Ada"); await expect(row).toContainText("paid"); }
  } finally { await context.close(); }
});
test("cursor, sorting and numbered navigation preserve state", async ({ page }) => {
  await page.goto("/orders?per=10&sort=customer&dir=desc");
  const rows = page.locator("tbody tr");
  const first = await rows.allTextContents();
  await page.getByRole("link", { name: "Next", exact: true }).click();
  await expect(page).toHaveURL(/after=/);
  await expect(page).toHaveURL(/per=10/);
  await page.getByRole("link", { name: "Previous", exact: true }).click();
  expect(await rows.allTextContents()).toEqual(first);
  await page.getByRole("link", { name: "Customer", exact: true }).click();
  await expect(page).toHaveURL(/dir=asc/);
  expect(new URL(page.url()).searchParams.has("before")).toBe(false);
  await page.goto("/orders?mode=pages&per=10");
  await page.getByRole("link", { name: "Page 2", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Showing 11–20 of 67");
  await page.goto("/orders?q=DoesNotExist&sort=unknown&after=garbage");
  await expect(page.getByRole("heading", { name: "No results" })).toBeVisible();
});
for (const scheme of ["light", "dark"] as const) {
  test(`orders is accessible in ${scheme} at narrow width`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize({ width: 320, height: 800 });
    await page.goto("/orders");
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    const skip = page.getByRole("link", { name: "Skip to main content" });
    // Some WebKit builds skip links according to the platform preference.
    // Test activation there after focusing the link, and tab order elsewhere.
    if (test.info().project.name === "webkit") await skip.focus();
    else await page.keyboard.press("Tab");
    await expect(skip).toBeFocused();
    await page.keyboard.press("Enter");
    expect(new URL(page.url()).hash).toBe("#main");
  });
}
test("server collections add no behavior module", async ({ page }) => {
  await page.goto("/orders");
  expect(await page.locator('script[type="module"]').count()).toBe(0);
});
