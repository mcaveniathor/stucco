import { type Page, type Response } from "@playwright/test";
import { test, expect } from "./fixtures";
import { audit } from "../audit";

// Role names exclude the aria-hidden required marker; label text includes it.
const customer = (page: Page) => page.getByRole("textbox", { name: "Customer", exact: true });
const notebook = (page: Page) => page.getByRole("textbox", { name: "Notebook quantity", exact: true });
const organizer = (page: Page) => page.getByRole("textbox", { name: "Desk organizer quantity", exact: true });
const revision = (page: Page) => page.locator('input[name="expected_revision"]');
const saveButton = (page: Page) => page.getByRole("button", { name: "Save order", exact: true });
const total = (one: number, two: number) => `$${((one * 1200 + two * 3500) / 100).toFixed(2)}`;
const nativePost = (response: Response) =>
  response.request().method() === "POST" && new URL(response.url()).pathname === "/orders/1/edit";

async function openEditor(page: Page) {
  await page.goto("/orders/1/edit");
  await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
}

async function expectSaved(page: Page) {
  await expect(page.getByRole("status").filter({ hasText: "Order saved." })).toBeVisible();
  await expect(saveButton(page)).toBeEnabled();
}

async function draft(page: Page) {
  return {
    customer: await customer(page).inputValue(),
    quantity_one: await notebook(page).inputValue(),
    quantity_two: await organizer(page).inputValue(),
    expected_revision: await revision(page).inputValue(),
    _csrf: await page.locator('form[action^="/orders"] input[name="_csrf"]').inputValue(),
    expected_user_id: await page.locator('input[name="expected_user_id"]').inputValue(),
    expected_tenant_id: await page.locator('input[name="expected_tenant_id"]').inputValue(),
    expected_generation: await page.locator('input[name="expected_generation"]').inputValue(),
  };
}

async function delayHydration(page: Page) {
  let release!: () => void;
  const ready = new Promise<void>((resolve) => { release = resolve; });
  await page.route("**/pkg/**", async (route) => {
    await ready;
    // A native form navigation may cancel the first page's pending module.
    await route.continue().catch(() => {});
  });
  return release;
}

test("Rust quantities update the total locally and save through the server", async ({ page }) => {
  await openEditor(page);
  const before = BigInt(await revision(page).inputValue());
  const requests: string[] = [];
  page.on("request", (request) => {
    if (request.isNavigationRequest() || new URL(request.url()).pathname.startsWith("/api/")) {
      requests.push(request.url());
    }
  });

  await notebook(page).fill("3");
  await organizer(page).fill("4");
  await expect(page.locator("output")).toHaveText(total(3, 4));
  expect(requests).toEqual([]);

  await customer(page).fill(`Saved by ${test.info().project.name}`);
  await saveButton(page).click();
  await expectSaved(page);
  expect(BigInt(await revision(page).inputValue())).toBe(before + 1n);
  expect(requests.some((url) => new URL(url).pathname.startsWith("/api/"))).toBe(true);
  await page.reload();
  await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
  await expect(customer(page)).toHaveValue(`Saved by ${test.info().project.name}`);
  await expect(notebook(page)).toHaveValue("3");
  await expect(organizer(page)).toHaveValue("4");
});

test("server validation preserves invalid text and exposes the field error", async ({ page }) => {
  await openEditor(page);
  const before = await revision(page).inputValue();
  await notebook(page).fill("2x");
  await saveButton(page).click();

  await expect(notebook(page)).toHaveAttribute("aria-invalid", "true");
  await expect(page.getByRole("alert", { name: "There is a problem" })).toBeFocused();
  await expect(notebook(page)).toHaveAttribute("aria-describedby", "quantity-one-hint quantity-one-error");
  await expect(page.locator("#quantity-one-error")).toHaveText("Error: Enter a whole quantity from 1 to 999");
  await expect(notebook(page)).toHaveValue("2x");
  await expect(revision(page)).toHaveValue(before);
  await expect(page.locator("output")).toHaveText("Enter valid quantities");
  expect((await audit(page)).violations).toEqual([]);
});

