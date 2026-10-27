import { type Page } from "@playwright/test";
import { test, expect } from "./fixtures";

const customer = (page: Page) => page.getByRole("textbox", { name: "Customer", exact: true });
const notebook = (page: Page) => page.getByRole("textbox", { name: "Notebook quantity", exact: true });
const organizer = (page: Page) => page.getByRole("textbox", { name: "Desk organizer quantity", exact: true });
const revision = (page: Page) => page.locator('input[name="expected_revision"]');
const rows = (page: Page) => page.locator("table tbody tr");
const createButton = (page: Page) => page.getByRole("button", { name: "Create order", exact: true });

function uniqueName(prefix: string) {
  return `${prefix} ${test.info().project.name} ${Date.now()}`;
}

async function ready(page: Page) {
  await expect(page.locator('main [data-hydrated="true"]')).toBeVisible();
}

async function search(page: Page, name: string) {
  await page.getByRole("textbox", { name: "Search customers", exact: true }).fill(name);
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await expect.poll(() => new URL(page.url()).searchParams.get("q")).toBe(name);
  await expect(page.getByRole("status").filter({ hasText: /orders · Page/ })).toContainText("Page 1 of 1");
}

async function createOrder(page: Page, name: string) {
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await ready(page);
  await customer(page).fill(name);
  await createButton(page).click();
  await expect(page).toHaveURL(/\/orders\/\d+$/);
  await ready(page);
  const id = Number(new URL(page.url()).pathname.split("/").at(-1));
  expect(id).toBeGreaterThan(12);
  return id;
}

async function removeOrder(page: Page, id: number) {
  // Only remove orders created by this test, through the real confirmation UI.
  expect(id).toBeGreaterThan(12);
  const response = await page.goto(`/orders/${id}/delete`);
  if (response?.status() === 404) return;
  await ready(page);
  await page.getByRole("button", { name: "Delete order", exact: true }).click();
  await expect(page).toHaveURL(/\/orders$/);
}

test("a deleted order keeps its raw draft until explicit discard clears the unload warning", async ({ page, context }) => {
  const dialogs: string[] = [];
  page.on("dialog", async (dialog) => {
    dialogs.push(dialog.type());
    await dialog.accept();
  });
  await page.goto("/orders");
  await ready(page);
  const id = await createOrder(page, uniqueName("Deleted draft"));
  const other = await context.newPage();
  let deleted = false;
  try {
    await page.getByRole("link", { name: "Edit order", exact: true }).click();
    await ready(page);
    const rawName = "  Keep <this> & my draft  ";
    await customer(page).fill(rawName);
    await notebook(page).fill("7x");
    await organizer(page).fill(" 03 ");
    const documents: string[] = [];
    page.on("request", (request) => {
      if (request.isNavigationRequest() && request.frame() === page.mainFrame()) {
        documents.push(request.url());
      }
    });
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await ready(page);

    await removeOrder(other, id);
    deleted = true;
    await page.goBack();
    await expect(page).toHaveURL(new RegExp(`/orders/${id}/edit$`));
    await expect(page.getByRole("heading", { name: "Order not found", exact: true })).toBeVisible();
    await expect(page.getByText("Your unsaved draft is retained below.", { exact: false })).toBeVisible();
    expect(await page.locator("dl dd").allTextContents()).toEqual([rawName, "7x", " 03 "]);
    await expect(page.locator('form[action^="/orders"]')).toHaveCount(0);
    expect(documents).toEqual([]);

    await page.getByRole("button", { name: "Discard deleted order draft", exact: true }).click();
    await expect(page.getByRole("button", { name: "Discard deleted order draft", exact: true })).toHaveCount(0);
    await expect(page.locator("dl")).toHaveCount(0);
    // The discard click gives the document user activation. Reload would show
    // beforeunload if the missing order's root-owned draft were still retained.
    const response = await page.reload();
    expect(response?.status()).toBe(404);
    await expect(page.getByRole("heading", { name: "Order not found", exact: true })).toBeVisible();
    expect(dialogs).toEqual([]);
  } finally {
    if (!deleted) await removeOrder(other, id);
    await other.close();
  }
});

