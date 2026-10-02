import { test, expect } from "@playwright/test";

test("a forced scheme on <html> reaches named-theme sections", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto("/palette.html");
  await page.evaluate(() => {
    document.documentElement.dataset.theme = "dark";
  });
  const scheme = await page
    .locator('section[data-st-theme="iris"]')
    .evaluate((el) => getComputedStyle(el).colorScheme);
  expect(scheme).toBe("dark");
});
