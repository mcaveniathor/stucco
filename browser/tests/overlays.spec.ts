import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const dialog = (page: Page) => page.locator("dialog#delete-order");

test("a dialog opens from its button, traps focus and returns it", async ({ page }) => {
  await page.goto("/overlays.html");
  const opener = page.getByRole("button", { name: "Delete order" }).first();
  await opener.click();
  await expect(dialog(page)).toBeVisible();
  await expect(page.getByRole("dialog", { name: "Delete order 1042?" })).toBeVisible();
  // Focus is inside the dialog.
  expect(await page.evaluate(() => !!document.activeElement?.closest("dialog"))).toBe(true);
  await page.keyboard.press("Escape");
  await expect(dialog(page)).toBeHidden();
  await expect(opener).toBeFocused();
});

test("Cancel, the close button and a click outside close the dialog", async ({ page }) => {
  await page.goto("/overlays.html");
  const opener = page.getByRole("button", { name: "Delete order" }).first();
  await opener.click();
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  await expect(dialog(page)).toBeHidden();
  await opener.click();
  await dialog(page).getByRole("button", { name: "Close" }).click();
  await expect(dialog(page)).toBeHidden();
  await opener.click();
  await page.mouse.click(5, 5);
  await expect(dialog(page)).toBeHidden();
});

test("a link opener opens the dialog in place", async ({ page }) => {
  await page.goto("/overlays.html");
  await page.getByRole("link", { name: "Delete (link opener)" }).click();
  await expect(dialog(page)).toBeVisible();
  expect(new URL(page.url()).hash).toBe("");
});

test("a menu opens under its button and closes on Escape", async ({ page }) => {
  await page.goto("/overlays.html");
  const button = page.getByRole("button", { name: "Actions" });
  await button.click();
  const menu = page.locator("#order-actions");
  await expect(menu).toBeVisible();
  const b = (await button.boundingBox())!;
  const m = (await menu.boundingBox())!;
  expect(m.y).toBeGreaterThanOrEqual(b.y + b.height - 1);
  expect(Math.abs(m.x - b.x)).toBeLessThan(2);
  await expect(menu.getByRole("link", { name: "Edit" })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
});

test("a tooltip shows on focus and Escape hides it", async ({ page }) => {
  await page.goto("/overlays.html");
  const tip = page.locator(".st-tooltip", { hasText: "Duplicate" });
  await page.getByRole("button", { name: "Duplicate" }).focus();
  await expect(tip).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(tip).toBeHidden();
});

test("a toast can be dismissed", async ({ page }) => {
  await page.goto("/overlays.html");
  const toast = page.getByRole("status").filter({ hasText: "Order 1042 saved" });
  await expect(toast).toBeVisible();
  await toast.getByRole("button", { name: "Dismiss" }).click();
  await expect(toast).toHaveCount(0);
});

test("the fallback opens and closes dialogs where browsers lack commands", async ({ page }) => {
  // Hide native invoker commands and closedby from the script, and count the
  // dialog methods it calls; native commands never call them.
  await page.addInitScript(() => {
    (window as any).nativeClosedBy = "closedBy" in HTMLDialogElement.prototype;
    delete (HTMLButtonElement.prototype as any).command;
    delete (HTMLDialogElement.prototype as any).closedBy;
    const calls = ((window as any).dialogCalls = { showModal: 0, close: 0 });
    for (const name of ["showModal", "close"] as const) {
      const native = HTMLDialogElement.prototype[name];
      HTMLDialogElement.prototype[name] = function (this: HTMLDialogElement) {
        calls[name]++;
        return native.call(this);
      };
    }
  });
  await page.goto("/overlays.html");
  const calls = () => page.evaluate(() => (window as any).dialogCalls);
  await page.getByRole("button", { name: "Delete order" }).first().click();
  await expect(dialog(page)).toBeVisible();
  expect((await calls()).showModal).toBe(1);
  await dialog(page).getByRole("button", { name: "Cancel" }).click();
  await expect(dialog(page)).toBeHidden();
  expect((await calls()).close).toBe(1);
  await page.getByRole("button", { name: "Delete order" }).first().click();
  await page.mouse.click(5, 5);
  await expect(dialog(page)).toBeHidden();
  // Native light dismiss closes on pointer down, before the fallback's click.
  const native = await page.evaluate(() => (window as any).nativeClosedBy);
  expect((await calls()).close).toBe(native ? 1 : 2);
});

test("open dialogs and menus have no axe violations", async ({ page }) => {
  await page.goto("/overlays.html");
  await page.getByRole("button", { name: "Delete order" }).first().click();
  await expect(dialog(page)).toBeVisible();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  await page.keyboard.press("Escape");
  await page.getByRole("button", { name: "Actions" }).click();
  await expect(page.locator("#order-actions")).toBeVisible();
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
});

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("menus still open and toasts show no dead button", async ({ page }) => {
    await page.goto("/overlays.html");
    await page.getByRole("button", { name: "Actions" }).click();
    await expect(page.locator("#order-actions")).toBeVisible();
    await page.keyboard.press("Escape");
    const toast = page.locator(".st-toast");
    await expect(toast).toBeVisible();
    await expect(toast.getByRole("button", { name: "Dismiss" })).toBeHidden();
  });
});
