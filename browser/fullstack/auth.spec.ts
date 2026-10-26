import { test, expect, type Browser, type Page, type Request } from "@playwright/test";
import { audit } from "../audit";
import { accounts, expireSession, loginHttp, loginUi, type AccountName } from "./fixtures";

const origin = "http://127.0.0.1:4183";
const customer = (page: Page) => page.getByRole("textbox", { name: "Customer", exact: true });
const notebook = (page: Page) => page.getByRole("textbox", { name: "Notebook quantity", exact: true });
const revision = (page: Page) => page.locator('input[name="expected_revision"]');
const rows = (page: Page) => page.locator("table tbody tr");
const orderForm = (page: Page) => page.locator('form[action^="/orders"]');
const summary = (page: Page) => page.getByRole("alert", { name: "There is a problem", exact: true });
const mainLink = (page: Page) => page.getByRole("navigation", { name: "Main navigation", exact: true }).getByRole("link", { name: "STUCCO / ORDERS", exact: true });
const apiPost = (request: Request) => request.method() === "POST" && new URL(request.url()).pathname.startsWith("/api/");
const body = (request: Request) => new URLSearchParams(request.postData() ?? "");
const unique = (label: string) => `${label} ${test.info().project.name} ${Date.now()}`;

test.beforeEach(async ({ page }) => {
  page.on("dialog", async (dialog) => {
    if (dialog.type() === "beforeunload") await dialog.accept();
    else await dialog.dismiss();
  });
});

async function ready(page: Page, hydrated = true) {
  await expect(page.locator(`main [data-hydrated="${hydrated}"]`)).toBeVisible();
}

async function start(page: Page, account: AccountName, hydrated = true, path = "/orders") {
  await page.goto(path);
  await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
  await loginUi(page, account, hydrated);
  await ready(page, hydrated);
}

async function signOut(page: Page, hydrated = true) {
  const posted = page.waitForResponse((response) => {
    const request = response.request();
    if (!hydrated) return request.method() === "POST" && new URL(request.url()).pathname === "/logout";
    const input = body(request);
    return apiPost(request) && !input.has("customer") && !input.has("username") && !input.has("id");
  });
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  expect((await posted).status()).toBe(hydrated ? 200 : 303);
  await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
  await expect(customer(page)).toHaveCount(0);
}

async function create(page: Page, name: string, hydrated = true) {
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await ready(page, hydrated);
  await customer(page).fill(name);
  await notebook(page).fill("2");
  await page.getByRole("button", { name: "Create order", exact: true }).click();
  await expect(page).toHaveURL(/\/orders\/\d+$/);
  const id = Number(new URL(page.url()).pathname.split("/").at(-1));
  expect(id).toBeGreaterThan(0);
  await expect(page.getByRole("heading", { name: `Order #${id}`, exact: true })).toBeVisible();
  await ready(page, hydrated);
  return id;
}

async function search(page: Page, name: string) {
  await page.getByRole("textbox", { name: "Search customers", exact: true }).fill(name);
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await expect.poll(() => new URL(page.url()).searchParams.get("q")).toBe(name);
  await expect(page.locator('[role="status"]').filter({ hasText: /orders · Page/ })).toContainText("Page 1 of 1");
}

async function cleanup(browser: Browser, account: AccountName, ids: number[] = [], name?: string) {
  // Cleanup must also work when a native test exposes a broken WASM build.
  const context = await browser.newContext({ baseURL: origin, javaScriptEnabled: false });
  try {
    await loginHttp(context.request, account);
    const page = await context.newPage();
    if (name) {
      await page.goto("/orders");
      await ready(page, false);
      await search(page, name);
      for (const href of await rows(page).getByRole("link").evaluateAll((links) => links.map((link) => link.getAttribute("href")))) {
        if (href) ids.push(Number(href.split("/").at(-1)));
      }
    }
    for (const id of new Set(ids)) {
      const response = await page.goto(`/orders/${id}/delete`);
      if (response?.status() === 404) continue;
      await ready(page, false);
      await page.getByRole("button", { name: "Delete order", exact: true }).click();
      await expect(page).toHaveURL(/\/orders$/);
    }
  } finally {
    await context.close();
  }
}

