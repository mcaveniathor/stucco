// The stucco site's enhancements: the theme menu, random themes, the
// playground, and copy and download buttons. Every page works without it.

const THEME_KEY = "stucco-site-theme";
// stucco's own scheme key, read by the theme script in every page's head.
const SCHEME_KEY = "stucco-theme";
const root = document.documentElement;
const wasmUrl = new URL("site_wasm.wasm", import.meta.url);

// ---- Theme engine (Rust compiled to WebAssembly) --------------------------

let engine;

function loadEngine() {
  engine ??= WebAssembly.instantiateStreaming(fetch(wasmUrl))
    .catch(async () => WebAssembly.instantiate(await (await fetch(wasmUrl)).arrayBuffer()))
    .then(({ instance }) => instance.exports)
    .catch((error) => {
      engine = undefined;
      throw error;
    });
  return engine;
}

/** Runs the engine: `{query, label, rust, css, scoped, tokens, summary}`. */
async function generate(query, scope) {
  const x = await loadEngine();
  const encoder = new TextEncoder();
  const put = (text) => {
    const bytes = encoder.encode(text);
    const ptr = x.alloc(bytes.length);
    new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
    return [ptr, bytes.length];
  };
  const [queryPtr, queryLen] = put(query);
  const [scopePtr, scopeLen] = put(scope);
  const packed = x.generate(queryPtr, queryLen, scopePtr, scopeLen);
  const ptr = Number(packed >> 32n);
  const len = Number(packed & 0xffffffffn);
  const json = new TextDecoder().decode(new Uint8Array(x.memory.buffer, ptr, len));
  x.dealloc(ptr, len);
  x.dealloc(queryPtr, queryLen);
  x.dealloc(scopePtr, scopeLen);
  return JSON.parse(json);
}

/** A random 64-bit seed, as a decimal string. */
function randomSeed() {
  return crypto.getRandomValues(new BigUint64Array(1))[0].toString();
}

// ---- Site theme ------------------------------------------------------------

function readSaved() {
  try {
    return JSON.parse(localStorage.getItem(THEME_KEY) ?? "null");
  } catch {
    return null;
  }
}

function save(state) {
  try {
    if (state) localStorage.setItem(THEME_KEY, JSON.stringify(state));
    else localStorage.removeItem(THEME_KEY);
  } catch {
    // Storage can be unavailable (private windows); the theme still applies now.
  }
}

function applyTheme(state) {
  document.getElementById("site-theme-css")?.remove();
  delete root.dataset.stTheme;
  if (state?.css) {
    const style = document.createElement("style");
    style.id = "site-theme-css";
    style.textContent = state.css;
    document.head.append(style);
    root.dataset.stTheme = "site-custom";
  } else if (state?.preset) {
    root.dataset.stTheme = state.preset;
  }
  document.dispatchEvent(new CustomEvent("site-theme-change"));
}

function applyScheme(scheme) {
  try {
    if (scheme === "system") localStorage.removeItem(SCHEME_KEY);
    else localStorage.setItem(SCHEME_KEY, scheme);
  } catch {
    // As above: applies for this page view only.
  }
  if (scheme === "system") delete root.dataset.theme;
  else root.dataset.theme = scheme;
}

/** Saves and applies a generated theme as the site theme. */
async function useGenerated(query) {
  const result = await generate(query, "site-custom");
  const state = { query: result.query, label: result.label, css: result.scoped };
  save(state);
  applyTheme(state);
  return result;
}

