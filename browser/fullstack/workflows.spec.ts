import { type Locator, type Page, type Request, type Response } from "@playwright/test";
import { test, expect } from "./fixtures";
import { audit } from "../audit";

const customer = (page: Page) => page.getByRole("textbox", { name: "Customer", exact: true });
const notebook = (page: Page) => page.getByRole("textbox", { name: "Notebook quantity", exact: true });
const organizer = (page: Page) => page.getByRole("textbox", { name: "Desk organizer quantity", exact: true });
const revision = (page: Page) => page.locator('input[name="expected_revision"]');
const searchBox = (page: Page) => page.getByRole("textbox", { name: "Search customers", exact: true });
const rows = (page: Page) => page.locator("table tbody tr");
const problem = (page: Page) => page.getByRole("alert", { name: "There is a problem" });
const listStatus = (page: Page) => page.locator('[role="status"]').filter({ hasText: /orders · Page/ });
const nativePost = (path: string) => (response: Response) =>
  response.request().method() === "POST" && new URL(response.url()).pathname === path;
const apiRead = (request: Request) => request.method() === "GET"
  && new URL(request.url()).pathname.startsWith("/api/");

test.beforeEach(async ({ page }) => {
  page.on("dialog", async (dialog) => {
    if (dialog.type() === "beforeunload") await dialog.accept();
    else await dialog.dismiss();
  });
});

function uniqueName(prefix: string) {
  return `${prefix} ${test.info().project.name} ${Date.now()}`;
}

async function ready(page: Page, hydrated = true) {
  await expect(page.locator(`main [data-hydrated="${hydrated}"]`)).toBeVisible();
}

async function openList(page: Page, hydrated = true) {
  await page.goto("/orders");
  await expect(page.getByRole("heading", { name: "Orders", exact: true })).toBeVisible();
  await expect(page).toHaveTitle(/^Orders(?:\s|$)/);
  await ready(page, hydrated);
  await expect(listStatus(page)).toBeVisible();
}

function documentRequests(page: Page) {
  const requests: string[] = [];
  page.on("request", (request) => {
    if (request.isNavigationRequest() && request.frame() === page.mainFrame()) {
      requests.push(request.url());
    }
  });
  return requests;
}

async function search(page: Page, query: string) {
  await searchBox(page).fill(query);
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await expect.poll(() => new URL(page.url()).searchParams.get("q")).toBe(query);
}

async function createOrder(page: Page, name: string, hydrated = true) {
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Create order", exact: true })).toBeVisible();
  await expect(page).toHaveTitle(/Create order/);
  await ready(page, hydrated);
  await customer(page).fill(name);
  await notebook(page).fill("3");
  await organizer(page).fill("2");
  const posted = hydrated ? undefined : page.waitForResponse(nativePost("/orders/new"));
  await page.getByRole("button", { name: "Create order", exact: true }).click();
  if (posted) expect((await posted).status()).toBe(303);
  await expect(page).toHaveURL(/\/orders\/\d+$/);
  const id = Number(new URL(page.url()).pathname.split("/").at(-1));
  expect(id).toBeGreaterThan(12);
  await expect(page.getByRole("heading", { name: `Order #${id}`, exact: true })).toBeVisible();
  await expect(page).toHaveTitle(new RegExp(`Order #${id}(?:\\D|$)`));
  await ready(page, hydrated);
  return id;
}

async function removeOrder(page: Page, id: number, hydrated = true) {
  // Cleanup follows the same confirmation form as a user and never touches seeds.
  expect(id).toBeGreaterThan(12);
  const response = await page.goto(`/orders/${id}/delete`);
  if (response?.status() === 404) return;
  await ready(page, hydrated);
  await page.getByRole("button", { name: "Delete order", exact: true }).click();
  await expect(page).toHaveURL(/\/orders$/);
}

