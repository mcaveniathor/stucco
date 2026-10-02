import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

for (const path of [
  "/index.html",
  "/palette.html",
  "/layout.html",
  "/typography.html",
  "/actions.html",
  "/forms.html",
  "/collections.html",
  "/app.html",
  "/fixtures/enhanced.html",
]) {
  for (const colorScheme of ["light", "dark"] as const) {
    test(`${path} has no axe violations (${colorScheme})`, async ({ page }) => {
      await page.emulateMedia({ colorScheme });
      await page.goto(path);
      const results = await new AxeBuilder({ page }).analyze();
      expect(results.violations).toEqual([]);
    });
  }
}

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("the palette page renders every preset", async ({ page }) => {
    await page.goto("/palette.html");
    await expect(page.locator("section.g-preset")).toHaveCount(14);
  });
});
