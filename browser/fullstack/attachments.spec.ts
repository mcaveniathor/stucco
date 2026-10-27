import { readFile } from "node:fs/promises";
import type { Page } from "@playwright/test";
import { test, expect, expireSession, loginHttp, loginUi } from "./fixtures";
import { audit } from "../audit";

const picker = (page: Page) => page.getByLabel(/^Choose an attachment/);
const panel = (page: Page) => page.getByRole("region", { name: "Attachments", exact: true });
const upload = (page: Page) => page.getByRole("button", { name: "Upload attachment", exact: true });
const attachmentPost = (url: string) => /\/orders\/\d+\/attachments$/.test(new URL(url).pathname);

async function createOrder(page: Page, hydrated = true) {
  await page.goto("/orders/new");
  await expect(page.locator(`main [data-hydrated="${hydrated}"]`)).toBeVisible();
  await page.getByRole("textbox", { name: "Customer", exact: true }).fill(`Attachment test ${test.info().project.name} ${Date.now()}`);
  await page.getByRole("button", { name: "Create order", exact: true }).click();
  await expect(page).toHaveURL(/\/orders\/\d+$/);
  await expect(panel(page)).toBeVisible();
  if (hydrated) await expect(page.locator('main [data-hydrated="true"]')).toBeVisible();
  return new URL(page.url()).pathname;
}
async function removeOrder(page: Page, path: string) {
  // Preserve the original failure when timeout teardown has already closed it.
  if (page.isClosed()) return;
  await page.goto(`${path}/delete`);
  await page.getByRole("button", { name: "Delete order", exact: true }).click();
  await expect(page).toHaveURL(/\/orders$/);
}
async function selectFile(page: Page, name = "receipt.txt", content = "Paid in full") {
  await picker(page).setInputFiles({ name, mimeType: "text/plain", buffer: Buffer.from(content) });
}

