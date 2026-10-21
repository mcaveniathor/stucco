import { test, expect } from "@playwright/test";
import { audit } from "../audit";

test("audits settled colors without hiding a real contrast failure", async ({ page }) => {
  await page.setContent(`<!doctype html><html lang="en"><head><title>Audit readiness</title>
    <style>
      body { background: white; color: black; }
      #message { color: #ddd; transition: color 200ms linear; }
      #spinner { display: inline-block; animation: spin 1s linear infinite; }
      @keyframes spin { to { transform: rotate(360deg); } }
    </style></head><body><main><h1>Audit readiness</h1>
    <p id="message">This text becomes readable when the transition finishes.</p>
    <span id="spinner" aria-hidden="true">↻</span>
    </main></body></html>`);
  await page.locator("#message").evaluate(el => {
    // Establish the starting style before starting a real CSS transition.
    getComputedStyle(el).color;
    (el as HTMLElement).style.color = "black";
  });
  expect((await audit(page)).violations).toEqual([]);
  await expect(page.locator("#message")).toHaveCSS("color", "rgb(0, 0, 0)");
  expect(await page.locator("#spinner").evaluate(el =>
    el.getAnimations().some(animation => animation.playState === "running"),
  )).toBe(true);

  await page.locator("#message").evaluate(el => {
    (el as HTMLElement).style.color = "#ddd";
  });
  const result = await audit(page);
  expect(result.violations.map(violation => violation.id)).toContain("color-contrast");
});
