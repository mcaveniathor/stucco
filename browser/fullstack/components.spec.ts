import { type Page } from "@playwright/test";
import { audit } from "../audit";
import { test, expect } from "./fixtures";

const gallery = (page: Page) => page.locator("#component-gallery");
const city = (page: Page) => page.getByLabel("City", { exact: true });
const recipient = (page: Page, position: number) => page.getByRole("textbox", { name: `Recipient ${position}`, exact: true });
const add = (page: Page) => page.getByRole("button", { name: "Add recipient", exact: true });
const remove = (page: Page, position: number) => page.getByRole("button", { name: `Remove recipient ${position}`, exact: true });

async function open(page: Page, path = "/components", hydrated = true) {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const response = await page.goto(path);
  expect(response?.status()).toBe(200);
  await expect(gallery(page)).toHaveAttribute("data-hydrated", String(hydrated));
  expect(errors).toEqual([]);
  return errors;
}

test.describe("component gallery without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("native fields submit repeated values and the dialog has a useful fallback page", async ({ page }) => {
    await open(page, "/components", false);
    await expect(add(page)).toBeDisabled();
    await expect(remove(page, 1)).toBeDisabled();
    await city(page).fill("A new city <&>");
    await recipient(page, 1).fill("First <recipient>");
    await recipient(page, 2).fill("Second & recipient");
    await page.getByRole("button", { name: "Preview submitted values", exact: true }).click();
    await expect(gallery(page)).toHaveAttribute("data-hydrated", "false");
    expect(new URL(page.url()).searchParams.getAll("recipient")).toEqual(["First <recipient>", "Second & recipient"]);
    await expect(city(page)).toHaveValue("A new city <&>");
    await expect(page.locator("#gallery-value-preview")).toContainText("First <recipient>, Second & recipient");
    await expect(page.getByText("Native form values received. No data was saved.", { exact: true })).toBeVisible();
    await page.getByRole("link", { name: "Open gallery help", exact: true }).click();
    await expect(page).toHaveURL(/details=open/);
    await expect(page.locator("#gallery-help-fallback")).toContainText("The same component works inside any Leptos page.");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await page.getByRole("link", { name: "Back to gallery", exact: true }).click();
    await expect(page.locator("#gallery-help-fallback")).toHaveCount(0);
    await expect(page.getByLabel("Example files", { exact: true })).toBeEnabled();
  });
});

test("native modal contains keyboard focus and Escape restores its opening link", async ({ page }) => {
  const errors = await open(page);
  const opener = page.getByRole("link", { name: "Open gallery help", exact: true });
  await opener.focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "About this gallery", exact: true });
  await expect(dialog).toBeVisible();
  const close = dialog.getByRole("button", { name: "Close help", exact: true });
  await expect(close).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(dialog.getByRole("link")).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(close).toBeFocused();
  // Native dialogs can include browser chrome in traversal at their boundary.
  // The invariant is that no background page control receives focus, rather
  // than a fixed number of Tabs returning to a particular modal control.
  for (let step = 0; step < 4; step++) {
    await page.keyboard.press("Tab");
    await expect.poll(() => dialog.evaluate((element) => {
      const active = document.activeElement;
      return element.contains(active) || active === document.body || active === document.documentElement;
    })).toBe(true);
  }
  await close.focus();
  await city(page).focus();
  // showModal makes the background inert even to a programmatic focus request.
  await expect(close).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible();
  await expect(opener).toBeFocused();
  await page.setViewportSize({ width: 360, height: 900 });
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await opener.click();
  await expect(dialog).toBeVisible();
  const screenshot = test.info().outputPath("components-dialog-en-narrow-dark.png");
  await page.screenshot({ path: screenshot });
  await test.info().attach("components-dialog-en-narrow-dark", { path: screenshot, contentType: "image/png" });
  await close.click();
  await expect(dialog).not.toBeVisible();
  await expect(opener).toBeFocused();
  expect(errors).toEqual([]);
});

test("repeated inputs retain stable rows and focus while enforcing configured button limits", async ({ page }) => {
  const errors = await open(page);
  await add(page).click();
  await expect(recipient(page, 3)).toBeFocused();
  await recipient(page, 3).fill("Third survives removal");
  const thirdId = await recipient(page, 3).getAttribute("id");
  await remove(page, 2).click();
  await expect(recipient(page, 2)).toHaveValue("Third survives removal");
  await expect(recipient(page, 2)).toHaveAttribute("id", thirdId!);
  await expect(recipient(page, 2)).toBeFocused();
  for (let count = 3; count <= 5; count++) {
    await add(page).click();
    await expect(recipient(page, count)).toBeFocused();
  }
  await expect(add(page)).toBeDisabled();
  await expect(page.locator('#gallery-form input[name="recipient"]')).toHaveCount(5);
  for (let count = 5; count > 1; count--) {
    await remove(page, count).click();
    await expect(recipient(page, count - 1)).toBeFocused();
  }
  await expect(remove(page, 1)).toBeDisabled();
  await expect(add(page)).toBeEnabled();
  expect(errors).toEqual([]);
});