test("two tabs preserve a stale draft and revision after a conflicting save", async ({ page, context }) => {
  await openEditor(page);
  const staleRevision = await revision(page).inputValue();
  const other = await context.newPage();
  try {
    await openEditor(other);
    await expect(revision(other)).toHaveValue(staleRevision);
    await customer(other).fill(`Other editor ${test.info().project.name}`);
    await saveButton(other).click();
    await expectSaved(other);

    const mine = `Unsaved draft ${test.info().project.name}`;
    await customer(page).fill(mine);
    await notebook(page).fill("7");
    await saveButton(page).click();
    await expect(page.getByRole("alert")).toContainText("This order changed");
    await expect(page.getByRole("alert", { name: "There is a problem" })).toBeFocused();
    await expect(customer(page)).toHaveValue(mine);
    await expect(notebook(page)).toHaveValue("7");
    await expect(revision(page)).toHaveValue(staleRevision);
    await saveButton(page).click();
    await expect(page.getByRole("alert")).toContainText("Your edits are still here");
    await expect(revision(page)).toHaveValue(staleRevision);

    await other.reload();
    await expect(customer(other)).toHaveValue(`Other editor ${test.info().project.name}`);
  } finally {
    await other.close();
  }
});

test.describe("native form submissions without JavaScript", () => {
  test.use({ javaScriptEnabled: false });

  test("a successful POST returns 303 and saves the entered values", async ({ page }) => {
    await page.goto("/orders/1/edit");
    const before = BigInt(await revision(page).inputValue());
    await customer(page).fill(`Native editor ${test.info().project.name}`);
    await notebook(page).fill("5");
    await organizer(page).fill("2");
    const posted = page.waitForResponse(nativePost);
    await saveButton(page).click();
    expect((await posted).status()).toBe(303);
    await expect(page).toHaveURL(/\/orders\/1$/);
    await expect(page.getByRole("heading", { name: "Order #1", exact: true })).toBeVisible();
    await expect(page.getByText(`Native editor ${test.info().project.name}`, { exact: true })).toBeVisible();
    await expect(page.locator("output")).toHaveText(total(5, 2));
    await page.getByRole("link", { name: "Edit order", exact: true }).click();
    await expect(customer(page)).toHaveValue(`Native editor ${test.info().project.name}`);
    await expect(revision(page)).toHaveValue((before + 1n).toString());
    await expect(page.locator('[data-hydrated="false"]')).toBeVisible();
  });

  test("422 and 409 responses retain values and the submitted revision", async ({ page, context }) => {
    await page.goto("/orders/1/edit");
    const staleRevision = await revision(page).inputValue();
    await notebook(page).fill("not a quantity");
    const invalid = page.waitForResponse(nativePost);
    await saveButton(page).click();
    expect((await invalid).status()).toBe(422);
    await expect(notebook(page)).toHaveValue("not a quantity");
    await expect(notebook(page)).toHaveAttribute("aria-invalid", "true");
    await expect(revision(page)).toHaveValue(staleRevision);

    const other = await context.newPage();
    try {
      await other.goto("/orders/1/edit");
      await customer(other).fill(`Native other ${test.info().project.name}`);
      const saved = other.waitForResponse(nativePost);
      await saveButton(other).click();
      expect((await saved).status()).toBe(303);

      await customer(page).fill(`Native unsaved ${test.info().project.name}`);
      await notebook(page).fill("6");
      const conflict = page.waitForResponse(nativePost);
      await saveButton(page).click();
      expect((await conflict).status()).toBe(409);
      await expect(page.getByRole("alert")).toContainText("This order changed");
      await expect(customer(page)).toHaveValue(`Native unsaved ${test.info().project.name}`);
      await expect(notebook(page)).toHaveValue("6");
      await expect(revision(page)).toHaveValue(staleRevision);
    } finally {
      await other.close();
    }
  });
});

