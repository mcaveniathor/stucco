import { readFile } from "node:fs/promises";
import { type APIRequestContext, type Page } from "@playwright/test";
import { audit } from "../audit";
import { test, expect, expireSession, loginHttp } from "./fixtures";

// Keep each engine below the production limit of eight retained jobs per account.
// Cancellation revokes downloads but retains request identities for safe replay.
test.use({ account: async ({ browserName }, use) => {
  await use(({ chromium: "export_chromium", firefox: "export_firefox", webkit: "export_webkit" } as const)[browserName]);
} });

const csvHeader = "order_id,revision,customer,quantity_one,quantity_two,total_cents\r\n";
const startForm = (page: Page) => page.locator('form[action="/exports"]');
const downloadLink = (page: Page) => page.getByRole("link", { name: "Download CSV", exact: true });

function hidden(html: string, name: string): string {
  const tag = html.match(/<input\b[^>]*>/g)?.find((tag) => tag.includes(`name="${name}"`));
  const value = tag?.match(/\bvalue="([^"]*)"/)?.[1];
  if (value === undefined) throw new Error(`Missing export form input ${name}`);
  return value;
}

async function startFields(request: APIRequestContext) {
  const response = await request.get("/exports");
  expect(response.status()).toBe(200);
  const html = await response.text();
  return Object.fromEntries(["_csrf", "expected_user_id", "expected_tenant_id", "expected_generation", "token"]
    .map((name) => [name, hidden(html, name)]));
}

async function startNative(request: APIRequestContext) {
  const form = await startFields(request);
  const response = await request.post("/exports", { form, maxRedirects: 0 });
  expect(response.status()).toBe(303);
  expect(response.headers().location).toBe(`/exports/${form.token}`);
  return form;
}

async function cancel(request: APIRequestContext, token: string | undefined) {
  if (!token) return;
  const form = await startFields(request);
  const response = await request.post(`/exports/${token}/cancel`, { form, maxRedirects: 0 });
  expect(response.status()).toBe(303);
}

async function ready(page: Page, hydrated = true) {
  await expect(page.locator(`.export-progress[data-hydrated="${hydrated}"]`)).toBeVisible();
}

async function nativeReady(page: Page) {
  await expect.poll(async () => {
    await page.getByRole("link", { name: "Refresh status", exact: true }).click();
    return downloadLink(page).count();
  }, { timeout: 20_000, intervals: [200, 500, 1000] }).toBe(1);
}

async function downloadedCsv(page: Page) {
  const [download] = await Promise.all([page.waitForEvent("download"), downloadLink(page).click()]);
  expect(download.suggestedFilename()).toMatch(/^orders-[a-f0-9]{32}\.csv$/);
  const path = await download.path();
  expect(path).not.toBeNull();
  const csv = await readFile(path!, "utf8");
  expect(csv.startsWith(csvHeader)).toBe(true);
  expect(csv.split("\r\n").filter(Boolean).length).toBeGreaterThan(1);
  return csv;
}

test.describe("exports without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("native export creation, explicit status refresh and CSV download work", async ({ page, context }) => {
    let token: string | undefined;
    try {
      await page.goto("/exports");
      await expect(startForm(page)).toBeVisible();
      token = await startForm(page).locator('input[name="token"]').inputValue();
      const posted = page.waitForResponse((response) => response.request().method() === "POST"
        && new URL(response.url()).pathname === "/exports");
      await page.getByRole("button", { name: "Export orders", exact: true }).click();
      expect((await posted).status()).toBe(303);
      await expect(page).toHaveURL(`/exports/${token}`);
      await ready(page, false);
      await nativeReady(page);
      await downloadedCsv(page);
    } finally {
      await cancel(context.request, token);
    }
  });
});

