import { test, expect } from "./fixtures";

test("CSP permits Rust hydration and mutations while blocking untrusted inline scripts", async ({ page }) => {
  await page.addInitScript(() => {
    (window as any).__policyViolations = [];
    document.addEventListener("securitypolicyviolation", (event) => {
      (window as any).__policyViolations.push({
        directive: event.effectiveDirective, blocked: event.blockedURI,
      });
    });
  });
  const response = await page.goto("/orders/1/edit");
  expect(response!.headers()["cache-control"]).toBe("private, no-store");
  const policy = response!.headers()["content-security-policy"];
  expect(policy).toContain("'nonce-");
  expect(policy).toContain("'wasm-unsafe-eval'");
  expect(policy).not.toContain("'unsafe-inline'");
  expect(policy).not.toContain("'unsafe-eval'");
  await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
  const quantity = page.getByRole("textbox", { name: "Notebook quantity", exact: true });
  await quantity.fill("3");
  await page.getByRole("button", { name: "Save order", exact: true }).click();
  await expect(page.getByRole("status").filter({ hasText: "Order saved." })).toBeVisible();
  expect(await page.evaluate(() => (window as any).__policyViolations)).toEqual([]);
  await page.reload();
  await expect(page.locator('[data-hydrated="true"]')).toBeVisible();
  await expect(quantity).toHaveValue("3");
  expect(await page.evaluate(() => (window as any).__policyViolations)).toEqual([]);

  // Playwright's evaluation is privileged; adding a script element exercises the
  // document's real CSP, rather than treating evaluation as evidence of enforcement.
  await page.evaluate(() => {
    const script = document.createElement("script");
    script.textContent = "window.__untrustedInlineRan = true";
    document.head.append(script);
  });
  await expect.poll(() => page.evaluate(() => (window as any).__policyViolations.length)).toBeGreaterThan(0);
  expect(await page.evaluate(() => (window as any).__untrustedInlineRan)).toBeUndefined();
  const violations = await page.evaluate(() => (window as any).__policyViolations);
  expect(violations.every((entry: { directive: string; blocked: string }) =>
    entry.directive.startsWith("script-src") && entry.blocked === "inline")).toBe(true);
});