test("hydration adopts text entered while the WASM package is delayed", async ({ page }) => {
  const release = await delayHydration(page);
  try {
    await page.goto("/orders/1/edit", { waitUntil: "commit" });
    await expect(page.locator('[data-hydrated="false"]')).toBeVisible();
    await customer(page).fill(`Before hydration ${test.info().project.name}`);
    await notebook(page).fill("7");
    await organizer(page).fill("3");
    release();
    await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
    await expect(customer(page)).toHaveValue(`Before hydration ${test.info().project.name}`);
    await expect(notebook(page)).toHaveValue("7");
    await expect(organizer(page)).toHaveValue("3");
    await expect(organizer(page)).toBeFocused();
    await organizer(page).press("End");
    await page.keyboard.type("4");
    await expect(organizer(page)).toHaveValue("34");
    await organizer(page).fill("3");
    await expect(page.locator("output")).toHaveText(total(7, 3));
    await saveButton(page).click();
    await expectSaved(page);
    await page.reload();
    await expect(customer(page)).toHaveValue(`Before hydration ${test.info().project.name}`);
    await expect(page.locator("output")).toHaveText(total(7, 3));
  } finally {
    release();
  }
});

test("list hydration preserves the active search field and its newly typed query", async ({ page }) => {
  const release = await delayHydration(page);
  try {
    await page.goto("/orders?q=Grace", { waitUntil: "commit" });
    const search = page.getByRole("textbox", { name: "Search customers", exact: true });
    await expect(page.locator('[data-hydrated="false"]')).toBeVisible();
    await search.fill("Barbar");
    release();
    await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
    await expect(search).toBeFocused();
    await page.keyboard.type("a");
    await expect(search).toHaveValue("Barbara");
    await search.press("Enter");
    await expect.poll(() => new URL(page.url()).searchParams.get("q")).toBe("Barbara");
    await expect(page.locator("table tbody tr")).toHaveCount(1);
    await expect(page.locator("table tbody tr")).toContainText("Barbara Liskov");
    await expect(page.getByRole("heading", { name: "Order results", exact: true })).toBeFocused();
  } finally {
    release();
  }
});

test("a native invalid response hydrates without replacing the submitted draft", async ({ page }) => {
  const release = await delayHydration(page);
  try {
    await page.goto("/orders/1/edit", { waitUntil: "commit" });
    const before = await revision(page).inputValue();
    await customer(page).fill("Keep this submitted name");
    await notebook(page).fill("invalid before hydration");
    const posted = page.waitForResponse(nativePost);
    await saveButton(page).click({ noWaitAfter: true });
    expect((await posted).status()).toBe(422);
    await expect(notebook(page)).toHaveValue("invalid before hydration");
    await expect(notebook(page)).toHaveAttribute("aria-invalid", "true");
    release();
    await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
    await expect(customer(page)).toHaveValue("Keep this submitted name");
    await expect(notebook(page)).toHaveValue("invalid before hydration");
    await expect(revision(page)).toHaveValue(before);
    await expect(page.getByRole("alert", { name: "There is a problem" })).toBeFocused();
    await notebook(page).fill("5");
    await organizer(page).fill("2");
    await saveButton(page).click();
    await expectSaved(page);
    await expect(page.locator("output")).toHaveText(total(5, 2));
  } finally {
    release();
  }
});

test("missing or invalid CSRF tokens reject native writes", async ({ page }) => {
  await openEditor(page);
  const state = await draft(page);
  const { _csrf, ...withoutToken } = state;
  const missing = await page.request.post("/orders/1/edit", { form: withoutToken });
  expect(missing.status()).toBe(403);
  const invalid = await page.request.post("/orders/1/edit", { form: { ...state, _csrf: "invalid" } });
  expect(invalid.status()).toBe(403);
  const crossOrigin = await page.request.post("/orders/1/edit", {
    headers: { origin: "https://unrelated.example" },
    form: { ...state, _csrf },
  });
  expect(crossOrigin.status()).toBe(403);
  await page.reload();
  await expect(revision(page)).toHaveValue(state.expected_revision);
});

