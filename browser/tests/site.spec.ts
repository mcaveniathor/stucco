import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

// The documentation site, served by the "site" web server in the config.
const SITE = "http://localhost:4182/";

const openThemeMenu = async (page: Page) => {
  const menu = page.locator('[data-site-theme="menu"]');
  await expect(menu).toBeVisible();
  if ((await menu.getAttribute("open")) === null) await menu.locator("summary").click();
  await expect(page.locator(".site-theme-panel")).toBeVisible();
};

/** Points inside the open theme panel that the panel itself doesn't paint. */
const panelGaps = (page: Page) =>
  page.evaluate(() => {
    const panel = document.querySelector(".site-theme-panel")!;
    const r = panel.getBoundingClientRect();
    const inset = 24;
    const points = [
      [r.left + inset, r.top + inset],
      [r.right - inset, r.top + inset],
      [r.left + inset, r.bottom - inset],
      [r.right - inset, r.bottom - inset],
      [r.left + r.width / 2, r.bottom - 12],
    ];
    return points
      .filter(([x, y]) => x < 0 || x > innerWidth || !panel.contains(document.elementFromPoint(x, y)))
      .map(([x, y]) => `${Math.round(x)},${Math.round(y)}`);
  });

/** Applies a theme from the playground's query string to the whole site. */
const useTheme = async (page: Page, query: string) => {
  await page.goto(`${SITE}playground.html?${query}`);
  // The engine has built the theme once the options show the base's choices.
  await expect(page.locator("#pg-corners option").first()).toHaveText(/^Base \(.+\)$/);
  const status = page.locator("#pg-status");
  await page.getByRole("button", { name: "Use on this site" }).click();
  await expect(status).toContainText("The site now uses");
};

test("the theme menu stays whole and usable through many random themes", async ({ page }) => {
  await page.goto(SITE);
  await openThemeMenu(page);
  const status = page.locator('[data-site-theme="status"]');
  for (let i = 0; i < 12; i++) {
    await status.evaluate((s) => (s.textContent = ""));
    // A real click: fails if anything covers the button.
    await page.getByRole("button", { name: "Random theme" }).click();
    await expect(status).toHaveText(/^Theme: /);
    expect(await panelGaps(page), `theme ${i + 1}: ${await status.textContent()}`).toEqual([]);
  }
});

for (const edge of ["wave", "zigzag", "torn"]) {
  test(`a ${edge} header edge doesn't clip the theme menu`, async ({ page }) => {
    await useTheme(page, `preset=slate&edge=${edge}&pattern=grid&material=glass`);
    await page.goto(SITE);
    await openThemeMenu(page);
    expect(await panelGaps(page)).toEqual([]);
    await page.getByRole("button", { name: "Reset" }).click();
    await expect(page.locator('[data-site-theme="status"]')).toHaveText("Theme reset to Slate.");
  });
}

test("the theme menu fits a phone screen", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 740 });
  await page.goto(SITE);
  await openThemeMenu(page);
  const box = (await page.locator(".site-theme-panel").boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(360);
  expect(await panelGaps(page)).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(360);
});

test("a chosen theme follows the visitor between pages, and Reset clears it", async ({ page }) => {
  await page.goto(SITE);
  await openThemeMenu(page);
  await page.locator("#site-theme-preset").selectOption("ocean");
  await expect(page.locator("html")).toHaveAttribute("data-st-theme", "ocean");
  await page.goto(`${SITE}guide/`);
  await expect(page.locator("html")).toHaveAttribute("data-st-theme", "ocean");
  await openThemeMenu(page);
  await expect(page.locator("#site-theme-preset")).toHaveValue("ocean");
  await page.getByRole("button", { name: "Reset" }).click();
  await expect(page.locator("html")).not.toHaveAttribute("data-st-theme", /./);
});

test("Escape and a click elsewhere close the theme menu", async ({ page }) => {
  await page.goto(SITE);
  const menu = page.locator('[data-site-theme="menu"]');
  await openThemeMenu(page);
  await page.keyboard.press("Escape");
  await expect(menu).not.toHaveAttribute("open", /.*/);
  await expect(menu.locator("summary")).toBeFocused();
  await openThemeMenu(page);
  await page.getByRole("heading", { level: 1 }).click();
  await expect(menu).not.toHaveAttribute("open", /.*/);
});