async function captureAccessibleState(page: Page, name: string) {
  for (const [layout, width, colorScheme] of [
    ["wide-light", 1280, "light"], ["narrow-dark", 360, "dark"],
  ] as const) {
    await page.setViewportSize({ width, height: 900 });
    await page.emulateMedia({ colorScheme, reducedMotion: "reduce" });
    expect((await audit(page)).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const path = test.info().outputPath(`auth-${name}-${layout}.png`);
    await page.screenshot({ path, fullPage: true });
    await test.info().attach(`auth-${name}-${layout}`, { path, contentType: "image/png" });
  }
}

for (const hydrated of [true, false]) {
  test.describe(hydrated ? "hydrated authentication" : "authentication without JavaScript", () => {
    test.use({ javaScriptEnabled: hydrated });

    test("login validates credentials, resumes the requested page and logout protects it", async ({ page }) => {
      await page.goto("/orders/1/edit");
      await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
      await ready(page, hydrated);
      await expect(customer(page)).toHaveCount(0);
      await page.getByRole("textbox", { name: "Username", exact: true }).fill(accounts.alice.username);
      await page.getByLabel("Password", { exact: true }).fill("incorrect-browser-password");
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await expect(summary(page)).toContainText("The username or password is incorrect.");
      await expect(page.getByRole("textbox", { name: "Username", exact: true })).toHaveValue(accounts.alice.username);
      await expect(page.getByLabel("Password", { exact: true })).toHaveValue("");
      if (hydrated) {
        await expect(summary(page)).toBeFocused();
        await captureAccessibleState(page, "login-error");
      }
      await loginUi(page, "alice", hydrated);
      await expect(page).toHaveURL(/\/orders\/1\/edit$/);
      await expect(customer(page)).toBeVisible();
      await signOut(page, hydrated);
      await page.goto("/orders/1/edit");
      await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
      await expect(customer(page)).toHaveCount(0);
    });

    test("another tenant cannot list or open a created order", async ({ page, browser }) => {
      await start(page, "alice", hydrated);
      const name = unique("Private team A order");
      const id = await create(page, name, hydrated);
      try {
        await signOut(page, hydrated);
        await loginUi(page, "bob", hydrated);
        for (const suffix of ["", "/edit", "/delete"]) {
          const response = await page.goto(`/orders/${id}${suffix}`);
          expect(response?.status()).toBe(404);
          await expect(page.getByRole("heading", { name: "Order not found", exact: true })).toBeVisible();
          expect(await response!.text()).not.toContain(name);
          await expect(customer(page)).toHaveCount(0);
        }
        await mainLink(page).click();
        await search(page, name);
        await expect(rows(page)).toHaveCount(0);
        await expect(page.getByText("No orders match this search.", { exact: false })).toBeVisible();
        await expect(page.locator(".account-menu")).toContainText(accounts.bob.displayName);
      } finally {
        await cleanup(browser, "alice", [id]);
      }
    });

    for (const reauthenticatedAs of ["alice", "carol"] as const) {
      test(`simulated expiry of the browser session cookie ${reauthenticatedAs === "alice" ? "recovers only the original account's" : "does not transfer another principal's"} submitted draft`, async ({ page, context, browser }) => {
        await start(page, "alice", hydrated);
        const savedName = unique("Recovery owner");
        const id = await create(page, savedName, hydrated);
        try {
          await page.getByRole("link", { name: "Edit order", exact: true }).click();
          await ready(page, hydrated);
          const originalRevision = await revision(page).inputValue();
          const rawName = `  ${unique("Raw <draft> & text")}  `;
          await customer(page).fill(rawName);
          await notebook(page).fill("7x");
          await expireSession(context);
          const rejected = hydrated ? undefined : page.waitForResponse((response) => response.request().method() === "POST"
            && new URL(response.url()).pathname === `/orders/${id}/edit`);
          await page.getByRole("button", { name: "Save order", exact: true }).click();
          if (rejected) {
            expect((await rejected).status()).toBe(401);
            await expect(page.getByRole("heading", { name: "Sign in to recover your draft", exact: true })).toBeVisible();
            await expect(customer(page)).toHaveCount(0);
            await page.getByRole("link", { name: "Sign in", exact: true }).click();
          } else {
            await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
            await expect(page.getByText("Your session ended. Sign in to the same account to recover your unsaved draft.", { exact: true })).toBeVisible();
          }
          await expect(customer(page)).toHaveCount(0);
          if (hydrated && reauthenticatedAs === "alice") {
            await captureAccessibleState(page, "session-recovery");
          }

          // Carol shares the tenant, so this independently checks that the
          // unauthorized submission did not modify the saved order.
          const observer = await browser.newContext({ baseURL: origin });
          try {
            await loginHttp(observer.request, "carol");
            const saved = await observer.request.get(`/orders/${id}`);
            expect(saved.status()).toBe(200);
            const html = await saved.text();
            expect(html).toContain(savedName);
            expect(html).not.toContain(rawName);
            expect(html).toContain(`Saved revision ${originalRevision}`);
          } finally { await observer.close(); }

          await loginUi(page, reauthenticatedAs, hydrated);
          await expect(page).toHaveURL(new RegExp(`/orders/${id}/edit$`));
          await ready(page, hydrated);
          await expect(revision(page)).toHaveValue(originalRevision);
          if (reauthenticatedAs === "alice") {
            await expect(customer(page)).toHaveValue(rawName);
            await expect(notebook(page)).toHaveValue("7x");
            if (hydrated) await captureAccessibleState(page, "recovered-draft");
            await notebook(page).fill("7");
            await page.getByRole("button", { name: "Save order", exact: true }).click();
            if (hydrated) {
              await expect(page.locator('[role="status"]').filter({ hasText: "Order saved." })).toBeVisible();
              await expect(revision(page)).toHaveValue((BigInt(originalRevision) + 1n).toString());
            } else {
              await expect(page).toHaveURL(new RegExp(`/orders/${id}$`));
              await expect(page.getByText(`Saved revision ${BigInt(originalRevision) + 1n}`, { exact: true })).toBeVisible();
            }
          } else {
            await expect(customer(page)).toHaveValue(savedName);
            await expect(notebook(page)).toHaveValue("2");
            await expect(summary(page)).toHaveCount(0);
          }
        } finally { await cleanup(browser, "alice", [id]); }
      });
    }
  });
}

for (const replacement of ["bob", "carol"] as const) {
  test(`logout clears existing and new-order drafts before ${replacement} signs in`, async ({ page, browser }) => {
    await start(page, "alice");
    const savedName = unique("Saved before logout");
    const id = await create(page, savedName);
    try {
      await page.getByRole("link", { name: "Edit order", exact: true }).click();
      await ready(page);
      const editDraft = unique("Private edited draft");
      await customer(page).fill(editDraft);
      await notebook(page).fill("8x");
      await page.getByRole("link", { name: "Back to orders", exact: true }).click();
      await page.getByRole("link", { name: "New order", exact: true }).click();
      await ready(page);
      const newDraft = unique("Private new draft");
      await customer(page).fill(newDraft);
      await notebook(page).fill("9x");
      await signOut(page);
      await loginUi(page, replacement);
      await expect(page).toHaveURL(/\/orders\/new$/);
      await expect(customer(page)).toHaveValue("");
      await expect(notebook(page)).toHaveValue("1");
      await expect(page.getByRole("button", { name: "Create order", exact: true })).not.toHaveAttribute("aria-disabled", "true");
      await expect(summary(page)).toHaveCount(0);
      await mainLink(page).click();
      if (replacement === "carol") {
        await search(page, savedName);
        await page.getByRole("link", { name: `Order #${id}`, exact: true }).click();
        await page.getByRole("link", { name: "Edit order", exact: true }).click();
        await expect(customer(page)).toHaveValue(savedName);
        await expect(notebook(page)).toHaveValue("2");
      } else {
        await search(page, savedName);
        await expect(rows(page)).toHaveCount(0);
      }
      const values = await page.locator("input").evaluateAll((inputs) => inputs.map((input) => input.value));
      expect(values).not.toContain(editDraft);
      expect(values).not.toContain(newDraft);
    } finally { await cleanup(browser, "alice", [id]); }
  });
}

test("a creation response committed before logout cannot replace the next account's draft", async ({ page, browser }) => {
  await start(page, "alice");
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await ready(page);
  const committedName = unique("Alice late creation");
  await customer(page).fill(committedName);
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let fetched!: () => void;
  const committed = new Promise<void>((resolve) => { fetched = resolve; });
  let finished!: () => void;
  const delivered = new Promise<void>((resolve) => { finished = resolve; });
  let mutations = 0;
  await page.route("**/api/**", async (route) => {
    if (!apiPost(route.request()) || body(route.request()).get("customer") !== committedName) {
      await route.continue(); return;
    }
    mutations++;
    const response = await route.fetch();
    expect(response.status()).toBe(200);
    fetched();
    await held;
    try { await route.fulfill({ response }); }
    catch { /* Logout may cancel the old browser request. */ }
    finally { finished(); }
  });
  try {
    await page.getByRole("button", { name: "Create order", exact: true }).click();
    await committed;
    await expect(orderForm(page)).toHaveAttribute("aria-busy", "true");
    await signOut(page);
    await loginUi(page, "bob");
    await expect(customer(page)).toHaveValue("");
    const bobsDraft = unique("Bob keeps this draft");
    await customer(page).fill(bobsDraft);
    await notebook(page).fill("8x");
    release();
    await delivered;
    await expect(page).toHaveURL(/\/orders\/new$/);
    await expect(customer(page)).toHaveValue(bobsDraft);
    await expect(notebook(page)).toHaveValue("8x");
    await expect(page.locator(".account-menu")).toContainText(accounts.bob.displayName);
    await expect(page.getByRole("button", { name: "Create order", exact: true })).not.toHaveAttribute("aria-disabled", "true");
    await expect(page.locator('[role="status"]').filter({ hasText: /created|Order saved/ })).toHaveCount(0);
    expect(mutations).toBe(1);
    await mainLink(page).click();
    await search(page, committedName);
    await expect(rows(page)).toHaveCount(0);
    await page.reload();
    await expect(page.locator(".account-menu")).toContainText(accounts.bob.displayName);
    await expect(rows(page)).toHaveCount(0);
  } finally {
    release();
    await page.unrouteAll({ behavior: "wait" });
    await cleanup(browser, "alice", [], committedName);
  }
});

test("a delayed authenticated read cannot populate another tenant's list", async ({ page, browser }) => {
  await start(page, "alice");
  const name = unique("Alice delayed read");
  const id = await create(page, name);
  await mainLink(page).click();
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  let fetched!: () => void;
  const snapshotReady = new Promise<void>((resolve) => { fetched = resolve; });
  let finished!: () => void;
  const delivered = new Promise<void>((resolve) => { finished = resolve; });
  let intercepted = false;
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    if (intercepted || request.method() !== "GET" || new URL(request.url()).searchParams.get("query") !== name) {
      await route.continue(); return;
    }
    intercepted = true;
    const response = await route.fetch();
    expect(await response.text()).toContain(name);
    fetched();
    await held;
    try { await route.fulfill({ response }); }
    catch { /* The old tenant's resource may be cancelled during logout. */ }
    finally { finished(); }
  });
  try {
    await page.getByRole("textbox", { name: "Search customers", exact: true }).fill(name);
    await page.getByRole("button", { name: "Search", exact: true }).click();
    await snapshotReady;
    await signOut(page);
    await loginUi(page, "bob");
    await expect(rows(page)).toHaveCount(0);
    release();
    await delivered;
    await expect(page.locator(".account-menu")).toContainText(accounts.bob.displayName);
    await expect(rows(page)).toHaveCount(0);
    await expect(page.getByText(name, { exact: true })).toHaveCount(0);
    await expect(page.getByRole("textbox", { name: "Search customers", exact: true })).toHaveValue(name);
  } finally {
    release();
    await page.unrouteAll({ behavior: "wait" });
    await cleanup(browser, "alice", [id]);
  }
});

