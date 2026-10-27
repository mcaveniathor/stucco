import { test as base, expect, type APIRequestContext, type BrowserContext, type Page } from "@playwright/test";
import { accounts, type AccountName } from "./accounts";

export { accounts, type AccountName } from "./accounts";
export { expect } from "@playwright/test";

export function csrfFromHtml(html: string) {
  const input = html.match(/<input\b[^>]*>/g)?.find((tag) => /\bname="_csrf"/.test(tag));
  const token = input?.match(/\bvalue="([^"]+)"/)?.[1];
  if (!token) throw new Error("The login page did not include its native CSRF field.");
  return token;
}

export async function loginHttp(request: APIRequestContext, account: AccountName = "alice", baseURL = "http://127.0.0.1:4183") {
  const login = await request.get(new URL("/login", baseURL).href);
  expect(login.status()).toBe(200);
  const csrf = csrfFromHtml(await login.text());
  const credentials = accounts[account];
  const response = await request.post(new URL("/login", baseURL).href, {
    form: { username: credentials.username, password: credentials.password, _csrf: csrf, return_to: "/orders" },
    maxRedirects: 0,
  });
  expect(response.status()).toBe(303);
}

export async function loginUi(page: Page, account: AccountName, hydrated = true) {
  const credentials = accounts[account];
  if (hydrated) {
    await expect(page.locator('main [data-hydrated="true"]')).toBeVisible();
  }
  await page.getByRole("textbox", { name: "Username", exact: true }).fill(credentials.username);
  await page.getByLabel("Password", { exact: true }).fill(credentials.password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Username", exact: true })).toHaveCount(0);
  await expect(page.locator(".account-menu")).toContainText(credentials.displayName);
  await expect(page.getByRole("button", { name: "Sign out", exact: true })).toBeVisible();
}

export async function expireSession(context: BrowserContext) {
  // Simulate an expired browser session cookie while retaining CSRF protection.
  // Actual server expiry/revocation is covered by the native HTTP tests. The
  // running redb server exclusively owns its file, so no CLI writes occur here.
  await context.clearCookies({ name: "orders-session" });
}

type AuthFixtures = { account: AccountName; authenticated: void };
export const test = base.extend<AuthFixtures>({
  account: ["alice", { option: true }],
  authenticated: [async ({ context, account, baseURL }, use) => {
    await loginHttp(context.request, account, baseURL);
    await use();
  }, { auto: true }],
});
