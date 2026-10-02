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
  const custom = preset.querySelector('option[value="custom"]');
  const sync = () => {
    const saved = readSaved();
    custom.hidden = custom.disabled = !saved?.css;
    // Seeds are up to 20 digits: keep the option short enough to read.
    const label = saved?.label?.length > 18 ? `${saved.label.slice(0, 17)}…` : saved?.label;
    custom.textContent = saved?.css ? `Custom: ${label}` : "Custom";
    preset.value = saved?.css ? "custom" : saved?.preset ?? "slate";
    scheme.value = root.dataset.theme ?? "system";
  };
  sync();
  document.addEventListener("site-theme-change", sync);
  menu.hidden = false;

  preset.addEventListener("change", () => {
    if (preset.value === "custom") return;
    applyThemeAndSave({ preset: preset.value });
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
    status.textContent = "Theme reset to Slate.";
  });
  const edit = menu.querySelector('[data-site-action="edit"]');
  edit.addEventListener("click", () => {
    const saved = readSaved();
    const query = saved?.css ? saved.query : `preset=${saved?.preset ?? "slate"}`;
    edit.href = `${edit.href.split("?")[0]}?${query}`;
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
  const preset = byId("pg-preset");
  const seed = byId("pg-seed");
  const status = byId("pg-status");
  const options = [...form.querySelectorAll("select")].filter((s) => s !== preset);
  // Digits are a seed (Theme::seeded); other text is a name (Theme::seeded_str).
  const numeric = () => /^\d+$/.test(seed.value.trim());
  const validSeed = () => numeric() && BigInt(seed.value.trim()) < 2n ** 64n;
  let current;
  let ticket = 0;

  const query = () => {
    const params = new URLSearchParams();
    const text = seed.value.trim();
    if (validSeed()) params.set("seed", text);
    else if (text && !numeric()) params.set("name", text);
    else params.set("preset", preset.value);
    for (const select of options) if (select.value) params.set(select.name, select.value);
    return params.toString();
  };

  const load = (search) => {
    const params = new URLSearchParams(search);
    if (params.has("preset")) preset.value = params.get("preset");
    seed.value = params.get("seed") ?? params.get("name") ?? "";
    for (const select of options) select.value = params.get(select.name) ?? "";
  };

  async function update(remember) {
    const mine = ++ticket;
    seed.setAttribute("aria-invalid", String(numeric() && !validSeed()));
    let result;
    try {
      result = await generate(query(), "playground");
    } catch {
      status.textContent = "The theme engine could not load, so the preview can't update.";
      return;
    }
    if (mine !== ticket) return;
    current = result;
    // A seed or name replaces the preset as the starting point.
    preset.disabled = !result.query.startsWith("preset=");
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
    if (remember) history.replaceState(null, "", `?${result.query}`);
  }

  form.addEventListener("input", () => update(true));
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    update(true);
  });
  form.addEventListener("click", async (event) => {
    const action = event.target.closest("[data-pg-action]")?.dataset.pgAction;
    if (action === "random") {
      seed.value = randomSeed();
      await update(true);
      status.textContent = `Seed ${seed.value}.`;
    } else if (action === "clear") {
      seed.value = "";
      await update(true);
      status.textContent = "Starting from the preset.";
    } else if (action === "apply" && current) {
      try {
        await useGenerated(current.query);
        status.textContent = `The site now uses ${current.label}${current.query.includes("&") ? " with your options" : ""}.`;
      } catch {
        status.textContent = "The theme engine could not load.";
      }
    } else if (action === "link") {
      const ok = await copyText(location.href);
      status.textContent = ok ? "Link copied." : "Copy the address bar to share this theme.";
    }
  });

  load(location.search);
  update(location.search !== "");
}