for (const operation of ["edit", "create", "delete"] as const) {
  test(`a ${operation} form uses the fresh token after the same account signs in in another tab`, async ({ page, context, browser }) => {
    await start(page, "alice");
    const name = unique(`Rotated token ${operation}`);
    let id: number | undefined;
    let deleted = false;
    const other = await context.newPage();
    try {
      if (operation === "create") {
        await page.getByRole("link", { name: "New order", exact: true }).click();
      } else {
        id = await create(page, name);
        await page.getByRole("link", { name: operation === "edit" ? "Edit order" : "Delete order", exact: true }).click();
      }
      await ready(page);
      const originalRevision = await revision(page).inputValue();
      const oldCsrf = await orderForm(page).locator('input[name="_csrf"]').inputValue();
      if (operation !== "delete") {
        await customer(page).fill(name);
        await notebook(page).fill("4");
      }
      const navigations: string[] = [];
      page.on("request", (request) => {
        if (request.isNavigationRequest() && request.frame() === page.mainFrame()) navigations.push(request.url());
      });

      // Both tabs share cookies. Native login rotates the session and CSRF
      // token without changing the principal that owns the first tab's draft.
      await other.goto("/orders");
      await other.bringToFront();
      await loginHttp(other.request, "alice");
      await page.bringToFront();
      await page.evaluate(() => window.dispatchEvent(new Event("focus")));
      await expect(orderForm(page).locator('input[name="_csrf"]')).not.toHaveValue(oldCsrf);
      await expect(revision(page)).toHaveValue(originalRevision);
      await expect(page.locator(".account-menu")).toContainText(accounts.alice.displayName);
      if (operation !== "delete") {
        await expect(customer(page)).toHaveValue(name);
        await expect(notebook(page)).toHaveValue("4");
      }
      const label = operation === "edit" ? "Save order" : operation === "create" ? "Create order" : "Delete order";
      await expect(page.getByRole("button", { name: label, exact: true })).not.toHaveAttribute("aria-disabled", "true");
      await page.getByRole("button", { name: label, exact: true }).click();

      if (operation === "edit") {
        await expect(page.locator('[role="status"]').filter({ hasText: "Order saved." })).toBeVisible();
        await expect(revision(page)).toHaveValue((BigInt(originalRevision) + 1n).toString());
        await expect(page.getByRole("button", { name: "Save order", exact: true })).not.toHaveAttribute("aria-disabled", "true");
      } else if (operation === "create") {
        await expect(page).toHaveURL(/\/orders\/\d+$/);
        id = Number(new URL(page.url()).pathname.split("/").at(-1));
        await expect(page.getByRole("heading", { name: `Order #${id}`, exact: true })).toBeVisible();
        await expect(page.locator(".page-heading")).toContainText(name);
      } else {
        await expect(page).toHaveURL(/\/orders$/);
        deleted = true;
        await search(page, name);
        await expect(rows(page)).toHaveCount(0);
      }
      await expect(page.locator('[role="status"]').filter({ hasText: /outcome is unknown/ })).toHaveCount(0);
      expect(navigations).toEqual([]);
    } finally {
      await other.close();
      if (!deleted) await cleanup(browser, "alice", id === undefined ? [] : [id], id === undefined ? name : undefined);
    }
  });
}