test("datalist suggestions supplement free text and native GET submission retains all fields", async ({ page }) => {
  const errors = await open(page);
  await expect(city(page)).toHaveAttribute("list", "gallery-city-options");
  await expect(page.locator("#gallery-city-options option")).toHaveCount(3);
  await city(page).fill("Cairo");
  await expect(page.locator("#gallery-value-preview")).toContainText("City: Cairo");
  await city(page).fill("An unlisted city");
  await recipient(page, 1).fill("First edited");
  await add(page).click();
  await recipient(page, 3).fill("Third edited");
  await page.getByRole("button", { name: "Preview submitted values", exact: true }).click();
  await expect(gallery(page)).toHaveAttribute("data-hydrated", "true");
  await expect(city(page)).toHaveValue("An unlisted city");
  await expect(recipient(page, 3)).toHaveValue("Third edited");
  expect(new URL(page.url()).searchParams.getAll("recipient")).toEqual(["First edited", "Grace", "Third edited"]);
  await expect(page.locator("#gallery-value-preview")).toContainText("First edited, Grace, Third edited");
  expect(errors).toEqual([]);
});

test("hydration adopts early text and selected files without clearing the native input", async ({ page }) => {
  let release!: () => void;
  const gate = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/pkg/**", async (route) => {
    await gate;
    await route.continue().catch(() => {});
  });
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  try {
    await page.goto("/components", { waitUntil: "domcontentloaded" });
    await expect(gallery(page)).toHaveAttribute("data-hydrated", "false");
    await city(page).fill("Typed before hydration");
    await recipient(page, 2).fill("Early recipient");
    await page.getByLabel("Example files", { exact: true }).setInputFiles({
      name: "early.txt", mimeType: "text/plain", buffer: Buffer.from("early-file"),
    });
    await recipient(page, 2).focus();
    release();
    await expect(gallery(page)).toHaveAttribute("data-hydrated", "true");
    await expect(recipient(page, 2)).toBeFocused();
    await expect(page.locator("#gallery-value-preview")).toContainText("Typed before hydration");
    await expect(page.locator("#gallery-value-preview")).toContainText("Early recipient");
    await expect(page.locator("#gallery-file-preview")).toHaveText("early.txt · 10 bytes");
    expect(await page.getByLabel("Example files", { exact: true }).evaluate((input: HTMLInputElement) => input.files?.item(0)?.name)).toBe("early.txt");
    expect(errors).toEqual([]);
  } finally {
    release();
  }
});

test("native picker reports metadata for changes and clearing without upload requests", async ({ page }) => {
  const errors = await open(page);
  const requests: string[] = [];
  page.on("request", (request) => {
    if (request.method() !== "GET") requests.push(request.url());
  });
  const input = page.getByLabel("Example files", { exact: true });
  await input.setInputFiles([
    { name: "notes.txt", mimeType: "text/plain", buffer: Buffer.from("notes") },
    { name: "report.csv", mimeType: "text/csv", buffer: Buffer.from("a,b\n1,2\n") },
  ]);
  await expect(page.locator("#gallery-file-preview li")).toHaveText(["notes.txt · 5 bytes", "report.csv · 8 bytes"]);
  await input.setInputFiles([]);
  await expect(page.locator("#gallery-file-preview")).toHaveText("No files selected.");
  expect(requests).toEqual([]);
  expect(errors).toEqual([]);
});

test("gallery supports localized labels, RTL and accessible light/dark narrow layouts", async ({ page }) => {
  for (const [lang, dir] of [["en", "ltr"], ["en", "rtl"], ["ar", "rtl"]] as const) {
    const errors = await open(page, `/components?lang=${lang}&dir=${dir}`);
    await expect(gallery(page)).toHaveAttribute("lang", lang);
    await expect(gallery(page)).toHaveAttribute("dir", dir);
    if (lang === "ar") {
      await expect(page.getByRole("heading", { name: "معرض المكونات", exact: true })).toBeVisible();
      await expect(page.getByRole("button", { name: "إضافة مستلم", exact: true })).toBeEnabled();
      await expect(page.locator("#gallery-formatted-amount")).toContainText("١٬٢٣٤٫٥٠ €");
    }
    for (const [layout, width, colorScheme] of [["wide-light", 1280, "light"], ["narrow-dark", 360, "dark"]] as const) {
      await page.setViewportSize({ width, height: 1000 });
      await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
      expect((await audit(page)).violations).toEqual([]);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      const path = test.info().outputPath(`components-${lang}-${dir}-${layout}.png`);
      await page.screenshot({ path, fullPage: true });
      await test.info().attach(`components-${lang}-${dir}-${layout}`, { path, contentType: "image/png" });
    }
    expect(errors).toEqual([]);
  }
  const opener = page.getByRole("link", { name: "فتح مساعدة المعرض", exact: true });
  await opener.click();
  await expect(page.getByRole("dialog", { name: "عن هذا المعرض", exact: true })).toBeVisible();
  expect((await audit(page)).violations).toEqual([]);
  const path = test.info().outputPath("components-dialog-ar-rtl-narrow-dark.png");
  await page.screenshot({ path });
  await test.info().attach("components-dialog-ar-rtl-narrow-dark", { path, contentType: "image/png" });
});
