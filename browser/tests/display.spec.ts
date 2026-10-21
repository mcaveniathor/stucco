import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/display.html");
});

test("display page fits 320px", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  const unbroken = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz01234567";
  expect(unbroken).toHaveLength(60);
  for (const selector of [".st-badge", ".st-breadcrumbs", ".st-steps"]) {
    await expect(page.locator(selector).getByText(unbroken, { exact: true })).toBeVisible();
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});

test("disclosure toggles with Enter and Space", async ({ page }) => {
  const details = page.locator("details", { hasText: "Shipping details" }).first();
  const summary = details.locator("summary");
  await summary.focus();
  await expect(details).not.toHaveAttribute("open");
  await page.keyboard.press("Enter");
  await expect(details).toHaveAttribute("open", "");
  await page.keyboard.press("Space");
  await expect(details).not.toHaveAttribute("open");
});

test("exclusive accordion closes siblings", async ({ page }) => {
  const accordion = page.locator(".st-accordion");
  const first = accordion.locator("details").nth(0);
  const second = accordion.locator("details").nth(1);
  await expect(first).toHaveAttribute("open", "");
  await second.locator("summary").click();
  await expect(second).toHaveAttribute("open", "");
  await expect(first).not.toHaveAttribute("open");
});

test("progress and meter have accessible names", async ({ page }) => {
  await expect(page.getByRole("progressbar", { name: "Import" })).toBeVisible();
  await expect(page.getByRole("progressbar", { name: "Uploading" })).toBeVisible();
  await expect(page.getByRole("meter", { name: "Storage" })).toBeVisible();
});

test("breadcrumb separators are not announced", async ({ page }) => {
  const trail = page.getByRole("navigation", { name: "Breadcrumb" });
  // Drop the snapshot's own "/url:" lines; what's left is what is announced.
  const snapshot = (await trail.ariaSnapshot())
    .split("\n")
    .filter((line) => !line.trim().startsWith("- /url:"))
    .join("\n");
  expect(snapshot).not.toContain("/");
  expect(snapshot).toContain("Edit");
});

test("steps announce their state", async ({ page }) => {
  const steps = page.getByRole("navigation", { name: "Checkout" });
  await expect(steps.getByRole("link", { name: "Cart (completed)" })).toBeVisible();
  await expect(steps.getByText("Account (completed)")).toBeAttached();
  await expect(steps.locator('[aria-current="step"]')).toHaveText("Shipping");
});