test("hydrated export status survives refresh and downloads a private accessible result", async ({ page, context }) => {
  let token: string | undefined;
  try {
    await page.goto("/exports");
    await expect(page.locator('section[data-hydrated="true"]')).toBeVisible();
    token = await startForm(page).locator('input[name="token"]').inputValue();
    await page.getByRole("button", { name: "Export orders", exact: true }).click();
    await expect(page).toHaveURL(`/exports/${token}`);
    await ready(page);
    await page.reload();
    await ready(page);
    await expect(downloadLink(page)).toBeVisible({ timeout: 20_000 });
    await downloadedCsv(page);
    const response = await context.request.get(`/exports/${token}/download`);
    expect(response.status()).toBe(200);
    expect(response.headers()["cache-control"]).toContain("private, no-store");
    expect(response.headers()["x-content-type-options"]).toBe("nosniff");
    for (const [layout, width, colorScheme] of [["wide-light", 1280, "light"], ["narrow-dark", 360, "dark"]] as const) {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
      expect((await audit(page)).violations).toEqual([]);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const path = test.info().outputPath(`export-complete-${layout}.png`);
      await page.screenshot({ path, fullPage: true });
      await test.info().attach(`export-complete-${layout}`, { path, contentType: "image/png" });
    }
  } finally {
    await cancel(context.request, token);
  }
});

test("replaying a native export request returns the same job without another list entry", async ({ page, context }) => {
  let token: string | undefined;
  try {
    const form = await startNative(context.request);
    token = form.token;
    const replay = await context.request.post("/exports", { form, maxRedirects: 0 });
    expect(replay.status()).toBe(303);
    expect(replay.headers().location).toBe(`/exports/${token}`);
    await page.goto("/exports");
    await expect(page.locator(`li a[href="/exports/${token}"]`)).toHaveCount(1);
    await page.locator(`li a[href="/exports/${token}"]`).click();
    await ready(page);
    await expect(downloadLink(page)).toBeVisible({ timeout: 20_000 });
    await expect(page.getByText("Attempt 1 of 3", { exact: true })).toBeVisible();
  } finally {
    await cancel(context.request, token);
  }
});

test("cancelling a completed export revokes its download and cannot be retried", async ({ page, context }) => {
  let token: string | undefined;
  try {
    token = (await startNative(context.request)).token;
    await page.goto(`/exports/${token}`);
    await ready(page);
    await expect(downloadLink(page)).toBeVisible({ timeout: 20_000 });
    await page.getByRole("button", { name: "Cancel and remove export", exact: true }).click();
    await expect(page.locator('[role="status"]').filter({ hasText: /^Cancelled$/ })).toBeVisible();
    await expect(downloadLink(page)).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Retry export", exact: true })).toHaveCount(0);
    expect((await context.request.get(`/exports/${token}/download`)).status()).toBe(404);
    await page.reload();
    await expect(page.locator('[role="status"]').filter({ hasText: /^Cancelled$/ })).toBeVisible();
  } finally {
    await cancel(context.request, token);
  }
});

test("downloads are private to the requesting account and require a current session", async ({ page, context, browser, baseURL, account }) => {
  let token: string | undefined;
  try {
    token = (await startNative(context.request)).token;
    await page.goto(`/exports/${token}`);
    await ready(page);
    await expect(downloadLink(page)).toBeVisible({ timeout: 20_000 });
    for (const account of ["carol", "bob"] as const) {
      const other = await browser.newContext({ baseURL });
      try {
        await loginHttp(other.request, account, baseURL);
        expect((await other.request.get(`/exports/${token}/download`)).status()).toBe(404);
        const detail = await other.request.get(`/exports/${token}`);
        expect(detail.status()).toBe(404);
        expect(await detail.text()).not.toContain("Download CSV");
      } finally {
        await other.close();
      }
    }
    await expireSession(context);
    const expired = await context.request.get(`/exports/${token}/download`);
    expect(expired.status()).toBe(401);
    expect(await expired.text()).toContain("Sign in to continue");
  } finally {
    await loginHttp(context.request, account);
    await cancel(context.request, token);
  }
});