test("Enter preserves focus while saving and repeated submits send one mutation", async ({ page }) => {
  await openEditor(page);
  const before = BigInt(await revision(page).inputValue());
  let release!: () => void;
  const ready = new Promise<void>((resolve) => { release = resolve; });
  let mutationCount = 0;
  await page.route("**/api/**", async (route) => {
    if (route.request().method() !== "POST" || !route.request().postData()?.includes("quantity_one=")) {
      await route.continue();
      return;
    }
    mutationCount++;
    await ready;
    await route.continue();
  });
  try {
    await notebook(page).fill("6");
    await notebook(page).press("Enter");
    await expect(page.locator('form[action^="/orders"]')).toHaveAttribute("aria-busy", "true");
    await expect(notebook(page)).toBeFocused();
    await expect(notebook(page)).toHaveJSProperty("readOnly", true);
    await expect(notebook(page)).toHaveJSProperty("disabled", false);
    await expect(page.getByRole("button", { name: "Saving…" })).toHaveAttribute("aria-disabled", "true");
    await notebook(page).press("Enter");
    await notebook(page).press("Enter");
    expect(mutationCount).toBe(1);
    release();
    await expectSaved(page);
    await expect(notebook(page)).toBeFocused();
    await expect(notebook(page)).toHaveJSProperty("readOnly", false);
    await expect(revision(page)).toHaveValue((before + 1n).toString());
    expect(mutationCount).toBe(1);
  } finally {
    release();
  }
});

test("reload and browser Back load current values without restoring an older draft", async ({ page, context }) => {
  await openEditor(page);
  await customer(page).fill("Unsaved before reload");
  await notebook(page).fill("9");
  const other = await context.newPage();
  try {
    await openEditor(other);
    await customer(other).fill(`Current after reload ${test.info().project.name}`);
    await notebook(other).fill("4");
    await saveButton(other).click();
    await expectSaved(other);
    const currentRevision = await revision(other).inputValue();

    page.once("dialog", (dialog) => dialog.accept());
    await page.reload();
    await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
    await expect(customer(page)).toHaveValue(`Current after reload ${test.info().project.name}`);
    await expect(notebook(page)).toHaveValue("4");
    await expect(revision(page)).toHaveValue(currentRevision);

    await customer(page).fill("Unsaved before Back");
    await notebook(page).fill("8");
    page.once("dialog", (dialog) => dialog.accept());
    await page.goto("/theme.css");
    await customer(other).fill(`Current after Back ${test.info().project.name}`);
    await notebook(other).fill("3");
    await saveButton(other).click();
    await expectSaved(other);
    const latestRevision = await revision(other).inputValue();
    await page.goBack();
    await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
    await expect(customer(page)).toHaveValue(`Current after Back ${test.info().project.name}`);
    await expect(notebook(page)).toHaveValue("3");
    await expect(revision(page)).toHaveValue(latestRevision);
  } finally {
    await other.close();
  }
});

test("a lost response after commit clears old errors and reconciles by reloading", async ({ page }) => {
  await openEditor(page);
  await notebook(page).fill("invalid");
  await saveButton(page).click();
  await expect(notebook(page)).toHaveAttribute("aria-invalid", "true");
  const before = BigInt(await revision(page).inputValue());
  const name = `Committed without response ${test.info().project.name}`;
  await customer(page).fill(name);
  await notebook(page).fill("6");
  await organizer(page).fill("2");

  let mutationCount = 0;
  let committedStatus: number | undefined;
  await page.route("**/api/**", async (route) => {
    if (route.request().method() !== "POST" || !route.request().postData()?.includes("quantity_one=")) {
      await route.continue();
      return;
    }
    mutationCount++;
    // Execute the real transaction, then lose its response at the browser boundary.
    const response = await route.fetch();
    committedStatus = response.status();
    await route.abort("failed");
  });
  await saveButton(page).click();
  await expect(page.getByRole("status").filter({ hasText: "The save outcome is unknown" })).toBeVisible();
  expect(committedStatus).toBe(200);
  expect(mutationCount).toBe(1);
  await expect(customer(page)).toHaveValue(name);
  await expect(notebook(page)).toHaveValue("6");
  await expect(revision(page)).toHaveValue(before.toString());
  await expect(notebook(page)).not.toHaveAttribute("aria-invalid", "true");
  await expect(page.locator("#quantity-one-error")).toHaveCount(0);
  await expect(page.getByRole("alert")).toHaveCount(0);
  await page.unrouteAll();
  await page.getByRole("link", { name: "Reload saved order" }).click();
  await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
  await expect(customer(page)).toHaveValue(name);
  await expect(page.locator("output")).toHaveText(total(6, 2));
  await expect(revision(page)).toHaveValue((before + 1n).toString());
});