for (const path of ["gallery/", "gallery/actions.html", "gallery/app.html", "gallery/palette.html"]) {
  test(`${path} has the site header after a skip link`, async ({ page }) => {
    await page.goto(SITE + path);
    // The skip link is the first stop, then the site's navigation.
    const order = await page.evaluate(() =>
      [...document.querySelectorAll("a[href], button, summary, input, select, textarea")]
        .filter((e) => !e.closest("[hidden]"))
        .slice(0, 2)
        .map((e) => e.textContent?.trim()),
    );
    expect(order).toEqual(["Skip to main content", "stucco"]);
    // WebKit's Tab skips links unless the system setting says otherwise.
    if (test.info().project.name !== "webkit") {
      await page.keyboard.press("Tab");
      await expect(page.locator(":focus")).toHaveText("Skip to main content");
      await page.keyboard.press("Tab");
      await expect(page.locator(":focus")).toHaveText("stucco");
    }
    const nav = page.getByRole("navigation", { name: "Site" });
    await expect(nav.getByRole("link", { name: "Gallery" })).toHaveAttribute("aria-current", "page");
    await expect(page.locator(".st-skip-link")).toHaveCount(1);
    await nav.getByRole("link", { name: "Playground" }).click();
    await expect(page).toHaveURL(`${SITE}playground.html`);
  });
}

test("the palette page links back to the gallery", async ({ page }) => {
  await page.goto(`${SITE}gallery/palette.html`);
  await page.getByRole("link", { name: "← Gallery" }).click();
  await expect(page).toHaveURL(`${SITE}gallery/index.html`);
});

test("the playground keeps its preview in view beside the options", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(`${SITE}playground.html`);
  // The options sit in more than one column, labelled with the base's choice.
  const corners = page.locator("#pg-corners");
  await expect(corners.locator("option").first()).toHaveText(/^Base \(.+\)$/);
  const [first, second] = await Promise.all(
    ["#pg-radius", "#pg-density"].map((id) => page.locator(id).boundingBox()),
  );
  expect(second!.y).toBe(first!.y);
  // The actions come before the options.
  const apply = (await page.getByRole("button", { name: "Use on this site" }).boundingBox())!;
  expect(apply.y).toBeLessThan(first!.y);
  // Partway through the options, the preview is pinned to the top of the
  // screen; with the last option at the bottom, it is still wholly on screen.
  const preview = page.locator(".site-pg-preview");
  await page.locator("#pg-motion").evaluate((e) => e.scrollIntoView({ block: "center" }));
  expect(Math.round((await preview.boundingBox())!.y)).toBe(16);
  await page.locator("#pg-edge").evaluate((e) => e.scrollIntoView({ block: "end" }));
  const pinned = (await preview.boundingBox())!;
  expect(pinned.y).toBeGreaterThanOrEqual(0);
  expect(pinned.y + pinned.height).toBeLessThanOrEqual(800);
  // Changing an option updates the preview's theme and the address.
  await page.locator("#pg-edge").selectOption("wave");
  await expect(page).toHaveURL(/edge=wave/);
  await expect(page.locator("#pg-rust")).toContainText("HeaderEdge::Wave");
});

test("the playground is one column on a phone", async ({ page }) => {
  await page.setViewportSize({ width: 360, height: 740 });
  await page.goto(`${SITE}playground.html`);
  const controls = (await page.locator("#pg-form").boundingBox())!;
  const preview = (await page.locator(".site-pg-preview").boundingBox())!;
  expect(preview.y).toBeGreaterThan(controls.y + controls.height - 1);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(360);
});

for (const width of [1280, 360]) {
  for (const path of ["", "guide/", "playground.html", "gallery/", "gallery/app.html", "gallery/palette.html"]) {
    test(`/${path} passes axe at ${width}px`, async ({ page }) => {
      await page.setViewportSize({ width, height: 800 });
      await page.goto(SITE + path);
      const results = await new AxeBuilder({ page }).analyze();
      expect(results.violations.map((v) => `${v.id}: ${v.nodes[0]?.target}`)).toEqual([]);
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    });
  }
}