for (const menu of document.querySelectorAll('[data-site-theme="menu"]')) {
  const preset = menu.querySelector("#site-theme-preset");
  const scheme = menu.querySelector("#site-theme-scheme");
  const status = menu.querySelector('[data-site-theme="status"]');
  const edit = menu.querySelector('[data-site-action="edit"]');
  const playgroundUrl = edit.href.split("?")[0];
  const custom = preset.querySelector('option[value="custom"]');
  const sync = () => {
    const saved = readSaved();
    custom.hidden = custom.disabled = !saved?.css;
    // Seeds are up to 20 digits: keep the option short enough to read.
    const label = saved?.label?.length > 18 ? `${saved.label.slice(0, 17)}…` : saved?.label;
    custom.textContent = saved?.css ? `Custom: ${label}` : "Custom";
    preset.value = saved?.css ? "custom" : saved?.preset ?? "";
    scheme.value = root.dataset.theme ?? "system";
    const query = saved?.css ? saved.query : saved?.preset ? `preset=${saved.preset}` : null;
    edit.href = query ? `${playgroundUrl}?${query}` : edit.dataset.defaultRecipe;
    edit.textContent = query ? "Edit in the playground" : "View Atelier recipe";
  };
  sync();
  document.addEventListener("site-theme-change", sync);
  menu.hidden = false;

  preset.addEventListener("change", () => {
    if (preset.value === "custom") return;
    applyThemeAndSave(preset.value ? { preset: preset.value } : null);
    status.textContent = `Theme: ${preset.selectedOptions[0].textContent}.`;
  });
  scheme.addEventListener("change", () => applyScheme(scheme.value));
  menu.querySelector('[data-site-action="random"]').addEventListener("click", async () => {
    status.textContent = "Generating…";
    try {
      const result = await useGenerated(`seed=${randomSeed()}`);
      status.textContent = `Theme: ${result.label}.`;
    } catch {
      status.textContent = "The theme engine could not load. Try again later.";
    }
  });
  menu.querySelector('[data-site-action="reset"]').addEventListener("click", () => {
    applyThemeAndSave(null);
    applyScheme("system");
    sync();
    status.textContent = "Theme reset to Atelier.";
  });

}

function applyThemeAndSave(state) {
  save(state);
  applyTheme(state);
}

// Close open theme menus with Escape or a click elsewhere.
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  for (const menu of document.querySelectorAll('[data-site-theme="menu"][open]')) {
    menu.open = false;
    menu.querySelector("summary").focus();
  }
});
document.addEventListener("click", (event) => {
  for (const menu of document.querySelectorAll('[data-site-theme="menu"][open]')) {
    if (!menu.contains(event.target)) menu.open = false;
  }
});

// ---- Copy and download -----------------------------------------------------

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

function flash(button, label) {
  const original = button.dataset.label ?? button.textContent;
  button.dataset.label = original;
  button.textContent = label;
  setTimeout(() => {
    button.textContent = original;
  }, 1600);
}

document.addEventListener("click", async (event) => {
  const copy = event.target.closest("[data-copy]");
  if (copy) {
    const ok = await copyText(document.getElementById(copy.dataset.copy).textContent);
    flash(copy, ok ? "Copied" : "Copy failed");
    return;
  }
  const download = event.target.closest("[data-download]");
  if (download) {
    const text = document.getElementById(download.dataset.download).textContent;
    const link = document.createElement("a");
    link.href = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
    link.download = download.dataset.filename;
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 1000);
  }
});

// ---- Playground ------------------------------------------------------------

const form = document.getElementById("pg-form");
if (form) playground(form);

