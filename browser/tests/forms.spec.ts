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

test("keyboard order: skip link, back link, then the first field", async ({ page }) => {
  // Whether Tab stops on links depends on the platform (WebKit skips them on
  // some builds), so allow either; whatever is focused before the first field
  // must be the skip link and then the back link, in that order.
  const expected = ["Skip to main content", "← Gallery"];
  const seen: string[] = [];
  for (let i = 0; i < 3; i++) {
    await page.keyboard.press("Tab");
    const focused = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      return { tag: el?.tagName ?? "", name: el?.getAttribute("name") ?? "", text: el?.textContent?.trim() ?? "" };
    });
    if (focused.tag === "INPUT" && focused.name === "name") break;
    seen.push(focused.text);
  }
  await expect(page.getByLabel("Name")).toBeFocused();
  expect(expected.filter((label) => seen.includes(label))).toEqual(seen);
});
