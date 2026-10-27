import { directions } from "../directions";
import { test, expect, type Page } from "@playwright/test";
import { audit } from "../audit";
const SITE = "http://localhost:4182/";
const ready = async (page: Page) => expect(page.locator("#pg-corners option").first()).toHaveText(/^Base \(.+\)$/);

test("landing direction links enhance in place and preserve a complete destination", async ({ page }) => {
  await page.goto(SITE);
  const preview = page.locator("#landing-preview");
  await expect(page.locator("[data-preview-direction]")).toHaveCount(directions.length);
  for (const name of [...directions.slice(1), directions[0]]) {
    const link = page.locator(`[data-preview-direction="${name}"]`);
    await expect(link).toHaveAttribute("href", new RegExp(`showcase-${name}-orders.html$`));
    await link.click();
    await expect(preview).toHaveAttribute("data-st-theme", name);
    await expect(link).toHaveAttribute("aria-current", "true");
  }
});

for (const direction of ["grove", "relay", "marquee"]) {
  test(`${direction} guided exports survive selection and URL reload`, async ({ page }) => {
    await page.goto(`${SITE}playground.html`);
    await ready(page);
    await expect(page.locator("#pg-direction option")).toHaveCount(directions.length);
    await page.locator("#pg-direction").selectOption(direction);
    await page.locator("#pg-seed").fill("42");
    const variant = direction[0].toUpperCase() + direction.slice(1);
    await expect(page.locator("#pg-rust")).toContainText(`Direction::${variant}.guided_v1(42)`);
    const css = await page.locator("#pg-css").textContent();
    const tokens = JSON.parse(await page.locator("#pg-json").innerText());
    expect(tokens.recipe.query).toBe(`guided=${direction}:1:42`);
    await page.goto(`${SITE}playground.html?${tokens.recipe.query}`);
    await ready(page);
    await expect(page.locator("#pg-direction")).toHaveValue(direction);
    await expect(page.locator("#pg-css")).toHaveText(css!);
    expect(JSON.parse(await page.locator("#pg-json").innerText())).toEqual(tokens);
  });
}

test("guided locks, favorites, comparisons and JSON reproduce the displayed theme", async ({ page }) => {
  await page.goto(`${SITE}playground.html?guided=atelier:1:42`);
  await ready(page);
  const before = JSON.parse(await page.locator("#pg-json").innerText());
  await page.getByLabel("Lock palette", { exact: true }).check();
  await page.getByLabel("Lock typography", { exact: true }).check();
  await page.getByRole("button", { name: "Random seed", exact: true }).click();
  await expect(page.locator("#pg-status")).toContainText("Kept palette, typography");
  const locked = JSON.parse(await page.locator("#pg-json").innerText());
  expect(locked.light).toEqual(before.light);
  expect(locked.dark).toEqual(before.dark);
  expect(locked.recipe.query).toContain("palette=stone");
  expect(locked.recipe.query).toContain("heading-font=serif");
  const css = await page.locator("#pg-css").textContent();
  await page.getByRole("button", { name: "Save favorite", exact: true }).click();
  await expect(page.locator("#pg-status")).toContainText("Favorite saved");
  await page.locator("#pg-screen").selectOption("project");
  await page.locator("#pg-scheme").selectOption("dark");
  await page.locator("#pg-narrow").check();
  await expect(page.locator('[data-pg-screen="project"]:visible')).toHaveCount(2);
  await expect(page.locator('[data-pg-scheme="light"]:visible')).toHaveCount(0);
  await page.locator("#pg-direction").selectOption("play");
  await expect(page.locator("#pg-rust")).toContainText("Direction::Play");
  await page.getByRole("button", { name: "Load favorite", exact: true }).click();
  await expect(page.locator("#pg-css")).toHaveText(css!);
  await page.goto(`${SITE}playground.html?${locked.recipe.query}`);
  await ready(page);
  await expect(page.locator("#pg-css")).toHaveText(css!);
  await expect(page.locator("#pg-favorite option")).toHaveCount(2);
});

test("unavailable storage keeps discovery functional and explains favorite lifetime", async ({ page }) => {
  await page.addInitScript(() => {
    Storage.prototype.setItem = () => { throw new Error("unavailable"); };
    Storage.prototype.getItem = () => { throw new Error("unavailable"); };
  });
  await page.goto(`${SITE}playground.html`);
  await ready(page);
  await page.getByRole("button", { name: "Save favorite", exact: true }).click();
  await expect(page.locator("#pg-status")).toContainText("kept for this visit");
  await expect(page.locator("#pg-comparison")).toBeVisible();
});

test("invalid seeds keep the last valid preview and exports", async ({ page }) => {
  await page.goto(`${SITE}playground.html?guided=prism:1:42`);
  await ready(page);
  const css = await page.locator("#pg-css").textContent();
  await page.locator("#pg-seed").fill("18446744073709551616");
  await expect(page.locator("#pg-seed")).toHaveAttribute("aria-invalid", "true");
  await expect(page.locator("#pg-css")).toHaveText(css!);
  await page.locator("#pg-seed").fill("7");
  await expect(page.locator("#pg-rust")).toContainText("guided_v1(7)");
});

