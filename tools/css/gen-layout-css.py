# Generates crates/stucco-ui/css/layout.css (run from the repo root: python tools/css/gen-layout-css.py).
SPACES = ["0", "1", "2", "3", "4", "5", "6", "8", "10", "12"]
MEASURES = {"xs": "20rem", "sm": "30rem", "md": "40rem", "lg": "60rem", "xl": "75rem", "prose": "65ch"}
def sp(s): return "0" if s == "0" else f"var(--st-space-{s})"
out = ["@layer stucco.layout {",
"  /* Stack: vertical rhythm between children. */",
"  .st-stack { display: flex; flex-direction: column; justify-content: flex-start; }",
"  .st-stack > * { margin-block: 0; }"]
for s in SPACES:
    out.append(f'  .st-stack[data-space="{s}"] > * + * {{ margin-block-start: {sp(s)}; }}')
    out.append(f'  .st-stack[data-space="{s}"][data-recursive] * + * {{ margin-block-start: {sp(s)}; }}')
out += ["  /* Cluster: wrapping inline group. */",
"  .st-cluster { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-start; }"]
for s in SPACES: out.append(f'  .st-cluster[data-space="{s}"] {{ gap: {sp(s)}; }}')
for k, v in {"start": "flex-start", "center": "center", "end": "flex-end", "between": "space-between"}.items():
    out.append(f'  .st-cluster[data-justify="{k}"] {{ justify-content: {v}; }}')
for k, v in {"start": "flex-start", "center": "center", "end": "flex-end", "baseline": "baseline", "stretch": "stretch"}.items():
    out.append(f'  .st-cluster[data-align="{k}"] {{ align-items: {v}; }}')
out += ["  /* Grid: as many columns as fit at the minimum width. */", "  .st-grid { display: grid; }"]
for k, v in MEASURES.items(): out.append(f'  .st-grid[data-min="{k}"] {{ grid-template-columns: repeat(auto-fit, minmax(min(100%, {v}), 1fr)); }}')
for s in SPACES: out.append(f'  .st-grid[data-space="{s}"] {{ gap: {sp(s)}; }}')
out += ["  /* Center: a horizontally centred measure. */", "  .st-center { box-sizing: content-box; margin-inline: auto; }"]
for k, v in MEASURES.items(): out.append(f'  .st-center[data-max="{k}"] {{ max-inline-size: {v}; }}')
for s in SPACES: out.append(f'  .st-center[data-gutters="{s}"] {{ padding-inline: {sp(s)}; }}')
out.append("  .st-center[data-intrinsic] { display: flex; flex-direction: column; align-items: center; }")
out += ["  /* Container: page width with responsive gutters. */",
"  .st-container { margin-inline: auto; padding-inline: clamp(var(--st-space-4), 4vw, var(--st-space-8)); }"]
for k, v in MEASURES.items(): out.append(f'  .st-container[data-size="{k}"] {{ max-inline-size: {v}; }}')
out.append("}")
open("crates/stucco-ui/css/layout.css", "w", newline="\n").write("\n".join(out) + "\n")

# --- Task 5 additions (appended inside the layer) ---
css = open("crates/stucco-ui/css/layout.css").read().rstrip()
assert css.endswith("}")
more = ["  /* Sidebar: a fixed-width side beside a fluid main that wraps when cramped. */",
"  .st-sidebar { display: flex; flex-wrap: wrap; }",
"  .st-sidebar > .st-sidebar-side { flex-grow: 1; }",
"  .st-sidebar > .st-sidebar-main { flex-basis: 0; flex-grow: 999; min-inline-size: 50%; }"]

for k, v in MEASURES.items(): more.append(f'  .st-sidebar[data-side-width="{k}"] > .st-sidebar-side {{ flex-basis: {v}; }}')
for s in SPACES: more.append(f'  .st-sidebar[data-space="{s}"] {{ gap: {sp(s)}; }}')
more += ["  /* Switcher: a row that becomes a column below the threshold. */",
"  .st-switcher { display: flex; flex-wrap: wrap; }", "  .st-switcher > * { flex-grow: 1; }"]
for k, v in MEASURES.items(): more.append(f'  .st-switcher[data-threshold="{k}"] > * {{ flex-basis: calc(({v} - 100%) * 999); }}')
for s in SPACES: more.append(f'  .st-switcher[data-space="{s}"] {{ gap: {sp(s)}; }}')
for n in range(2, 7):
    more.append(f'  .st-switcher[data-limit="{n}"] > :nth-last-child(n+{n+1}), .st-switcher[data-limit="{n}"] > :nth-last-child(n+{n+1}) ~ * {{ flex-basis: 100%; }}')
more += ["  /* Separator. */",
"  .st-separator { border: 0; border-block-start: 1px solid var(--st-border); margin-block: var(--st-space-4); }",
"  /* Skip link: hidden until focused. */",
"  .st-skip-link { position: absolute; inset-inline-start: var(--st-space-2); inset-block-start: var(--st-space-2); z-index: var(--st-z-overlay); padding: var(--st-space-2) var(--st-space-4); background: var(--st-surface); color: var(--st-accent-text); border: 2px solid var(--st-focus); border-radius: var(--st-radius-md); transform: translateY(-200%); }",
"  .st-skip-link:focus { transform: none; }",
"  /* Surface: a background plane. */",
"  .st-surface { border-radius: var(--st-radius-md); }",
'  .st-surface[data-level="flat"] { background: var(--st-bg); }',
'  .st-surface[data-level="raised"] { background: var(--st-surface); }',
'  .st-surface[data-level="overlay"] { background: var(--st-surface-raised); box-shadow: var(--st-shadow-2); }',
"  .st-surface[data-border] { border: 1px solid var(--st-border); }"]
for s in SPACES: more.append(f'  .st-surface[data-padding="{s}"] {{ padding: {sp(s)}; }}')
for r in ["sm", "md", "lg"]: more.append(f'  .st-surface[data-radius="{r}"] {{ border-radius: var(--st-radius-{r}); }}')
more += ["  /* Theme scope: repaints its subtree in another scheme or theme. */",
"  .st-theme-scope { background: var(--st-bg); color: var(--st-text); }"]
open("crates/stucco-ui/css/layout.css", "w", newline="\n").write(css[:-1].rstrip() + "\n" + "\n".join(more) + "\n}\n")
