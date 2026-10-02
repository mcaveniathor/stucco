// stucco client runtime: loads the behaviour modules that inserted fragments
// require. Contract placeholders (see stucco_core::behavior) are substituted
// when the bundle is built.

/** Import promises by absolute URL, so each module is requested once. */
const loaded = new Map();

/** The bundle directory: only modules served beside this file are accepted. */
const base = new URL(".", import.meta.url);

function accept(raw) {
  let url;
  try {
    url = new URL(raw, location.href);
  } catch {
    return null;
  }
  if (url.origin !== location.origin || !url.pathname.startsWith(base.pathname)) {
    console.error(`stucco: refusing module outside the bundle: ${raw}`);
    return null;
  }
  return url.href;
}

function load(href) {
  if (!loaded.has(href)) loaded.set(href, import(href));
  return loaded.get(href);
}

function fail(region, href) {
  region.setAttribute("__STUCCO_STATE_ATTR__", "error");
  region.dispatchEvent(
    new CustomEvent("__STUCCO_ASSET_ERROR_EVENT__", { bubbles: true, detail: { url: href } }),
  );
}

class Require extends HTMLElement {
  connectedCallback() {
    const region =
      this.parentElement?.closest("[__STUCCO_REGION_ATTR__]") ?? this.parentElement ?? document.body;
    const urls = (this.getAttribute("modules") ?? "").split(/\s+/).filter(Boolean);
    for (const raw of urls) {
      const href = accept(raw);
      if (href) load(href).catch(() => fail(region, href));
    }
    this.remove();
  }
}

if (!customElements.get("__STUCCO_REQUIRE_TAG__")) {
  customElements.define("__STUCCO_REQUIRE_TAG__", Require);
}