test("typed mutations enforce CSRF header syntax and require the matching cookie", async ({ page, browser }) => {
  await openEditor(page);
  const mutation = page.waitForRequest((request) => request.method() === "POST"
    && new URL(request.url()).pathname.startsWith("/api/")
    && !!request.postData()?.includes("quantity_one="));
  await saveButton(page).click();
  const endpoint = (await mutation).url();
  await expectSaved(page);
  const state = { ...await draft(page), id: "1" };
  const { _csrf, ...withoutToken } = state;

  // A valid documented header takes precedence over the form token. Invalid
  // quantity text prevents this protocol check from mutating the order.
  const acceptedHeader = await page.request.post(endpoint, {
    headers: { "Stucco-Csrf": _csrf },
    form: { ...state, _csrf: "not-the-cookie-token", quantity_one: "invalid" },
  });
  expect(acceptedHeader.status()).toBe(200);
  expect(await acceptedHeader.text()).toContain("Validation");

  const malformed = await page.request.post(endpoint, {
    headers: { "Stucco-Csrf": "malformed-token" }, form: state,
  });
  expect(malformed.status()).toBe(403);
  const wrongHeader = await page.request.post(endpoint, {
    headers: { "X-CSRF-Token": _csrf }, form: withoutToken,
  });
  expect(wrongHeader.status()).toBe(403);
  const noToken = await page.request.post(endpoint, { form: withoutToken });
  expect(noToken.status()).toBe(403);
  const crossOrigin = await page.request.post(endpoint, {
    headers: { origin: "https://unrelated.example" }, form: state,
  });
  expect(crossOrigin.status()).toBe(403);

  const anonymous = await browser.newContext();
  try {
    // Keep authentication so this still isolates CSRF-cookie rejection.
    await anonymous.addCookies((await page.context().cookies())
      .filter((cookie) => cookie.name !== "stucco-csrf" && cookie.name !== "__Host-stucco-csrf"));
    const noCookie = await anonymous.request.post(endpoint, {
      headers: { "Stucco-Csrf": _csrf }, form: state,
    });
    expect(noCookie.status()).toBe(403);
  } finally {
    await anonymous.close();
  }
  await page.reload();
  await expect(revision(page)).toHaveValue(state.expected_revision);
});

test("hostile customer text stays literal through save, SSR and hydration", async ({ page }) => {
  const hostile = "</script><script>globalThis.__stuccoInjection=true</script>";
  expect(hostile.length).toBeLessThanOrEqual(80);
  await page.addInitScript(() => { Reflect.set(globalThis, "__stuccoInjection", false); });
  await openEditor(page);
  await customer(page).fill(hostile);
  await saveButton(page).click();
  await expectSaved(page);
  await page.reload();
  await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
  await expect(customer(page)).toHaveValue(hostile);
  expect(await page.evaluate(() => Reflect.get(globalThis, "__stuccoInjection"))).toBe(false);

  // Leave the demonstration readable through the same user-facing save path.
  await customer(page).fill(`Accessible preview ${test.info().project.name}`);
  await saveButton(page).click();
  await expectSaved(page);
});

test("the editor is accessible in wide light and narrow dark layouts", async ({ page }, testInfo) => {
  for (const [name, width, colorScheme] of [
    ["wide-light", 1280, "light"],
    ["narrow-dark", 360, "dark"],
  ] as const) {
    await page.setViewportSize({ width, height: 900 });
    await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
    await openEditor(page);
    expect((await audit(page)).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const path = testInfo.outputPath(`orders-${name}.png`);
    await page.screenshot({ path, fullPage: true });
    await testInfo.attach(`orders-${name}`, { path, contentType: "image/png" });
  }
});
