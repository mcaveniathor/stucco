import { test, expect } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/forms.html");
});

test("every control is reachable by its label", async ({ page }) => {
  for (const label of ["Name", "Email", "Password", "Website", "Quantity", "Bio", "Plan", "I agree to the terms"]) {
    await expect(page.getByLabel(label, { exact: false }).first()).toBeVisible();
  }
  await expect(page.getByRole("radio", { name: "Monthly" })).toBeVisible();
});

test("invalid fields expose their errors to assistive technology", async ({ page }) => {
  const email = page.getByLabel("Email");
  await expect(email).toHaveAttribute("aria-invalid", "true");
  await expect(email).toHaveAccessibleDescription(/Enter a valid email address/);
  await expect(page.getByRole("radiogroup", { name: "Billing" })).toHaveAccessibleDescription(/Choose a billing period/);
});

test("the submitted password is not redisplayed", async ({ page }) => {
  await expect(page.getByLabel("Password")).toHaveValue("");
});

test("keyboard order: skip link, back link, then the first field", async ({ page, browserName }) => {
  if (browserName === "webkit") {
    // WebKit skips links when tabbing by default, so the first stop is the first field.
    await page.keyboard.press("Tab");
    await expect(page.getByLabel("Name")).toBeFocused();
    return;
  }
  const tab = "Tab";
  await page.keyboard.press(tab);
  await expect(page.getByRole("link", { name: "Skip to main content" })).toBeFocused();
  await page.keyboard.press(tab);
  await expect(page.getByRole("link", { name: "← Gallery" })).toBeFocused();
  await page.keyboard.press(tab);
  await expect(page.getByLabel("Name")).toBeFocused();
});
