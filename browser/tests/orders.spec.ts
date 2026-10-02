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
  // Up to the end of 2026: the seeded orders, not the ones other tests create.
  await page.goto("/orders?mode=pages&per=10&f.created.max=2026-12-31");
  await page.getByRole("link", { name: "Page 2", exact: true }).click();
  await expect(page.getByRole("status")).toHaveText("Showing 11–20 of 67");
  await page.goto("/orders?q=DoesNotExist&sort=unknown&after=garbage");
  await expect(page.getByRole("heading", { name: "No matching orders" })).toBeVisible();
  await page.getByRole("link", { name: "Remove filter: Search “DoesNotExist”" }).click();
  await expect(page).not.toHaveURL(/q=/);
});
test("an order goes from creation to deletion without JavaScript", async ({ browser }, info) => {
  const context = await browser.newContext({ javaScriptEnabled: false, baseURL: "http://localhost:4181" });
  try {
    const page = await context.newPage();
    // The announced notice: the outcome, not standing notes such as "archived".
    const notice = page.locator(".st-notice[role]");
    const name = `Browser test ${info.project.name} ${Date.now()}`;
    await page.goto("/orders");
    await page.getByRole("link", { name: "New order" }).click();
    // Accessible names, which leave out the required marker.
    const customer = page.getByRole("textbox", { name: "Customer", exact: true });
    const total = page.getByRole("textbox", { name: "Total", exact: true });
    await customer.fill(name);
    await total.fill("12.x");
    await page.locator("#order-created").fill("2030-01-01");
    await page.getByRole("button", { name: "Create order" }).click();
    // The server refuses it, keeps what was typed and links the problem.
    const summary = page.getByRole("group", { name: "There is a problem" });
    await expect(summary).toBeFocused();
    await expect(customer).toHaveValue(name);
    await expect(total).toHaveValue("12.x");
    await summary.getByRole("link", { name: "Enter the total as an amount, like 12.50" }).click();
    await expect(page).toHaveURL(/#order-total$/);
    await total.fill("12.50");
    await page.getByRole("button", { name: "Create order" }).click();
    await expect(notice).toContainText("was created");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(/^Order \d+$/);
    await page.getByRole("link", { name: "Edit", exact: true }).click();
    await page.getByRole("textbox", { name: "Note (optional)" }).fill("Fragile");
    await page.getByRole("button", { name: "Save changes" }).click();
    await expect(notice).toContainText("was saved");
    await expect(page.getByText("Fragile")).toBeVisible();
    await page.getByRole("link", { name: "Archive", exact: true }).click();
    await page.getByRole("button", { name: /^Archive order \d+$/ }).click();
    await expect(notice).toContainText("was archived");
    await expect(page.getByRole("link", { name: "Edit", exact: true })).toHaveCount(0);
    await page.getByRole("link", { name: "Delete", exact: true }).click();
    await page.getByRole("button", { name: /^Delete order \d+$/ }).click();
    await expect(page).toHaveURL(/\/orders(\?per=25)?$/);
    await expect(notice).toContainText("was deleted");
  } finally { await context.close(); }
});
for (const path of ["/orders/1", "/orders/new", "/orders/1/delete", "/orders/bulk?action=archive&id=1&id=2", "/orders/999999"]) {
  for (const scheme of ["light", "dark"] as const) {
    test(`${path} is accessible in ${scheme} at narrow width`, async ({ page }) => {
      await page.emulateMedia({ colorScheme: scheme });
      await page.setViewportSize({ width: 320, height: 800 });
      await page.goto(path);
      expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    });
  }
}
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
    await expect(page).toHaveURL(/#main$/);
  });
}
test("server collections add no behavior module", async ({ page }) => {
  await page.goto("/orders");
  expect(await page.locator('script[type="module"]').count()).toBe(0);
});
