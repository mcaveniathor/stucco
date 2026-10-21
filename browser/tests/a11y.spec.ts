import { test, expect } from "@playwright/test";
import { audit } from "../audit";

const paths = [
  "/index.html",
  "/showcase.html",
  "/palette.html",
  "/themes.html",
  "/layout.html",
  "/typography.html",
  "/actions.html",
  "/forms.html",
  "/overlays.html",
  "/display.html",
  "/tabs.html",
  "/tabs-items.html",
  "/collections.html",
  "/app.html",
  "/fixtures/enhanced.html",
];

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`${colorScheme} scheme`, () => {
    // Set the scheme before creating any page, including axe's analysis page.
    test.use({ colorScheme });

    for (const path of paths) {
      test(`${path} has no axe violations (${colorScheme})`, async ({ page }) => {
        await page.goto(path);
        await expect.poll(() => page.evaluate(() =>
          matchMedia("(prefers-color-scheme: dark)").matches,
        )).toBe(colorScheme === "dark");
        const results = await audit(page);
        expect(results.violations).toEqual([]);
      });
    }
  });
}

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("the palette page renders every preset", async ({ page }) => {
    await page.goto("/palette.html");
    await expect(page.locator("section.g-preset")).toHaveCount(14);
  });
});