test("a verified CSRF rejection preserves the create draft and requires an explicit retry", async ({ page, browser }) => {
  await start(page, "alice");
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await ready(page);
  const name = unique("Verified CSRF retry");
  const rawName = `  ${name}  `;
  await customer(page).fill(rawName);
  await notebook(page).fill("4");
  const originalRevision = await revision(page).inputValue();
  let mutations = 0;
  const isCreate = (request: Request) => apiPost(request)
    && body(request).has("quantity_one") && !body(request).has("id");
  await page.route("**/api/**", async (route) => {
    if (!isCreate(route.request())) { await route.continue(); return; }
    mutations++;
    if (mutations === 1) {
      // The real server must identify the rejection; this is not a mocked 403.
      const submitted = body(route.request());
      submitted.set("_csrf", "invalid-browser-test-token");
      await route.continue({ postData: submitted.toString() });
    } else await route.continue();
  });
  try {
    const rejected = page.waitForResponse((response) => isCreate(response.request()));
    await page.getByRole("button", { name: "Create order", exact: true }).press("Enter");
    const response = await rejected;
    expect(response.status()).toBe(403);
    expect(await response.text()).toBe("MiddlewareError|stucco.csrf.rejected");
    await expect(page.locator('[role="status"]').filter({
      hasText: "Form protection rejected the request before saving.",
    })).toBeVisible();
    await expect(customer(page)).toHaveValue(rawName);
    await expect(notebook(page)).toHaveValue("4");
    await expect(revision(page)).toHaveValue(originalRevision);
    await expect(customer(page)).toHaveJSProperty("readOnly", false);
    await expect(orderForm(page)).toHaveAttribute("aria-busy", "false");
    const createButton = page.getByRole("button", { name: "Create order", exact: true });
    await expect(createButton).toBeEnabled();
    await expect(createButton).not.toHaveAttribute("aria-disabled", "true");
    await expect(page.locator('[role="status"]').filter({ hasText: /outcome is unknown/ })).toHaveCount(0);
    expect(mutations).toBe(1);

    await createButton.click();
    await expect(page).toHaveURL(/\/orders\/\d+$/);
    await expect(page.locator(".page-heading")).toContainText(name);
    await expect(page.getByText("Saved revision 1", { exact: true })).toBeVisible();
    expect(mutations).toBe(2);
    await mainLink(page).click();
    await search(page, name);
    await expect(rows(page)).toHaveCount(1);
    await expect(rows(page)).toContainText(name);
  } finally {
    await page.unrouteAll({ behavior: "wait" });
    await cleanup(browser, "alice", [], name);
  }
});