async function scrollTableWithKeyboard(page: Page, region: Locator) {
  // Use native Tab navigation from the page's initial focus. No element.focus()
  // or scroll assignment can stand in for keyboard access to the overflow.
  for (let press = 0; press < 24; press++) {
    await page.keyboard.press("Tab");
    if (await region.evaluate(element => element === document.activeElement)) break;
  }
  await expect(region).toBeFocused();
  expect(await region.evaluate(element => element.scrollWidth - element.clientWidth)).toBeGreaterThan(1);
  const initial = await region.evaluate(element => element.scrollLeft);
  await page.keyboard.press("ArrowRight");
  await expect.poll(() => region.evaluate(element => element.scrollLeft)).toBeGreaterThan(initial);
  const right = await region.evaluate(element => element.scrollLeft);
  await page.keyboard.press("ArrowLeft");
  await expect.poll(() => region.evaluate(element => element.scrollLeft)).toBeLessThan(right);
  await expect(region).toBeFocused();
}

for (const hydrated of [true, false]) {
  test.describe(hydrated ? "hydrated workflows" : "native workflows without JavaScript", () => {
    test.use({ javaScriptEnabled: hydrated });

    test("narrow results and line items scroll with native keyboard controls", async ({ page }) => {
      // At 320px the card leaves less than 240px for the fixed catalogue's
      // nonwrapping columns. Assert real overflow before sending arrow keys.
      await page.setViewportSize({ width: 320, height: 900 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await openList(page, hydrated);
      await scrollTableWithKeyboard(page, page.getByRole("region", { name: "Orders matching your search", exact: true }));
      await page.goto("/orders/1");
      await ready(page, hydrated);
      await scrollTableWithKeyboard(page, page.getByRole("region", { name: "Line items", exact: true }));
    });

    test("create, inspect, edit and delete an order through the UI", async ({ page }) => {
      await openList(page, hydrated);
      const navigations = documentRequests(page);
      const name = uniqueName(hydrated ? "Live lifecycle" : "Native lifecycle");
      let id: number | undefined;
      let deleted = false;
      try {
        id = await createOrder(page, name, hydrated);
        await expect(page.locator(".page-heading")).toContainText(name);
        await expect(page.locator("output")).toHaveText("$106.00");
        await expect(page.getByText("Saved revision 1", { exact: true })).toBeVisible();

        await page.getByRole("link", { name: "Edit order", exact: true }).click();
        await ready(page, hydrated);
        await expect(page).toHaveTitle(new RegExp(`Edit order #${id}(?:\\D|$)`));
        await expect(revision(page)).toHaveValue("1");
        const updatedName = `${name} edited`;
        await customer(page).fill(updatedName);
        await notebook(page).fill("4");
        const posted = hydrated ? undefined : page.waitForResponse(nativePost(`/orders/${id}/edit`));
        await page.getByRole("button", { name: "Save order", exact: true }).click();
        if (posted) expect((await posted).status()).toBe(303);
        if (hydrated) {
          await expect(page.locator('[role="status"]').filter({ hasText: "Order saved." })).toHaveText("Order saved.");
          await expect(page).toHaveURL(new RegExp(`/orders/${id}/edit$`));
          await expect(revision(page)).toHaveValue("2");
          await page.getByRole("link", { name: "Back to orders", exact: true }).click();
          await search(page, updatedName);
          await expect(rows(page)).toHaveCount(1);
          await page.getByRole("link", { name: `Order #${id}`, exact: true }).click();
        }
        await expect(page.getByRole("heading", { name: `Order #${id}`, exact: true })).toBeVisible();
        await expect(page).toHaveTitle(new RegExp(`Order #${id}(?:\\D|$)`));
        await expect(page.locator(".page-heading")).toContainText(updatedName);
        await expect(page.getByText("Saved revision 2", { exact: true })).toBeVisible();
        await expect(page.locator("output")).toHaveText("$118.00");
        await page.getByRole("link", { name: "Delete order", exact: true }).click();
        await ready(page, hydrated);
        await expect(page.getByRole("heading", { name: `Delete order #${id}?`, exact: true })).toBeVisible();
        await expect(page).toHaveTitle(new RegExp(`Delete order #${id}(?:\\D|$)`));
        await expect(revision(page)).toHaveValue("2");
        const removed = hydrated ? undefined : page.waitForResponse(nativePost(`/orders/${id}/delete`));
        await page.getByRole("button", { name: "Delete order", exact: true }).click();
        if (removed) expect((await removed).status()).toBe(303);
        await expect(page).toHaveURL(/\/orders$/);
        deleted = true;
        await search(page, updatedName);
        await expect(page.getByText("No orders match this search.", { exact: false })).toBeVisible();
        await expect(rows(page)).toHaveCount(0);
        await expect(page.getByRole("navigation", { name: "Order pages", exact: true })).toHaveCount(0);
        await expect(page).toHaveTitle(/^Orders(?:\s|$)/);
        if (hydrated) expect(navigations).toEqual([]);
        else expect(navigations.length).toBeGreaterThan(0);
        const missing = await page.goto(`/orders/${id}`);
        expect(missing?.status()).toBe(404);
        await expect(page.getByRole("heading", { name: "Order not found", exact: true })).toBeVisible();
        await expect(page).toHaveTitle(/Order not found/);
      } finally {
        if (id !== undefined && !deleted) await removeOrder(page, id, hydrated);
      }
    });

    test("search, pagination and browser Back preserve the URL, input and results", async ({ page }) => {
      await openList(page, hydrated);
      const navigations = documentRequests(page);
      const liveRegion = hydrated ? await listStatus(page).elementHandle() : null;
      const expectResultsFocus = async () => {
        if (!hydrated) return;
        await expect(page.getByRole("heading", { name: "Order results", exact: true })).toBeFocused();
        expect(await liveRegion!.evaluate((element) => element.isConnected)).toBe(true);
        expect(await listStatus(page).evaluate((element, original) => element === original, liveRegion!)).toBe(true);
      };
      await search(page, "a");
      await expect(listStatus(page)).toContainText("Page 1 of");
      await expect(rows(page)).toHaveCount(5);
      await expectResultsFocus();
      if (hydrated) {
        // Submitting the current URL still moves focus to the existing results.
        await page.getByRole("button", { name: "Search", exact: true }).click();
        await expectResultsFocus();
      }
      const first = await rows(page).allTextContents();
      await page.getByRole("link", { name: "Next page", exact: true }).click();
      await expect.poll(() => new URL(page.url()).searchParams.get("page")).toBe("2");
      await expect(listStatus(page)).toContainText("Page 2 of");
      await expectResultsFocus();
      const second = await rows(page).allTextContents();
      expect(second).not.toEqual(first);
      await expect(searchBox(page)).toHaveValue("a");
      await search(page, "Grace");
      await expect(rows(page)).toHaveCount(1);
      await expect(rows(page)).toContainText("Grace Hopper");
      await expect(page.getByRole("navigation", { name: "Order pages", exact: true })).toHaveCount(0);
      await expectResultsFocus();
      await page.goBack();
      await expect.poll(() => new URL(page.url()).searchParams.get("page")).toBe("2");
      await expect(searchBox(page)).toHaveValue("a");
      await expect(listStatus(page)).toContainText("Page 2 of");
      await expect(rows(page)).toHaveText(second);
      await expectResultsFocus();
      await page.goBack();
      await expect.poll(() => new URL(page.url()).searchParams.get("q")).toBe("a");
      await expect(searchBox(page)).toHaveValue("a");
      await expect(listStatus(page)).toContainText("Page 1 of");
      await expect(rows(page)).toHaveText(first);
      await expectResultsFocus();
      if (hydrated) expect(navigations).toEqual([]);
    });
  });
}

test("internal navigation keeps an unsaved draft and its original revision after another client saves", async ({ page, context }) => {
  await openList(page);
  const id = await createOrder(page, uniqueName("Retained draft"));
  const other = await context.newPage();
  try {
    await page.getByRole("link", { name: "Edit order", exact: true }).click();
    await ready(page);
    const originalRevision = await revision(page).inputValue();
    const mine = uniqueName("Still unsaved");
    await customer(page).fill(mine);
    await notebook(page).fill("7x");
    const navigations = documentRequests(page);
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await expect(listStatus(page)).toBeVisible();

    await other.goto(`/orders/${id}/edit`);
    await ready(other);
    await customer(other).fill(uniqueName("Other client saved"));
    await notebook(other).fill("4");
    await other.getByRole("button", { name: "Save order", exact: true }).click();
    await expect(other.locator('[role="status"]').filter({ hasText: "Order saved." })).toHaveText("Order saved.");
    await expect(revision(other)).toHaveValue((BigInt(originalRevision) + 1n).toString());

    await page.getByRole("link", { name: `Order #${id}`, exact: true }).click();
    await page.getByRole("link", { name: "Edit order", exact: true }).click();
    await ready(page);
    await expect(customer(page)).toHaveValue(mine);
    await expect(notebook(page)).toHaveValue("7x");
    await expect(revision(page)).toHaveValue(originalRevision);
    await notebook(page).fill("7");
    await page.getByRole("button", { name: "Save order", exact: true }).click();
    await expect(problem(page)).toContainText("This order changed");
    await expect(problem(page)).toBeFocused();
    await expect(customer(page)).toHaveValue(mine);
    await expect(revision(page)).toHaveValue(originalRevision);
    expect(navigations).toEqual([]);
  } finally {
    await other.close();
    await removeOrder(page, id);
  }
});

test("leaving a pending creation keeps the returned form blocked until its outcome is reconciled", async ({ page }) => {
  await openList(page);
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await ready(page);
  const name = uniqueName("Pending creation");
  await customer(page).fill(name);
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let fetched!: () => void;
  const committed = new Promise<void>((resolve) => { fetched = resolve; });
  let completed!: () => void;
  const responseReleased = new Promise<void>((resolve) => { completed = resolve; });
  let mutations = 0;
  let id: number | undefined;
  await page.route("**/api/**", async (route) => {
    if (route.request().method() !== "POST") { await route.continue(); return; }
    mutations++;
    if (mutations > 1) {
      await route.abort("failed");
      return;
    }
    const response = await route.fetch();
    expect(response.status()).toBe(200);
    fetched();
    await held;
    try { await route.fulfill({ response }); }
    catch { /* Leaving the editor may cancel delivery to its disposed action. */ }
    finally { completed(); }
  });
  const navigations = documentRequests(page);
  try {
    await page.getByRole("button", { name: "Create order", exact: true }).click();
    await committed;
    await expect(page.locator('form[action^="/orders"]')).toHaveAttribute("aria-busy", "true");
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await expect(listStatus(page)).toBeVisible();
    await page.getByRole("link", { name: "New order", exact: true }).click();
    await ready(page);
    await expect(customer(page)).toHaveValue(name);
    const blockedCreate = page.getByRole("button", { name: /^(Create order|Saving…)$/ });
    await expect(blockedCreate).toBeVisible();
    await expect(blockedCreate).toHaveAttribute("aria-disabled", "true");
    await blockedCreate.press("Enter");
    expect(mutations).toBe(1);
    release();
    await responseReleased;
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await search(page, name);
    await expect(rows(page)).toHaveCount(1);
    await expect(rows(page)).toContainText(name);
    const href = await rows(page).getByRole("link", { name: /^Order #\d+$/ }).getAttribute("href");
    id = Number(href!.split("/").at(-1));
    expect(id).toBeGreaterThan(12);
    expect(mutations).toBe(1);
    expect(navigations).toEqual([]);
  } finally {
    release();
    await page.unrouteAll({ behavior: "wait" });
    if (id === undefined) {
      // A failed assertion must still remove the order committed behind the held response.
      await openList(page);
      await search(page, name);
      await expect(listStatus(page)).toContainText("Page 1 of 1");
      const match = rows(page).getByRole("link", { name: /^Order #\d+$/ });
      if (await match.count()) {
        const href = await match.getAttribute("href");
        if (href) id = Number(href.split("/").at(-1));
      }
    }
    if (id !== undefined) await removeOrder(page, id);
  }
});

test("reloading an editor or deletion confirmation preserves another order's unsaved draft", async ({ page, context }) => {
  await openList(page);
  const ids: number[] = [];
  const other = await context.newPage();
  try {
    const first = await createOrder(page, uniqueName("Keep other draft"));
    ids.push(first);
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    const second = await createOrder(page, uniqueName("Reload this order"));
    ids.push(second);
    const navigations = documentRequests(page);
    const editFromList = async (id: number) => {
      await page.getByRole("link", { name: `Order #${id}`, exact: true }).click();
      await page.getByRole("link", { name: "Edit order", exact: true }).click();
      await ready(page);
    };
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await editFromList(first);
    const firstRevision = await revision(page).inputValue();
    const retainedName = uniqueName("Still retained");
    await customer(page).fill(retainedName);
    await notebook(page).fill("9x");
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await editFromList(second);
    await customer(page).fill("Discard only this draft");
    await notebook(page).fill("8");

    await other.goto(`/orders/${second}/edit`);
    await ready(other);
    const currentName = uniqueName("Saved current order");
    await customer(other).fill(currentName);
    await notebook(other).fill("6");
    await other.getByRole("button", { name: "Save order", exact: true }).click();
    await expect(other.locator('[role="status"]').filter({ hasText: "Order saved." })).toHaveText("Order saved.");
    const currentRevision = await revision(other).inputValue();
    await page.getByRole("link", { name: "Reload saved order", exact: true }).click();
    await expect(customer(page)).toHaveValue(currentName);
    await expect(notebook(page)).toHaveValue("6");
    await expect(revision(page)).toHaveValue(currentRevision);
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await editFromList(first);
    await expect(customer(page)).toHaveValue(retainedName);
    await expect(notebook(page)).toHaveValue("9x");
    await expect(revision(page)).toHaveValue(firstRevision);

    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await page.getByRole("link", { name: `Order #${second}`, exact: true }).click();
    await page.getByRole("link", { name: "Delete order", exact: true }).click();
    await ready(page);
    await expect(revision(page)).toHaveValue(currentRevision);
    const newerName = `${currentName} newer`;
    await customer(other).fill(newerName);
    await other.getByRole("button", { name: "Save order", exact: true }).click();
    await expect(revision(other)).toHaveValue((BigInt(currentRevision) + 1n).toString());
    await page.getByRole("link", { name: "Reload saved order", exact: true }).click();
    await expect(revision(page)).toHaveValue((BigInt(currentRevision) + 1n).toString());
    await expect(page.locator(".page-heading")).toContainText(newerName);
    await page.getByRole("link", { name: "Cancel", exact: true }).click();
    await page.getByRole("link", { name: "Back to orders", exact: true }).click();
    await editFromList(first);
    await expect(customer(page)).toHaveValue(retainedName);
    await expect(notebook(page)).toHaveValue("9x");
    await expect(revision(page)).toHaveValue(firstRevision);
    expect(navigations).toEqual([]);
  } finally {
    await other.close();
    for (const id of ids) await removeOrder(page, id);
  }
});

test("a missing order returns 404 and a failed client read can be retried", async ({ page }) => {
  const missing = await page.goto("/orders/999999999");
  expect(missing?.status()).toBe(404);
  await expect(page.getByRole("heading", { name: "Order not found", exact: true })).toBeVisible();
  await openList(page);
  await search(page, "Grace");
  await expect(rows(page)).toHaveCount(1);
  let failed = false;
  await page.route("**/api/**", async (route) => {
    if (!failed && apiRead(route.request()) && new URL(route.request().url()).searchParams.get("id") === "2") {
      failed = true;
      await route.abort("failed");
    } else await route.continue();
  });
  const navigations = documentRequests(page);
  await page.getByRole("link", { name: "Order #2", exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText("The order could not be loaded. Please try again.");
  expect(failed).toBe(true);
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Order #2", exact: true })).toBeVisible();
  await expect(page.locator(".page-heading")).toBeFocused();
  await expect(page.locator(".page-heading")).toContainText("Grace Hopper");
  expect(navigations).toEqual([]);
});

test("a delayed earlier search cannot replace the newer query or results", async ({ page }) => {
  await openList(page);
  const liveRegion = await listStatus(page).elementHandle();
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let fetched!: () => void;
  const olderFetched = new Promise<void>((resolve) => { fetched = resolve; });
  let completed!: () => void;
  const olderCompleted = new Promise<void>((resolve) => { completed = resolve; });
  await page.route("**/api/**", async (route) => {
    if (!apiRead(route.request()) || new URL(route.request().url()).searchParams.get("query") !== "Grace") {
      await route.continue();
      return;
    }
    const response = await route.fetch();
    fetched();
    await held;
    try { await route.fulfill({ response }); }
    catch { /* The newer query may have cancelled the older browser request. */ }
    finally { completed(); }
  });
  const navigations = documentRequests(page);
  try {
    await search(page, "Grace");
    await olderFetched;
    expect(await liveRegion!.evaluate((element) => element.isConnected)).toBe(true);
    await search(page, "Barbara");
    await expect(rows(page)).toHaveCount(1);
    await expect(rows(page)).toContainText("Barbara Liskov");
    expect(await listStatus(page).evaluate((element, original) => element === original, liveRegion!)).toBe(true);
    release();
    await olderCompleted;
    await expect(searchBox(page)).toHaveValue("Barbara");
    await expect.poll(() => new URL(page.url()).searchParams.get("q")).toBe("Barbara");
    await expect(rows(page)).toHaveCount(1);
    await expect(rows(page)).toContainText("Barbara Liskov");
    expect(navigations).toEqual([]);
  } finally {
    release();
  }
});

test("deletion keeps a conflicting revision and reconciles a lost response after commit", async ({ page, context }) => {
  await openList(page);
  const originalName = uniqueName("Delete recovery");
  const id = await createOrder(page, originalName);
  const other = await context.newPage();
  let deleted = false;
  try {
    await page.getByRole("link", { name: "Delete order", exact: true }).click();
    await ready(page);
    const originalRevision = await revision(page).inputValue();
    await other.goto(`/orders/${id}/edit`);
    await ready(other);
    const updatedName = `${originalName} changed`;
    await customer(other).fill(updatedName);
    await other.getByRole("button", { name: "Save order", exact: true }).click();
    await expect(other.locator('[role="status"]').filter({ hasText: "Order saved." })).toHaveText("Order saved.");
    const savedRevision = await revision(other).inputValue();
    for (let attempt = 0; attempt < 2; attempt++) {
      const rejected = page.waitForResponse((response) => response.request().method() === "POST"
        && new URL(response.url()).pathname.startsWith("/api/"));
      await page.getByRole("button", { name: "Delete order", exact: true }).click();
      const response = await rejected;
      expect(response.status()).toBe(200);
      expect(await response.text()).toContain("Conflict");
      await expect(problem(page)).toContainText("It was not deleted");
      await expect(problem(page)).toContainText(updatedName);
      await expect(problem(page)).toBeFocused();
      await expect(revision(page)).toHaveValue(originalRevision);
    }
    await page.getByRole("link", { name: "Reload saved order", exact: true }).click();
    await ready(page);
    await expect(revision(page)).toHaveValue(savedRevision);
    let committedStatus: number | undefined;
    let mutations = 0;
    await page.route("**/api/**", async (route) => {
      if (route.request().method() !== "POST") { await route.continue(); return; }
      mutations++;
      const response = await route.fetch();
      committedStatus = response.status();
      deleted = committedStatus === 200;
      await route.abort("failed");
    });
    await page.getByRole("button", { name: "Delete order", exact: true }).press("Enter");
    await expect(page.locator('[role="status"]').filter({ hasText: "The deletion outcome is unknown" })).toContainText("The deletion outcome is unknown");
    expect(committedStatus).toBe(200);
    expect(mutations).toBe(1);
    await expect(problem(page)).toHaveCount(0);
    const blockedDelete = page.getByRole("button", { name: "Delete order", exact: true });
    await expect(blockedDelete).toBeVisible();
    await expect(blockedDelete).toBeFocused();
    await expect(blockedDelete).toHaveAttribute("aria-disabled", "true");
    await blockedDelete.press("Enter");
    await expect(page.locator('form[action^="/orders"]')).toHaveAttribute("aria-busy", "false");
    expect(mutations).toBe(1);
    await expect(revision(page)).toHaveValue(savedRevision);
    await page.unrouteAll();
    await page.getByRole("link", { name: "Check orders list", exact: true }).click();
    await ready(page);
    await search(page, updatedName);
    await expect(page.getByText("No orders match this search.", { exact: false })).toBeVisible();
    await expect(rows(page)).toHaveCount(0);
  } finally {
    await page.unrouteAll();
    await other.close();
    if (!deleted) await removeOrder(page, id);
  }
});

test("narrow list, create, detail and delete pages support keyboard focus and accessibility", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 360, height: 900 });
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  const checkPage = async (name: string) => {
    expect((await audit(page)).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const path = testInfo.outputPath(`workflow-${name}-narrow-dark.png`);
    await page.screenshot({ path, fullPage: true });
    await testInfo.attach(`workflow-${name}-narrow-dark`, { path, contentType: "image/png" });
  };
  let id: number | undefined;
  let deleted = false;
  try {
    await openList(page);
    await expect(page.locator(".page-heading")).toBeFocused();
    await checkPage("list");
    await page.keyboard.press("Tab");
    await expect(page.getByRole("link", { name: "New order", exact: true })).toBeFocused();
    await page.keyboard.press("Enter");
    await ready(page);
    await expect(page.getByRole("heading", { name: "Create order", exact: true })).toBeFocused();
    await checkPage("create");
    await customer(page).fill(uniqueName("Keyboard lifecycle"));
    await notebook(page).fill("2");
    await notebook(page).press("Enter");
    await expect(page).toHaveURL(/\/orders\/\d+$/);
    id = Number(new URL(page.url()).pathname.split("/").at(-1));
    expect(id).toBeGreaterThan(12);
    await expect(page.locator(".page-heading")).toBeFocused();
    await checkPage("detail");
    await page.getByRole("navigation", { name: "Main navigation", exact: true }).getByRole("link", { name: "STUCCO / ORDERS", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Orders", exact: true })).toBeVisible();
    await expect(page.locator(".page-heading")).toBeFocused();
    await page.getByRole("link", { name: `Order #${id}`, exact: true }).click();
    await expect(page.getByRole("heading", { name: `Order #${id}`, exact: true })).toBeVisible();
    await expect(page.locator(".page-heading")).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("link", { name: "Edit order", exact: true })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(page.getByRole("link", { name: "Delete order", exact: true })).toBeFocused();
    await page.keyboard.press("Enter");
    await ready(page);
    await expect(page.locator(".page-heading")).toBeFocused();
    await checkPage("delete");
    await page.keyboard.press("Tab");
    await expect(page.getByRole("button", { name: "Delete order", exact: true })).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/orders$/);
    deleted = true;
  } finally {
    if (id !== undefined && !deleted) await removeOrder(page, id);
  }
});

test("reloading a pending deletion cannot refetch or replace its confirmation", async ({ page }) => {
  await openList(page);
  const id = await createOrder(page, uniqueName("Pending delete reload"));
  let deleted = false;
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let intercepted!: () => void;
  const mutationStarted = new Promise<void>((resolve) => { intercepted = resolve; });
  try {
    await page.getByRole("link", { name: "Delete order", exact: true }).click();
    await ready(page);
    const originalRevision = await revision(page).inputValue();
    const reads: string[] = [];
    const navigations = documentRequests(page);
    let mutations = 0;
    await page.route("**/api/**", async (route) => {
      if (apiRead(route.request())) {
        reads.push(route.request().url());
        await route.continue();
        return;
      }
      if (route.request().method() !== "POST") { await route.continue(); return; }
      mutations++;
      intercepted();
      await held;
      await route.continue();
    });
    await page.getByRole("button", { name: "Delete order", exact: true }).press("Enter");
    await mutationStarted;
    await expect(page.locator('form[action^="/orders"]')).toHaveAttribute("aria-busy", "true");
    await expect(page.getByRole("button", { name: "Deleting…", exact: true })).toHaveAttribute("aria-disabled", "true");
    await page.getByRole("link", { name: "Reload saved order", exact: true }).click();
    await expect(page).toHaveURL(new RegExp(`/orders/${id}/delete$`));
    await expect(page.locator('form[action^="/orders"]')).toHaveAttribute("aria-busy", "true");
    await expect(revision(page)).toHaveValue(originalRevision);
    await expect(page.getByRole("heading", { name: `Delete order #${id}?`, exact: true })).toBeVisible();
    expect(reads).toEqual([]);
    expect(navigations).toEqual([]);
    expect(mutations).toBe(1);

    const posted = page.waitForResponse((response) => response.request().method() === "POST"
      && new URL(response.url()).pathname.startsWith("/api/"));
    release();
    expect((await posted).status()).toBe(200);
    await expect(page).toHaveURL(/\/orders$/);
    await expect(listStatus(page)).toBeVisible();
    deleted = true;
    expect(navigations).toEqual([]);
    expect(mutations).toBe(1);
  } finally {
    release();
    await page.unrouteAll({ behavior: "wait" });
    if (!deleted) await removeOrder(page, id);
  }
});
