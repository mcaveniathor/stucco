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

test("audits inherited light-dark colors after remounting content and switching schemes", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await page.setContent(`<!doctype html><html lang="en"><head><title>Theme audit readiness</title>
    <style>
      :root {
        color-scheme: light dark;
        --text: light-dark(#231f20, #f4eced);
        --surface: light-dark(#f9f6f6, #231f20);
        --muted: light-dark(#5e5253, #cbc0c1);
      }
      body { color: var(--text); background: var(--surface); }
      .card { background: var(--surface); padding: 24px; }
      .item > span { display: block; color: var(--muted); }
      .bad { color: light-dark(#ddd, #333); }
    </style></head><body><main><h1>Theme audit readiness</h1>
    <section class="card" aria-label="Items"></section></main></body></html>`);
  for (const colorScheme of ["dark", "light", "dark", "light"] as const) {
    await page.emulateMedia({ colorScheme });
    if (colorScheme === "dark") {
      // Session recovery mounts fresh descendants while the previous audit's
      // dark scheme is active, then the next audit switches back to light.
      await page.locator(".card").evaluate(card => {
        card.innerHTML = `<h2>Order details</h2><div><p class="item">Notebook<span>$12.00 each</span></p></div>`;
      });
    }
    expect((await audit(page)).violations).toEqual([]);
    const inherited = await page.locator(".item").evaluate(element => {
      const body = getComputedStyle(document.body);
      const item = getComputedStyle(element);
      return {
        body: [body.color, body.getPropertyValue("-webkit-text-fill-color")],
        item: [item.color, item.getPropertyValue("-webkit-text-fill-color")],
      };
    });
    expect(inherited.item).toEqual(inherited.body);
  }
  // Stable but incorrect colors must still fail in the single Axe run.
  await page.locator(".item").evaluate(element => element.classList.add("bad"));
  const result = await audit(page);
  expect(result.violations.some(violation => violation.id === "color-contrast"
    && violation.nodes.some(node => node.html.includes('class="item bad"')))).toBe(true);
});
