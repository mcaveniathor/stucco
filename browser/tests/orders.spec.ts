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
  // Parallel CRUD tests may add orders, so only the range is fixed.
  await expect(page.getByRole("status")).toHaveText(/^Showing 11–20 of \d+$/);
  await page.goto("/orders?q=DoesNotExist&sort=unknown&after=garbage");
  await expect(page.getByRole("heading", { name: "No results" })).toBeVisible();
});
for (const scheme of ["light", "dark"] as const) {
  test.describe(`orders in ${scheme}`, () => {
    test.use({ colorScheme: scheme, viewport: { width: 320, height: 800 } });

    test("is accessible at narrow width", async ({ page }) => {
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
  });
}
test("collections add no behavior module of their own", async ({ page }) => {
  // The row menus and delete dialogs load the overlay script; the table,
  // filters and pagination still need none.
  await page.goto("/orders");
  const modules = await page.locator('script[type="module"]').evaluateAll((scripts) =>
    scripts.map((s) => s.getAttribute("src") ?? ""),
  );
  expect(modules.map((src) => src.replace(/^\/_stucco\/|\.[0-9a-f]+\.js$/g, "")).sort()).toEqual(["overlay", "stucco-runtime"]);
});

for (const js of [true, false]) {
  test.describe(`orders CRUD with JavaScript ${js ? "on" : "off"}`, () => {
    test.use({ javaScriptEnabled: js });

    test("create, edit and delete an order", async ({ page }) => {
      // Unique per run, so a reused server's leftover orders can't match.
      const name = `Browser ${test.info().project.name} ${js ? "js" : "nojs"} ${Date.now().toString(36)}`;
      await page.goto("/orders/new");
      await page.getByLabel("Customer").fill(name);
      await page.getByLabel("Status").selectOption("paid");
      await page.getByLabel("Total").fill("12.50");
      await page.getByLabel("Created").fill("2026-10-01");
      await page.getByRole("button", { name: "Create order" }).click();
      await expect(page).toHaveURL(/\/orders$/);
      await expect(page.getByRole("status").filter({ hasText: "created" })).toBeVisible();

      await page.goto(`/orders?q=${encodeURIComponent(name)}`);
      const row = page.locator("tbody tr").filter({ hasText: name });
      await row.getByRole("button", { name: /^Actions/ }).click();
      await row.getByRole("link", { name: "Edit" }).click();
      await page.getByLabel("Customer").fill(`${name} edited`);
      await page.getByRole("button", { name: "Save order" }).click();
      await expect(page.getByRole("status").filter({ hasText: "saved" })).toBeVisible();

      await page.goto(`/orders?q=${encodeURIComponent(`${name} edited`)}`);
      const edited = page.locator("tbody tr").filter({ hasText: `${name} edited` });
      await edited.getByRole("button", { name: /^Actions/ }).click();
      await edited.getByRole("link", { name: "Delete…" }).click();
      // With script the dialog opens in place; without it, the link goes to
      // the confirmation page. Either way one "Delete order" button shows.
      if (js) await expect(page.getByRole("dialog")).toBeVisible();
      else await expect(page).toHaveURL(/\/orders\/\d+\/delete$/);
      await page.getByRole("button", { name: "Delete order" }).click();
      await expect(page.getByRole("status").filter({ hasText: "deleted" })).toBeVisible();
      await page.goto(`/orders?q=${encodeURIComponent(`${name} edited`)}`);
      await expect(page.getByRole("heading", { name: "No results" })).toBeVisible();
    });

    test("a failed submit focuses the error summary", async ({ page }) => {
      await page.goto("/orders/new");
      await page.getByLabel("Customer").fill("Ada");
      await page.getByLabel("Status").selectOption("paid");
      // Passes the browser's own checks, fails the server's.
      await page.getByLabel("Total").fill("abc");
      await page.getByLabel("Created").fill("2026-10-01");
      await page.getByRole("button", { name: "Create order" }).click();
      const summary = page.getByRole("group", { name: "There is a problem" });
      await expect(summary).toBeFocused();
      await expect(summary.getByRole("link", { name: "Enter an amount like 12.50" })).toHaveAttribute("href", "#order-total");
    });
  });
}

test("order pages are accessible", async ({ page }) => {
  for (const path of ["/orders/new", "/orders/1", "/orders/1/edit", "/orders/1/delete"]) {
    await page.goto(path);
    expect((await new AxeBuilder({ page }).analyze()).violations, path).toEqual([]);
  }
});

test("a stale edit shows an accessible conflict and keeps the entries", async ({ browser }) => {
  const context = await browser.newContext({ baseURL: "http://localhost:4181" });
  try {
    const name = `Conflict ${test.info().project.name} ${Date.now().toString(36)}`;
    const mine = await context.newPage();
    await mine.goto("/orders/new");
    await mine.getByLabel("Customer").fill(name);
    await mine.getByLabel("Status").selectOption("pending");
    await mine.getByLabel("Total").fill("5");
    await mine.getByLabel("Created").fill("2026-10-01");
    await mine.getByRole("button", { name: "Create order" }).click();
    const flash = await mine.getByRole("status").filter({ hasText: "created" }).textContent();
    const id = flash!.match(/Order (\d+)/)![1];

    await mine.goto(`/orders/${id}/edit`);
    const theirs = await context.newPage();
    await theirs.goto(`/orders/${id}/edit`);
    await theirs.getByLabel("Customer").fill(`${name} theirs`);
    await theirs.getByRole("button", { name: "Save order" }).click();
    await expect(theirs.getByRole("status").filter({ hasText: "saved" })).toBeVisible();

    await mine.getByLabel("Customer").fill(`${name} mine`);
    await mine.getByRole("button", { name: "Save order" }).click();
    await expect(mine.getByRole("heading", { name: "This record has changed" })).toBeVisible();
    await expect(mine.getByRole("group", { name: "There is a problem" })).toBeFocused();
    await expect(mine.getByLabel("Customer")).toHaveValue(`${name} mine`);
    expect((await new AxeBuilder({ page: mine }).analyze()).violations).toEqual([]);
  } finally {
    await context.close();
  }
});

test("not-found pages are accessible HTML", async ({ page }) => {
  for (const path of ["/orders/999999", "/orders/abc/edit", "/nowhere"]) {
    const response = await page.goto(path);
    expect(response!.status(), path).toBe(404);
    await expect(page.getByRole("link", { name: "Back to orders" })).toBeVisible();
    expect((await new AxeBuilder({ page }).analyze()).violations, path).toEqual([]);
  }
});

for (const js of [true, false]) {
  test(`compact filters retain range and sorting state with JavaScript ${js}`, async ({ browser }) => {
    const context = await browser.newContext({ javaScriptEnabled: js, baseURL: "http://localhost:4181" });
    try {
      const page = await context.newPage();
      await page.goto("/orders?mode=pages");
      const disclosure = page.locator(".st-filter-secondary");
      await expect(disclosure).not.toHaveAttribute("open", "");
      await page.getByText("More filters & sorting", { exact: true }).click();
      await page.locator('[name="f.total.min"]').fill("20");
      await page.getByLabel("Sort by", { exact: true }).selectOption("customer");
      await page.getByRole("button", { name: "Apply filters" }).click();
      await expect(page).toHaveURL(/mode=pages/);
      await expect(page).toHaveURL(/f.total.min=20/);
      await expect(disclosure).toHaveAttribute("open", "");
      await expect(page.locator('[name="f.total.min"]')).toHaveValue("20");
      await expect(page.getByLabel("Sort by", { exact: true })).toHaveValue("customer");
      for (const cell of await page.locator('tbody td[data-kind="number"]:nth-child(4)').all()) {
        expect(Number((await cell.innerText()).replace("$", ""))).toBeGreaterThanOrEqual(20);
      }
      await page.getByRole("link", { name: "Clear", exact: true }).click();
      // Clear resets filters but deliberately retains the existing sort.
      await expect(disclosure).toHaveAttribute("open", "");
      await expect(page.locator('[name="f.total.min"]')).toHaveValue("");
      await expect(page.getByLabel("Sort by", { exact: true })).toHaveValue("customer");
      await expect(page).toHaveURL(/mode=pages/);
    } finally { await context.close(); }
  });
}

test("long names and maximum amounts stay readable in cards, with exact page totals", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 320, height: 900 }, baseURL: "http://localhost:4181" });
  const page = await context.newPage();
  const prefix = `Visual ${test.info().project.name} ${Date.now()}`;
  const customer = `${prefix} Alexandra Montgomery-Sutherland & Partners`.slice(0, 80);
  const query = `/orders?q=${encodeURIComponent(prefix)}`;
  let created = 0;
  try {
    for (let i = 0; i < 2; i++) {
      await page.goto("/orders/new");
      await page.getByLabel("Customer").fill(customer);
      await page.getByLabel("Status").selectOption("pending");
      await page.getByLabel("Total").fill("184467440737095516.15");
      await page.getByLabel("Created").fill("2026-10-01");
      await page.getByRole("button", { name: "Create order" }).click();
      await expect(page.getByRole("status").filter({ hasText: "created" })).toBeVisible();
      created++;
    }
    await page.goto(query);
    await expect(page.locator(".st-stat").filter({ hasText: "Value on this page" }).locator(".st-stat-value")).toHaveText("$368934881474191032.30");
    await expect(page.locator("tbody tr")).toHaveCount(2);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    for (const row of await page.locator("tbody tr").all()) {
      await expect(row).toContainText(customer);
      await expect(row).toContainText("$184467440737095516.15");
    }
  } finally {
    for (let i = 0; i < created; i++) {
      await page.goto(query);
      const row = page.locator("tbody tr").first();
      await row.getByRole("button", { name: /^Actions/ }).click();
      await row.getByRole("link", { name: "Delete…" }).click();
      await page.getByRole("button", { name: "Delete order", exact: true }).click();
    }
    await context.close();
  }
});