test("a lost start response keeps the same request token for reconciliation and replay", async ({ page, context }) => {
  let token: string | undefined;
  let intercepted = false;
  try {
    await page.goto("/exports");
    await expect(page.locator('section[data-hydrated="true"]')).toBeVisible();
    token = await startForm(page).locator('input[name="token"]').inputValue();
    await page.route("**/api/**", async (route) => {
      const request = route.request();
      const body = new URLSearchParams(request.postData() ?? "");
      if (!intercepted && request.method() === "POST" && body.get("token") === token) {
        intercepted = true;
        const committed = await route.fetch();
        expect(committed.status()).toBe(200);
        await route.abort("failed");
      } else {
        await route.continue();
      }
    });
    await page.getByRole("button", { name: "Export orders", exact: true }).click();
    await expect(page.getByRole("status").filter({ hasText: "The request could not be confirmed." })).toBeVisible();
    expect(intercepted).toBe(true);
    await expect(page.getByRole("link", { name: "Check request status", exact: true })).toHaveAttribute("href", `/exports/${token}`);
    await expect(startForm(page).locator('input[name="token"]')).toHaveValue(token!);
    await page.getByRole("button", { name: "Export orders", exact: true }).click();
    await expect(page).toHaveURL(`/exports/${token}`);
    await expect(downloadLink(page)).toBeVisible({ timeout: 20_000 });
    await page.getByRole("link", { name: "Back to exports", exact: true }).click();
    await expect(page.locator(`li a[href="/exports/${token}"]`)).toHaveCount(1);
  } finally {
    await page.unroute("**/api/**");
    await cancel(context.request, token);
  }
});

test("a delayed history jump between export tokens hides the previous export's controls", async ({ page, context }) => {
  let tokenA: string | undefined;
  let tokenB: string | undefined;
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  let intercepted = false;
  try {
    tokenA = (await startNative(context.request)).token;
    tokenB = (await startNative(context.request)).token;
    expect(tokenB).not.toBe(tokenA);

    // Establish A → list → B in the same hydrated router. Skipping the list
    // during Back changes only /exports/:token, keeping its route component.
    await page.goto(`/exports/${tokenA}`);
    await ready(page);
    await expect(downloadLink(page)).toBeVisible({ timeout: 20_000 });
    await page.getByRole("link", { name: "Back to exports", exact: true }).click();
    await expect(page).toHaveURL("/exports");
    await page.locator(`li a[href="/exports/${tokenB}"]`).click();
    await expect(page).toHaveURL(`/exports/${tokenB}`);
    await ready(page);
    await expect(downloadLink(page)).toHaveAttribute("href", `/exports/${tokenB}/download`);

    await page.route("**/api/**", async (route) => {
      const request = route.request();
      if (request.method() === "GET" && new URL(request.url()).searchParams.get("token") === tokenA) {
        intercepted = true;
        await gate;
        await route.continue().catch(() => {});
      } else {
        await route.continue();
      }
    });
    await page.evaluate(() => history.go(-2));
    await expect(page).toHaveURL(`/exports/${tokenA}`);
    await expect.poll(() => intercepted).toBe(true);
    await expect(downloadLink(page)).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Cancel and remove export", exact: true })).toHaveCount(0);
    await expect(page.locator(`form[action="/exports/${tokenB}/cancel"]`)).toHaveCount(0);
    await expect(page.getByRole("link", { name: "Refresh status", exact: true })).toHaveAttribute("href", `/exports/${tokenA}`);

    release();
    await ready(page);
    await expect(downloadLink(page)).toHaveAttribute("href", `/exports/${tokenA}/download`);
    await expect(page.locator(`form[action="/exports/${tokenA}/cancel"]`)).toBeVisible();
    await expect(page.locator(`a[href="/exports/${tokenB}/download"]`)).toHaveCount(0);
  } finally {
    release();
    await page.unroute("**/api/**");
    await cancel(context.request, tokenA);
    await cancel(context.request, tokenB);
  }
});
