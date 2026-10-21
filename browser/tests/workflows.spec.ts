import { test, expect } from "@playwright/test";
import { audit } from "../audit";

for (const width of [320, 390, 1280]) {
  test.describe(`workflows at ${width}px`, () => {
    test.use({ viewport: { width, height: 900 }, javaScriptEnabled: false });

    test("one table and one copy of each action, with usable sorting and no overflow", async ({ page }) => {
      await page.goto("/workflows.html");
      const table = page.getByRole("table", { name: "Review queue" });
      await expect(table).toHaveCount(1);
      await expect(table.getByRole("columnheader")).toHaveCount(3);
      await expect(table.getByRole("row")).toHaveCount(3);
      for (const id of [42, 43]) {
        await expect(table.getByRole("link", { name: `Open ${id}` })).toHaveCount(1);
        await expect(table.getByRole("link", { name: `Open ${id}` })).toBeVisible();
      }
      await expect(table.locator("tbody > tr").first()).toHaveCSS("display", width < 768 ? "block" : "table-row");
      const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
      expect(overflow).toBe(false);
      const sort = table.getByRole("link", { name: "Customer", exact: true });
      await sort.focus();
      await expect(sort).toBeFocused();
      await page.keyboard.press("Enter");
      await expect(page).toHaveURL(/sort=name/);
    });

    test("context survives detail links and form submissions without replacing the stale revision", async ({ page }) => {
      await page.goto("/workflows.html");
      await page.getByRole("link", { name: "Open 42", exact: true }).click();
      await expect(page).toHaveURL(url => url.searchParams.get("record") === "42");
      const detail = new URL(page.url());
      expect(detail.searchParams.get("return_to")).toBe("/collections.html?dir=desc&page=2&q=Ada&sort=name");
      await expect(page.getByLabel("Customer", { exact: true })).toHaveValue("Ada (my unsaved edit)");
      await expect(page.locator('input[name="revision"]')).toHaveValue("v1");
      await page.getByLabel("Customer", { exact: true }).fill("My retained edit");
      await page.getByRole("button", { name: "Preview submission" }).click();
      await expect(page).toHaveURL(url => url.searchParams.get("customer") === "My retained edit");
      const submitted = new URL(page.url());
      expect(submitted.searchParams.get("revision")).toBe("v1");
      expect(submitted.searchParams.get("return_to")).toBe(detail.searchParams.get("return_to"));
      await page.getByRole("link", { name: "Discard entries and reload latest version" }).click();
      await expect(page).toHaveURL(/\/workflows-latest\.html/);
      await expect(page.getByRole("group", { name: "This record has changed" })).toHaveCount(0);
      await expect(page.getByLabel("Customer", { exact: true })).toHaveValue("Ada (latest saved)");
      await expect(page.locator('input[name="revision"]')).toHaveValue("v2");
      await page.getByRole("link", { name: "Back to queue" }).click();
      await expect(page).toHaveURL(url => url.pathname === "/collections.html" && url.searchParams.get("page") === "2");
    });

    test("blockers include a reason, role guidance and a working remedy", async ({ page }) => {
      await page.goto("/workflows.html");
      const panel = page.getByRole("region", { name: "Ready to publish" });
      await expect(panel).toContainText("1 of 2 checks complete.");
      await expect(panel).toContainText("A reviewer has not approved this record.");
      await expect(panel).toContainText("A reviewer can complete this step.");
      await panel.getByRole("link", { name: "Review latest record" }).click();
      await expect(page).toHaveURL(/workflows-latest.html/);
    });
  });
  for (const colorScheme of ["light", "dark"] as const) {
    test.describe(`workflow accessibility at ${width}px in ${colorScheme}`, () => {
      test.use({ colorScheme, viewport: { width, height: 900 } });

      test("has no axe violations", async ({ page }, testInfo) => {
        await page.goto("/workflows.html");
        expect((await audit(page)).violations).toEqual([]);
        await page.screenshot({ path: testInfo.outputPath("workflow.png"), fullPage: true });
      });
    });
  }
}

test("cards respond to container width, not just the viewport", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto("/workflows.html");
  await page.locator(".st-data-table").evaluate(el => (el as HTMLElement).style.width = "360px");
  await expect(page.locator(".st-table tbody > tr").first()).toHaveCSS("display", "block");
});

for (const javaScriptEnabled of [false, true]) {
  test.describe(`redirect feedback with JavaScript ${javaScriptEnabled ? "enabled" : "disabled"}`, () => {
    test.use({ javaScriptEnabled });
    test("shows labelled feedback with appropriate live regions", async ({ page }) => {
      await page.goto("/workflows.html");
      const success = page.locator('.st-flash[data-level="success"]');
      const error = page.locator('.st-flash[data-level="error"]');
      await expect(success).toBeVisible();
      await expect(success).toHaveAttribute("role", "status");
      await expect(success).toHaveAttribute("aria-live", "polite");
      await expect(success).toHaveText("Success: Order saved.");
      await expect(error).toBeVisible();
      await expect(error).toHaveAttribute("role", "alert");
      await expect(error).toHaveAttribute("aria-live", "assertive");
      await expect(error).toContainText("Error: The order could not be deleted.");
    });
  });
}
