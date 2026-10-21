import { test, expect } from "@playwright/test";

test("named scopes apply body typography without changing the surrounding page", async ({ page }) => {
  await page.goto("/showcase-relay-project.html");
  for (const scheme of ["light", "dark"]) {
    const scope = page.locator(`.st-theme-scope[data-theme="${scheme}"]`);
    await expect(scope.locator(".showcase-project-focus p").first()).toHaveCSS("font-family", /monospace/);
  }
  await expect(page.locator("body")).not.toHaveCSS("font-family", /monospace/);
  await page.goto("/showcase-grove-project.html");
  for (const scheme of ["light", "dark"]) {
    const scope = page.locator(`.st-theme-scope[data-theme="${scheme}"]`);
    expect(await scope.evaluate((el) => {
      const style = getComputedStyle(el);
      return parseFloat(style.lineHeight) / parseFloat(style.fontSize);
    })).toBeCloseTo(1.65);
  }
});

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