function playground(form) {
  const byId = (id) => document.getElementById(id);
  const preset = byId("pg-preset"), seed = byId("pg-seed"), mode = byId("pg-mode"), direction = byId("pg-direction");
  const status = byId("pg-status");
  const options = [...form.querySelectorAll(".site-pg-options select")];
  const numeric = () => /^\d+$/.test(seed.value.trim());
  const validSeed = () => numeric() && BigInt(seed.value.trim()) < 2n ** 64n;
  const typeKeys = ["heading-font", "headings", "leading", "labels", "icons"];
  const groupFor = (key) => key === "palette" ? "palette" : typeKeys.includes(key) ? "type" : "structure";
  const locks = [...form.querySelectorAll("[data-pg-lock]")];
  const favoriteSelect = byId("pg-favorite");
  const FAVORITES = "stucco-favorites-v1";
  let current, ticket = 0, comparisonTicket = 0, favorites = [];
  try {
    const saved = JSON.parse(localStorage.getItem(FAVORITES) ?? "[]");
    if (Array.isArray(saved)) favorites = saved.filter((f) => typeof f?.query === "string" && f.query.length < 8192 && typeof f?.label === "string").slice(0, 6);
  } catch { /* Local discovery still works when storage is unavailable. */ }

  const query = () => {
    const params = new URLSearchParams();
    const text = seed.value.trim();
    if (mode.value === "guided") params.set("guided", `${direction.value}:1:${text || "0"}`);
    else if (validSeed()) params.set("seed", text);
    else if (text && !numeric()) params.set("name", text);
    else params.set("preset", preset.value);
    for (const select of options) if (select.value) params.set(select.name, select.value);
    return params.toString();
  };
  const load = (search) => {
    const params = new URLSearchParams(search);
    mode.value = !search || params.has("guided") || params.has("direction") ? "guided" : "unrestricted";
    const guided = params.get("guided")?.split(":");
    if (guided) {
      direction.value = guided[0];
      seed.value = guided[2] ?? "0";
    } else {
      direction.value = params.get("direction") ?? "atelier";
      seed.value = params.get("seed") ?? params.get("name") ?? "";
    }
    if (!direction.value) direction.value = "atelier";
    if (params.has("preset")) preset.value = params.get("preset");
    for (const select of options) select.value = params.get(select.name) ?? "";
  };
  async function update(remember) {
    const mine = ++ticket;
    const guided = mode.value === "guided";
    direction.disabled = !guided;
    preset.disabled = guided || seed.value.trim() !== "";
    byId("pg-locks").disabled = !guided;
    const invalid = (numeric() && !validSeed()) || (guided && seed.value.trim() !== "" && !validSeed());
    seed.setAttribute("aria-invalid", String(invalid));
    if (invalid) {
      status.textContent = "Use a whole-number seed from 0 to 18446744073709551615. The preview still shows the last valid theme.";
      return false;
    }
    let result;
    try { result = await generate(query(), "playground"); }
    catch {
      status.textContent = "The theme engine could not load, so the preview can't update.";
      return false;
    }
    if (mine !== ticket) return false;
    current = result;
    byId("pg-theme").textContent = result.scoped;
    byId("pg-rust").textContent = result.rust;
    byId("pg-css").textContent = result.css;
    byId("pg-json").textContent = result.tokens;
    byId("pg-label").textContent = result.label;
    for (const select of options) {
      const value = result.summary[select.name];
      const choice = value && [...select.options].find((o) => o.value === value);
      select.options[0].textContent = choice ? `Base (${choice.textContent})` : "Base";
    }
    if (remember) history.replaceState(null, "", `?${result.query}${location.hash}`);
    return true;
  }

  function view() {
    const screen = byId("pg-screen").value, scheme = byId("pg-scheme").value;
    document.querySelectorAll("[data-pg-screen]").forEach((el) => el.hidden = el.dataset.pgScreen !== screen);
    document.querySelectorAll("[data-pg-scheme]").forEach((el) => el.hidden = scheme !== "both" && el.dataset.pgScheme !== scheme);
    document.querySelectorAll(".site-pg-screens").forEach((el) => el.classList.toggle("site-pg-narrow", byId("pg-narrow").checked));
  }
  for (const id of ["pg-screen", "pg-scheme", "pg-narrow"]) byId(id).addEventListener("change", view);
  function listFavorites(selected = "") {
    favoriteSelect.replaceChildren(new Option("Choose a saved theme", ""));
    favorites.forEach((f, i) => favoriteSelect.add(new Option(f.label, String(i))));
    favoriteSelect.value = selected;
  }
  function persistFavorites() {
    try { localStorage.setItem(FAVORITES, JSON.stringify(favorites)); return true; }
    catch { return false; }
  }
  async function compare() {
    const mine = ++comparisonTicket;
    byId("pg-comparison").hidden = true;
    if (favoriteSelect.value === "") return;
    const favorite = favorites[Number(favoriteSelect.value)];
    if (!favorite) return;
    try {
      const result = await generate(favorite.query, "comparison");
      if (mine !== comparisonTicket) return;
      byId("pg-comparison-theme").textContent = result.scoped;
      byId("pg-comparison-label").textContent = result.label;
      byId("pg-comparison").hidden = false;
      view();
    } catch { status.textContent = "The saved theme could not be loaded."; }
  }
  favoriteSelect.addEventListener("change", compare);
  byId("pg-load-favorite").addEventListener("click", async () => {
    if (favoriteSelect.value === "") return;
    load(favorites[Number(favoriteSelect.value)].query);
    await update(true);
  });
  byId("pg-remove-favorite").addEventListener("click", () => {
    if (favoriteSelect.value === "") return;
    favorites.splice(Number(favoriteSelect.value), 1);
    const stored = persistFavorites();
    listFavorites(); compare();
    status.textContent = stored ? "Favorite removed." : "Removed for this visit; local storage is unavailable.";
  });
  form.addEventListener("input", (event) => {
    if (event.target === direction) {
      seed.value = "0";
      for (const select of options) select.value = "";
      for (const lock of locks) lock.checked = false;
    }
    update(true);
  });
  form.addEventListener("submit", (event) => { event.preventDefault(); update(true); });
  form.addEventListener("click", async (event) => {
    const action = event.target.closest("[data-pg-action]")?.dataset.pgAction;
    if (action === "random") {
      // Resolve the current controls before capturing locks, including rapid input.
      if (!(await update(false))) return;
      const retained = [];
      if (mode.value === "guided") {
        for (const select of options) {
          const locked = locks.find((lock) => lock.dataset.pgLock === groupFor(select.name))?.checked;
          select.value = locked ? current.summary[select.name] ?? "" : "";
        }
        for (const lock of locks) if (lock.checked) retained.push(lock.parentElement.textContent.replace("Lock ", "").trim());
      }
      seed.value = randomSeed();
      if (await update(true)) status.textContent = `Seed ${seed.value}.${retained.length ? ` Kept ${retained.join(", ")}.` : ""}`;
    } else if (action === "clear") {
      seed.value = "";
      if (await update(true)) status.textContent = mode.value === "guided" ? "Starting from the signature with your overrides." : "Starting from the preset.";
    } else if (action === "apply") {
      if (!(await update(true))) return;
      try {
        const chosen = current;
        await useGenerated(chosen.query);
        status.textContent = `The site now uses ${chosen.label}${chosen.query.includes("&") ? " with your options" : ""}.`;
      } catch { status.textContent = "The theme engine could not load."; }
    } else if (action === "link") {
      if (!(await update(true))) return;
      const ok = await copyText(location.href);
      status.textContent = ok ? "Link copied." : "Copy the address bar to share this theme.";
    } else if (action === "favorite") {
      if (!(await update(true))) return;
      if (favorites.some((f) => f.query === current.query)) { status.textContent = "This theme is already saved."; return; }
      if (favorites.length === 6) { status.textContent = "Six favorites saved. Remove one before saving another."; return; }
      favorites.push({ query: current.query, label: current.label });
      const stored = persistFavorites();
      listFavorites(String(favorites.length - 1));
      await compare();
      status.textContent = stored ? "Favorite saved on this device." : "Favorite kept for this visit; local storage is unavailable.";
    }
  });
  load(location.search);
  listFavorites(); view(); update(location.search !== "");
}

// Landing links remain ordinary links to complete server-rendered showcases.
const directionPreview = document.getElementById("landing-preview");
if (directionPreview) {
  document.querySelectorAll("[data-preview-direction]").forEach((link) => {
    link.addEventListener("click", (event) => {
      if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      event.preventDefault();
      const name = link.dataset.previewDirection;
      directionPreview.dataset.stTheme = name;
      document.querySelectorAll("[data-preview-direction]").forEach((other) => {
        if (other === link) other.setAttribute("aria-current", "true");
        else other.removeAttribute("aria-current");
      });
      document.getElementById("landing-preview-label").textContent = name.toUpperCase();
      document.getElementById("landing-showcase-link").href = link.href;
    });
  });
}