test("Rust attachment uploads, downloads and removal retain the page and work in narrow dark layouts", async ({ page }, info) => {
  const path = await createOrder(page);
  try {
    const navigations: string[] = [];
    page.on("request", request => { if (request.isNavigationRequest()) navigations.push(request.url()); });
    await selectFile(page, "résumé.txt", "Attachment bytes ✓");
    await upload(page).click();
    await expect(panel(page).getByRole("status")).toHaveText("Attachment uploaded.");
    expect(navigations).toEqual([]);
    await expect(picker(page)).toHaveValue("");
    const downloaded = page.waitForEvent("download");
    await panel(page).getByRole("link", { name: "résumé.txt", exact: true }).click();
    const download = await downloaded;
    expect(download.suggestedFilename()).toBe("résumé.txt");
    expect(await readFile((await download.path())!, "utf8")).toBe("Attachment bytes ✓");
    await page.setViewportSize({ width: 390, height: 844 });
    await page.emulateMedia({ colorScheme: "dark" });
    await expect(panel(page)).toBeVisible();
    expect((await audit(page)).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    await page.screenshot({ path: info.outputPath("attachments-narrow-dark.png"), fullPage: true });
    await panel(page).getByRole("button", { name: "Remove résumé.txt", exact: true }).click();
    await expect(panel(page).getByRole("status")).toHaveText("Attachment removed.");
    await expect(panel(page).getByRole("link", { name: "résumé.txt", exact: true })).toHaveCount(0);
    await expect(panel(page).getByRole("button", { name: "Refresh attachments", exact: true })).toBeFocused();
  } finally { await removeOrder(page, path); }
});

test("a file selected before hydration remains selected and uploads through Rust", async ({ page }) => {
  const path = await createOrder(page);
  let release!: () => void;
  const gate = new Promise<void>(resolve => { release = resolve; });
  await page.route("**/pkg/**", async route => { await gate; await route.continue().catch(() => {}); });
  try {
    await page.goto(path, { waitUntil: "domcontentloaded" });
    await selectFile(page, "before-wasm.txt", "Selected before hydration");
    await picker(page).focus();
    release();
    await expect(page.locator('main [data-hydrated="true"]')).toBeVisible();
    await expect(picker(page)).toBeFocused();
    await expect(picker(page)).toHaveValue(/before-wasm\.txt$/);
    await upload(page).click();
    await expect(panel(page).getByRole("status")).toHaveText("Attachment uploaded.");
    await expect(panel(page).getByRole("link", { name: "before-wasm.txt", exact: true })).toBeVisible();
  } finally {
    release();
    await page.unroute("**/pkg/**");
    await removeOrder(page, path);
  }
});

test("a lost upload response requires reconciliation and never causes an automatic duplicate", async ({ page, context, browserName }) => {
  const path = await createOrder(page);
  let uploads = 0;
  await page.route(`**${path}/attachments`, async route => {
    uploads += 1;
    let postData: Buffer | undefined;
    if (browserName === "webkit") {
      // WebKit's interception protocol omits file bytes from postData. Restore
      // this known fixture payload when replaying the request to the real server;
      // retain the browser's multipart fields, CSRF token and filename verbatim.
      const original = route.request().postDataBuffer()!;
      const marker = Buffer.from("Content-Type: text/plain\r\n\r\n");
      const markerOffset = original.lastIndexOf(marker);
      expect(markerOffset).toBeGreaterThanOrEqual(0);
      const start = markerOffset + marker.length;
      const boundary = route.request().headers()["content-type"].split("boundary=")[1];
      const end = original.indexOf(Buffer.from(`\r\n--${boundary}`), start);
      expect(end).toBeGreaterThanOrEqual(start);
      if (end === start) {
        postData = Buffer.concat([original.subarray(0, start), Buffer.from("Exactly one copy"), original.subarray(end)]);
      }
    }
    const response = await route.fetch(postData ? { postData } : undefined);
    expect(response.status()).toBe(201);
    await route.abort("failed");
  });
  try {
    await selectFile(page, "confirmed-on-server.txt", "Exactly one copy");
    await upload(page).click();
    await expect(panel(page).getByRole("alert")).toContainText("The upload outcome is unknown");
    await expect(upload(page)).toHaveAttribute("aria-disabled", "true");
    await expect(picker(page)).toBeDisabled();
    expect(uploads).toBe(1);
    await page.unroute(`**${path}/attachments`);
    await panel(page).getByRole("button", { name: "Refresh attachments", exact: true }).click();
    await expect(panel(page).getByRole("link", { name: "confirmed-on-server.txt", exact: true })).toHaveCount(1);
    const saved = await context.request.get((await panel(page).getByRole("link", { name: "confirmed-on-server.txt", exact: true }).getAttribute("href"))!);
    expect(saved.status()).toBe(200);
    expect(await saved.text()).toBe("Exactly one copy");
    await expect(panel(page)).toContainText("1 of 3 attachment slots used.");
    await expect(upload(page)).toHaveAttribute("aria-disabled", "false");
    expect(uploads).toBe(1);
  } finally {
    await page.unroute(`**${path}/attachments`);
    await removeOrder(page, path);
  }
});

test("invalid file sizes are explained without a request and the picker stays usable", async ({ page }) => {
  const path = await createOrder(page);
  try {
    let posts = 0;
    page.on("request", request => { if (request.method() === "POST" && attachmentPost(request.url())) posts += 1; });
    await picker(page).setInputFiles({ name: "oversized.bin", mimeType: "application/octet-stream", buffer: Buffer.alloc(1024 * 1024 + 1) });
    await upload(page).click();
    await expect(panel(page).getByRole("alert")).toContainText("at most 1 MiB");
    await expect(panel(page).getByRole("alert")).toBeFocused();
    expect(posts).toBe(0);
    await expect(picker(page)).toBeEnabled();
    await selectFile(page, "small.txt");
    await upload(page).click();
    await expect(panel(page).getByRole("status")).toHaveText("Attachment uploaded.");
    expect(posts).toBe(1);
  } finally { await removeOrder(page, path); }
});

test("expired upload sessions require sign in and file reselection without leaking a pending file", async ({ page, context }) => {
  const path = await createOrder(page);
  try {
    await selectFile(page, "expired.txt", "Do not retain across sign in");
    await expireSession(context);
    await upload(page).click();
    await expect(page.getByRole("heading", { name: "Sign in", exact: true })).toBeVisible();
    await expect(picker(page)).toHaveCount(0);
    await loginUi(page, "alice");
    await expect(panel(page)).toBeVisible();
    await expect(picker(page)).toHaveValue("");
    await expect(panel(page)).toContainText("0 of 3 attachment slots used.");
  } finally { await removeOrder(page, path); }
});

test("an open attachment form uses a refreshed same-account CSRF token", async ({ page, context }) => {
  const path = await createOrder(page);
  try {
    const token = page.locator('form[enctype="multipart/form-data"] input[name="_csrf"]');
    const before = await token.inputValue();
    await selectFile(page, "rotated.txt");
    await loginHttp(context.request, "alice");
    await page.evaluate(() => window.dispatchEvent(new Event("focus")));
    await expect(token).not.toHaveValue(before);
    await expect(picker(page)).toHaveValue(/rotated\.txt$/);
    await upload(page).click();
    await expect(panel(page).getByRole("status")).toHaveText("Attachment uploaded.");
  } finally { await removeOrder(page, path); }
});

test.describe("native attachment forms", () => {
  test.use({ javaScriptEnabled: false });
  test("multipart upload, authorized download and removal work without JavaScript", async ({ page }) => {
    const path = await createOrder(page, false);
    try {
      await selectFile(page, "native.txt", "Native multipart data");
      const posted = page.waitForResponse(response => response.request().method() === "POST" && attachmentPost(response.url()));
      await upload(page).click();
      expect((await posted).status()).toBe(303);
      await expect(panel(page).getByRole("link", { name: "native.txt", exact: true })).toBeVisible();
      await expect(panel(page).getByRole("link", { name: "Reload saved attachments", exact: true })).toBeVisible();
      await expect(panel(page).getByRole("button", { name: "Refresh attachments", exact: true })).toHaveCount(0);
      const download = page.waitForEvent("download");
      await panel(page).getByRole("link", { name: "native.txt", exact: true }).click();
      expect(await readFile((await (await download).path())!, "utf8")).toBe("Native multipart data");
      await panel(page).getByRole("button", { name: "Remove native.txt", exact: true }).click();
      await expect(panel(page)).toContainText("0 of 3 attachment slots used.");
      // Axe's browser-side scheduling cannot run when JavaScript is disabled.
      // The enhanced case audits these same attachment controls and labels.
    } finally { await removeOrder(page, path); }
  });
});