test("a lost committed create response leaves a focused blocked button and reconciles without losing another draft", async ({ page, context }) => {
  page.on("dialog", async (dialog) => { await dialog.accept(); });
  await page.goto("/orders");
  await ready(page);
  const originalName = uniqueName("Other recovery draft");
  const otherId = await createOrder(page, originalName);
  const cleanup = await context.newPage();
  const name = uniqueName("Lost create response");
  let createdId: number | undefined;
  let mutations = 0;
  let committedStatus: number | undefined;
  try {
    await page.getByRole("link", { name: "Edit order", exact: true }).click();
    await ready(page);
    const retainedName = `${originalName} unsaved`;
    const retainedRevision = await revision(page).inputValue();
    await customer(page).fill(retainedName);
    await notebook(page).fill("9x");
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await ready(page);
    await page.getByRole("link", { name: "New order", exact: true }).click();
    await ready(page);
    await customer(page).fill(name);
    await notebook(page).fill("3");
    await organizer(page).fill("2");
    await page.route("**/api/**", async (route) => {
      if (route.request().method() !== "POST") {
        await route.continue();
        return;
      }
      mutations++;
      if (mutations === 1) {
        // Commit through the real service, then lose only the delivery to WASM.
        const response = await route.fetch();
        committedStatus = response.status();
      }
      await route.abort("failed");
    });
    await createButton(page).focus();
    await createButton(page).press("Enter");
    const unknown = page.getByRole("status").filter({ hasText: "The creation outcome is unknown" });
    await expect(unknown).toBeVisible();
    expect(committedStatus).toBe(200);
    await expect(createButton(page)).toBeFocused();
    await expect(createButton(page)).toHaveAttribute("aria-disabled", "true");
    await expect(createButton(page)).toHaveAttribute("aria-busy", "false");
    await expect(createButton(page)).toHaveJSProperty("disabled", false);
    await expect(customer(page)).toHaveValue(name);
    await expect(notebook(page)).toHaveValue("3");
    await expect(organizer(page)).toHaveValue("2");
    for (const field of [customer(page), notebook(page), organizer(page)]) {
      await expect(field).toHaveJSProperty("readOnly", true);
      await expect(field).toHaveJSProperty("disabled", false);
    }
    await expect(revision(page)).toHaveValue("1");
    await createButton(page).press("Enter");
    await createButton(page).press("Enter");
    await expect(createButton(page)).toBeFocused();
    await customer(page).press("Enter");
    await expect(unknown).toBeVisible();
    expect(mutations).toBe(1);

    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await ready(page);
    await search(page, name);
    await expect(rows(page)).toHaveCount(1);
    await expect(rows(page)).toContainText(name);
    const href = await rows(page).getByRole("link", { name: /^Order #\d+$/ }).getAttribute("href");
    createdId = Number(href!.split("/").at(-1));
    expect(createdId).toBeGreaterThan(12);
    await rows(page).getByRole("link", { name: `Order #${createdId}`, exact: true }).click();
    await ready(page);
    await expect(page.locator("output")).toHaveText("$106.00");
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await ready(page);
    await page.getByRole("link", { name: "New order", exact: true }).click();
    await ready(page);
    await expect(customer(page)).toHaveValue(name);
    await expect(createButton(page)).toHaveAttribute("aria-disabled", "true");
    await page.getByRole("link", { name: "Discard draft and start again", exact: true }).click();
    await expect(customer(page)).toHaveValue("");
    await expect(notebook(page)).toHaveValue("1");
    await expect(organizer(page)).toHaveValue("1");
    await expect(createButton(page)).toHaveAttribute("aria-disabled", "false");
    await expect(unknown).toHaveCount(0);
    expect(mutations).toBe(1);

    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await ready(page);
    await search(page, originalName);
    await expect(rows(page)).toHaveCount(1);
    await page.getByRole("link", { name: `Order #${otherId}`, exact: true }).click();
    await page.getByRole("link", { name: "Edit order", exact: true }).click();
    await ready(page);
    await expect(customer(page)).toHaveValue(retainedName);
    await expect(notebook(page)).toHaveValue("9x");
    await expect(revision(page)).toHaveValue(retainedRevision);
    expect(mutations).toBe(1);
  } finally {
    await page.unrouteAll({ behavior: "wait" });
    if (createdId === undefined && committedStatus === 200) {
      await cleanup.goto("/orders");
      await ready(cleanup);
      await search(cleanup, name);
      const match = rows(cleanup).getByRole("link", { name: /^Order #\d+$/ });
      if (await match.count()) {
        const href = await match.getAttribute("href");
        if (href) createdId = Number(href.split("/").at(-1));
      }
    }
    if (createdId !== undefined) await removeOrder(cleanup, createdId);
    await removeOrder(cleanup, otherId);
    await cleanup.close();
  }
});

for (const outcome of ["validation", "success"] as const) {
  test(`a returned create form adopts its late ${outcome} response`, async ({ page, context }) => {
    page.on("dialog", async (dialog) => { await dialog.accept(); });
    await page.goto("/orders");
    await ready(page);
    await page.getByRole("link", { name: "New order", exact: true }).click();
    await ready(page);
    const name = uniqueName(`Late ${outcome}`);
    await customer(page).fill(name);
    await notebook(page).fill("2x");
    await organizer(page).fill("2");
    if (outcome === "success") {
      // A prior failure must not be copied into the next unknown snapshot.
      await createButton(page).click();
      await expect(notebook(page)).toHaveAttribute("aria-invalid", "true");
      await notebook(page).fill("3");
    }
    let release!: () => void;
    const held = new Promise<void>((resolve) => { release = resolve; });
    let fetched!: () => void;
    const responseFetched = new Promise<void>((resolve) => { fetched = resolve; });
    let completed!: () => void;
    const responseReleased = new Promise<void>((resolve) => { completed = resolve; });
    let mutations = 0;
    let createdId: number | undefined;
    const cleanup = await context.newPage();
    await page.route("**/api/**", async (route) => {
      if (route.request().method() !== "POST") {
        await route.continue();
        return;
      }
      mutations++;
      if (mutations > 1) {
        await route.continue();
        return;
      }
      const response = await route.fetch();
      expect(response.status()).toBe(200);
      fetched();
      await held;
      try { await route.fulfill({ response }); }
      catch { /* Assertions below still require the returned form to settle. */ }
      finally { completed(); }
    });
    try {
      await createButton(page).click();
      await responseFetched;
      await expect(page.locator('form[action^="/orders"]')).toHaveAttribute("aria-busy", "true");
      await page.getByRole("link", { name: "Back to orders", exact: true }).click();
      await ready(page);
      await page.getByRole("link", { name: "New order", exact: true }).click();
      await ready(page);
      await expect(customer(page)).toHaveValue(name);
      await expect(notebook(page)).toHaveValue(outcome === "validation" ? "2x" : "3");
      await expect(createButton(page)).toHaveAttribute("aria-disabled", "true");
      await expect(createButton(page)).toHaveAttribute("aria-busy", "false");
      for (const field of [customer(page), notebook(page), organizer(page)]) {
        await expect(field).toHaveJSProperty("readOnly", true);
      }
      await expect(page.getByRole("status").filter({ hasText: "The creation outcome is unknown" })).toBeVisible();
      await expect(page.getByRole("alert", { name: "There is a problem" })).toHaveCount(0);
      await expect(notebook(page)).not.toHaveAttribute("aria-invalid", "true");
      await createButton(page).press("Enter");
      expect(mutations).toBe(1);

      release();
      await responseReleased;
      if (outcome === "validation") {
        const problem = page.getByRole("alert", { name: "There is a problem" });
        await expect(problem).toContainText("Check the highlighted fields");
        await expect(problem).toBeFocused();
        await expect(notebook(page)).toHaveValue("2x");
        await expect(notebook(page)).toHaveAttribute("aria-invalid", "true");
        await expect(revision(page)).toHaveValue("1");
        await expect(createButton(page)).toHaveAttribute("aria-disabled", "false");
        for (const field of [customer(page), notebook(page), organizer(page)]) {
          await expect(field).toHaveJSProperty("readOnly", false);
        }
        await notebook(page).fill("3");
        await createButton(page).click();
        await expect(page).toHaveURL(/\/orders\/\d+$/);
        await ready(page);
        createdId = Number(new URL(page.url()).pathname.split("/").at(-1));
        expect(mutations).toBe(2);
      } else {
        await expect(page.getByRole("status").filter({ hasText: "Your earlier order was created" })).toBeVisible();
        await expect(page.getByRole("status").filter({ hasText: "The creation outcome is unknown" })).toHaveCount(0);
        await expect(createButton(page)).toHaveAttribute("aria-disabled", "true");
        await expect(notebook(page)).toHaveJSProperty("readOnly", true);
        await createButton(page).press("Enter");
        expect(mutations).toBe(1);
        await page.getByRole("link", { name: "Discard draft and start again", exact: true }).click();
        await expect(customer(page)).toHaveValue("");
        await expect(notebook(page)).toHaveValue("1");
        await expect(organizer(page)).toHaveValue("1");
        await expect(createButton(page)).toHaveAttribute("aria-disabled", "false");
        await expect(notebook(page)).toHaveJSProperty("readOnly", false);
        await expect(page.locator('form[action^="/orders"] [role="status"]')).toHaveText("");
        await page.getByRole("link", { name: "Back to orders", exact: true }).click();
        await ready(page);
        await search(page, name);
        await expect(rows(page)).toHaveCount(1);
        const href = await rows(page).getByRole("link", { name: /^Order #\d+$/ }).getAttribute("href");
        createdId = Number(href!.split("/").at(-1));
        expect(mutations).toBe(1);
      }
      expect(createdId).toBeGreaterThan(12);
    } finally {
      release();
      await page.unrouteAll({ behavior: "wait" });
      if (createdId === undefined) {
        await cleanup.goto("/orders");
        await ready(cleanup);
        await search(cleanup, name);
        const match = rows(cleanup).getByRole("link", { name: /^Order #\d+$/ });
        if (await match.count()) {
          const href = await match.getAttribute("href");
          if (href) createdId = Number(href.split("/").at(-1));
        }
      }
      if (createdId !== undefined) await removeOrder(cleanup, createdId);
      await cleanup.close();
    }
  });
}
