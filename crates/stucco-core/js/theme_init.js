try {
  const t = localStorage.getItem("__STUCCO_THEME_STORAGE_KEY__");
  if (t === "light" || t === "dark") document.documentElement.dataset.theme = t;
} catch (_) {}