test("without scripts the previews, exports and direction destinations remain useful", async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto(SITE);
  await page.getByRole("link", { name: "Precision", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1 })).toContainText("precision / orders");
  await page.goto(`${SITE}playground.html`);
  await expect(page.locator("#pg-rust")).toContainText("Direction::Atelier.guided_v1(0)");
  await expect(page.locator("figcaption:visible").filter({ hasText: "Tasks completed per week: 2 → 7 across six weeks" })).toHaveCount(2);
  await context.close();
});

for (const direction of directions) {
  test(`${direction} complete screens fit 320px and pass axe in both schemes`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 800 });
    for (const screen of ["orders", "editorial", "project"]) {
      await page.goto(`/showcase-${direction}-${screen}.html`);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
      const result = await audit(page);
      expect(result.violations.map((v) => `${v.id}: ${v.nodes[0]?.target}`)).toEqual([]);
    }
  });
}

test("Prism becomes opaque for accessibility preferences and supports enlarged text", async ({ page }) => {
  await page.goto("/showcase-prism-project.html");
  await page.emulateMedia({ reducedMotion: "reduce", contrast: "more" });
  const card = page.locator(".showcase-project-focus").first();
  await expect.poll(() => card.evaluate((el) => ({
    contrast: matchMedia("(prefers-contrast: more)").matches,
    blur: getComputedStyle(el).backdropFilter,
  }))).toEqual({ contrast: true, blur: "none" });
  await page.emulateMedia({ forcedColors: "active" });
  await expect(card).toBeVisible();
  await page.setViewportSize({ width: 720, height: 900 });
  // Chromium can report one transient overflowing layout while text resizes;
  // the settled page must fit without horizontal scrolling.
  const scrollWidth = () => page.evaluate(() => document.documentElement.scrollWidth);
  await page.evaluate(() => document.documentElement.style.fontSize = "200%");
  await expect.poll(scrollWidth).toBeLessThanOrEqual(720);
  await page.evaluate(() => { document.documentElement.style.fontSize = ""; document.documentElement.style.zoom = "2"; });
  await expect.poll(scrollWidth).toBeLessThanOrEqual(720);
});

for (const direction of directions) {
  test(`${direction} retains error, selected, busy and overlay states`, async ({ page }) => {
    for (const scheme of ["light", "dark"]) {
      await page.goto("/forms.html");
      await page.evaluate(({ direction, scheme }) => { document.documentElement.dataset.stTheme = direction; document.documentElement.dataset.theme = scheme; }, { direction, scheme });
      await expect(page.getByLabel("Email", { exact: true })).toHaveAttribute("aria-invalid", "true");
      await expect(page.getByLabel("I agree to the terms")).toBeChecked();
      const result = await audit(page);
      expect(result.violations.map((v) => v.id)).toEqual([]);
    }
    await page.goto("/actions.html");
    await page.evaluate((direction) => document.documentElement.dataset.stTheme = direction, direction);
    await expect(page.getByRole("button", { name: "Disabled", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Saving", exact: true })).toHaveAttribute("aria-busy", "true");
    const ghost = page.getByRole("button", { name: "Ghost", exact: true });
    await ghost.hover(); await page.mouse.down();
    expect(await ghost.evaluate((el) => getComputedStyle(el).translate)).toBe("none");
    await page.mouse.up();
    await page.emulateMedia({ reducedMotion: "reduce" });
    const primary = page.getByRole("button", { name: "Primary", exact: true });
    await primary.hover(); await page.mouse.down();
    await expect.poll(() => primary.evaluate((el) => getComputedStyle(el).translate)).toBe("none");
    await page.mouse.up();
    await page.goto("/overlays.html");
    await page.evaluate((direction) => document.documentElement.dataset.stTheme = direction, direction);
    const opener = page.getByRole("button", { name: "Delete order", exact: true }).first();
    for (let i = 0; i < 3; i++) {
      await opener.click();
      await expect(page.locator("dialog#delete-order")).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(page.locator("dialog#delete-order")).toBeHidden();
      await expect(opener).toBeFocused();
    }
  });
}

test("reduced transparency turns Prism's frost into a solid surface", async ({ page, browserName }) => {
  test.skip(browserName !== "chromium", "CDP provides reduced-transparency emulation");
  await page.goto("/showcase-prism-project.html");
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setEmulatedMedia", { features: [{ name: "prefers-reduced-transparency", value: "reduce" }] });
  await expect.poll(() => page.locator(".showcase-project-focus").first().evaluate((el) => ({
    preference: matchMedia("(prefers-reduced-transparency: reduce)").matches,
    blur: getComputedStyle(el).backdropFilter,
  }))).toEqual({ preference: true, blur: "none" });
  await cdp.detach();
});

test("touch can switch directions and open compact native filters", async ({ browser }) => {
  const context = await browser.newContext({ hasTouch: true, viewport: { width: 360, height: 800 } });
  const page = await context.newPage();
  await page.goto(SITE);
  await page.locator('[data-preview-direction="play"]').tap();
  await expect(page.locator("#landing-preview")).toHaveAttribute("data-st-theme", "play");
  await page.goto("http://localhost:4181/orders");
  await page.getByText("More filters & sorting", { exact: true }).tap();
  await expect(page.locator("details.st-filter-secondary")).toHaveAttribute("open", "");
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(360);
  await context.close();
});