test("an opaque proxy 403 keeps the create outcome unknown and blocks retries", async ({ page }) => {
  await start(page, "alice");
  await page.getByRole("link", { name: "New order", exact: true }).click();
  await ready(page);
  const rawName = `  ${unique("Opaque proxy rejection")}  `;
  await customer(page).fill(rawName);
  await notebook(page).fill("4");
  const originalRevision = await revision(page).inputValue();
  let mutations = 0;
  await page.route("**/api/**", async (route) => {
    if (!apiPost(route.request()) || !body(route.request()).has("quantity_one") || body(route.request()).has("id")) {
      await route.continue(); return;
    }
    mutations++;
    await route.fulfill({ status: 403, contentType: "text/plain", body: "Forbidden by upstream proxy" });
  });
  try {
    await page.getByRole("button", { name: "Create order", exact: true }).press("Enter");
    await expect(page.locator('[role="status"]').filter({ hasText: "The creation outcome is unknown" })).toBeVisible();
    await expect(customer(page)).toHaveValue(rawName);
    await expect(notebook(page)).toHaveValue("4");
    await expect(revision(page)).toHaveValue(originalRevision);
    await expect(customer(page)).toHaveJSProperty("readOnly", true);
    await expect(orderForm(page)).toHaveAttribute("aria-busy", "false");
    const blockedCreate = page.getByRole("button", { name: "Create order", exact: true });
    await expect(blockedCreate).toBeVisible();
    await expect(blockedCreate).toBeFocused();
    await expect(blockedCreate).toHaveAttribute("aria-disabled", "true");
    await expect(page.locator('[role="status"]').filter({
      hasText: "Form protection rejected the request before saving.",
    })).toHaveCount(0);
    expect(mutations).toBe(1);
    await blockedCreate.press("Enter");
    await expect(orderForm(page)).toHaveAttribute("aria-busy", "false");
    await expect(page.locator('[role="status"]').filter({ hasText: "The creation outcome is unknown" })).toBeVisible();
    expect(mutations).toBe(1);
  } finally {
    await page.unrouteAll({ behavior: "wait" });
  }
});
