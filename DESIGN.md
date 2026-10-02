---
version: alpha
name: stucco
description: Server-rendered, themeable, accessible UI components for Rust. Token values are the default Slate preset in its light scheme, comfortable density and soft radius.
colors:
  bg: oklch(99.00% 0.0012 250.00)
  surface: oklch(97.50% 0.0018 250.00)
  surface-raised: oklch(95.00% 0.0030 250.00)
  hover: oklch(92.00% 0.0042 250.00)
  text: oklch(24.00% 0.0054 250.00)
  text-muted: oklch(45.00% 0.0102 250.00)
  border: oklch(84.50% 0.0066 250.00)
  border-strong: oklch(60.00% 0.0120 250.00)
  accent: oklch(60.00% 0.1500 250.00)
  accent-hover: oklch(71.00% 0.1200 250.00)
  accent-text: oklch(45.00% 0.1275 250.00)
  accent-soft: oklch(95.00% 0.0248 250.00)
  on-accent: oklch(0.00% 0.0000 0.00)
  focus: oklch(60.00% 0.1500 250.00)
  success: oklch(60.00% 0.1400 150.00)
  success-text: oklch(45.00% 0.1190 150.00)
  success-soft: oklch(95.00% 0.0350 150.00)
  warning: oklch(60.00% 0.1230 85.00)
  warning-text: oklch(45.00% 0.0924 85.00)
  warning-soft: oklch(95.00% 0.0350 85.00)
  danger: oklch(60.00% 0.1400 25.00)
  danger-text: oklch(45.00% 0.1190 25.00)
  danger-soft: oklch(95.00% 0.0249 25.00)
  info: oklch(60.00% 0.1375 240.00)
  info-text: oklch(45.00% 0.1032 240.00)
  info-soft: oklch(95.00% 0.0267 240.00)
typography:
  sans:
    fontFamily: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif
  mono:
    fontFamily: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace
  xs:
    fontSize: 0.6944rem
  sm:
    fontSize: 0.8333rem
  base:
    fontSize: 1rem
    lineHeight: 1.5
  lg:
    fontSize: 1.2rem
  xl:
    fontSize: 1.44rem
  2xl:
    fontSize: 1.728rem
  3xl:
    fontSize: 2.0736rem
rounded:
  sm: 4px
  md: 8px
  lg: 12px
  full: 9999px
spacing:
  "1": 4px
  "2": 8px
  "3": 12px
  "4": 16px
  "5": 20px
  "6": 24px
  "7": 32px
  "8": 40px
  "9": 48px
  "10": 64px
  "11": 80px
  "12": 96px
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-accent}"
    rounded: "{rounded.md}"
    height: 40px
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    height: 40px
  input:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    height: 40px
  card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
    padding: "{spacing.6}"
  tag:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.text}"
    rounded: "{rounded.full}"
---

# stucco

## Overview

stucco renders interfaces on the server from semantic HTML and themeable CSS. Every control must work through ordinary links and GET or POST forms without JavaScript; client behaviour is an optional enhancement loaded per component. Components stay independent of HTTP and storage, and a theme whose text or UI role pairs fail WCAG contrast does not build.

## Colors

Components use semantic roles only. Each role resolves to a step of a twelve-step scale generated in OKLCH from a preset's hue and chroma, so a role keeps its meaning across all fourteen presets and both schemes.

- **Backgrounds** layer from `bg` (the page) to `surface` (cards, panels, inputs, the app header and sidebar) to `surface-raised` (table headers, tags, input adornments, overlays). `hover` is the neutral fill for hovered secondary, ghost and navigation controls.
- **Text** is `text`, or `text-muted` for labels, hints, captions, result counts and footers.
- **Borders** use `border` for structure (cards, tables, separators) and `border-strong` for interactive edges (inputs, secondary buttons, hovered pagination).
- **Accent** marks the primary action and the current item. `accent` fills primary buttons and the current page link, with `on-accent` text; `accent-text` colours links; `accent-soft` backs the current sidebar link; `focus` draws focus rings.
- **Status** roles come in three strengths: the solid colour for borders and invalid-field outlines, `-text` for text on the page or on the soft fill, and `-soft` for tinted fills.

## Themes

Tokens above are the light scheme. Pages follow the operating system by default; `data-theme` forces a scheme on a subtree, and `data-st-theme` applies a named theme. Beyond the presets, `Theme::seeded` derives a complete theme, including its style personality, from one number; a given seed always yields the same theme and always passes the contrast checks. Every option is seedable on its own and matches the full theme's choice for the same seed, so presets can borrow individual options from a seed. Dark values for the default preset:

| Role | Dark |
| --- | --- |
| bg | oklch(16.00% 0.0012 250.00) |
| surface | oklch(19.00% 0.0018 250.00) |
| surface-raised | oklch(23.00% 0.0030 250.00) |
| hover | oklch(26.00% 0.0042 250.00) |
| text | oklch(95.00% 0.0054 250.00) |
| text-muted | oklch(80.00% 0.0102 250.00) |
| border | oklch(34.00% 0.0066 250.00) |
| border-strong | oklch(60.00% 0.0120 250.00) |
| accent | oklch(60.00% 0.1500 250.00) |
| accent-hover | oklch(66.00% 0.1500 250.00) |
| accent-text | oklch(80.00% 0.1049 250.00) |
| accent-soft | oklch(23.00% 0.0375 250.00) |
| on-accent | oklch(0.00% 0.0000 0.00) |
| focus | oklch(60.00% 0.1500 250.00) |
| success | oklch(60.00% 0.1400 150.00) |
| success-text | oklch(80.00% 0.1190 150.00) |
| success-soft | oklch(23.00% 0.0350 150.00) |
| warning | oklch(60.00% 0.1230 85.00) |
| warning-text | oklch(80.00% 0.1190 85.00) |
| warning-soft | oklch(23.00% 0.0350 85.00) |
| danger | oklch(60.00% 0.1400 25.00) |
| danger-text | oklch(80.00% 0.1143 25.00) |
| danger-soft | oklch(23.00% 0.0350 25.00) |
| info | oklch(60.00% 0.1375 240.00) |
| info-text | oklch(80.00% 0.1128 240.00) |
| info-soft | oklch(23.00% 0.0350 240.00) |

## Typography

Text uses the system sans stack, and code and keys use the system mono stack; serif presets replace the sans family. Sizes come from a modular scale around the base step and are fluid: each step shrinks slightly on small viewports and reaches its listed size on wide ones. Headings choose level and size independently, so a heading's visual size follows its place on the page rather than its outline depth: panel titles use `lg`, and page titles in the app header keep the level default. Labels, hints, table headers and tags use `sm`.

## Layout

Compose pages from the layout primitives (Stack, Cluster, Grid, Center, Container, Sidebar, Switcher) and their spacing steps instead of margins on components; components do not reset the margins those primitives give them. Grids and switchers adapt by intrinsic minimum widths, and component-level changes use container queries, so layouts respond to the space they get rather than to device breakpoints. Container and app-shell gutters scale with the viewport between `spacing.4` and `spacing.8`. The app shell keeps a fixed-width sidebar beside the main column and stacks it above the content when the main column would become the narrower share. Use logical properties for every direction-dependent value.

Inputs, selects and buttons share one control height, which compact density reduces. Tables scroll inside their own region; cells do not wrap, numbers align to the end with tabular figures, and dates use tabular figures.

## Elevation & Depth

Borders carry structure; shadows mark elevation. How much elevation surfaces get is part of a theme's personality: flat themes use borders only, outlined themes (the default) add a light shadow to cards, and raised themes also lift panels, filter bars and table cards. Components read `--st-card-shadow` and `--st-surface-shadow` instead of naming a shadow step, and overlays use the second step. Shadows are tinted from the neutral scale so they follow the theme.

## Shapes

Presets choose one of three radius sets: soft (the default), round or sharp. Controls and pagination use `md`, buttons use `md` or `full` depending on the theme's button shape, and underlined inputs round only their top corners; cards, panels, filter bars and table cards use `lg`; tags use `full`.

## Components

Theme personality sets table rules and stripes, input fill and borders, header fill, heading weight, button radius, link underlines, the current-page marker, the focus ring, the page's plaster finish, the app shell's layout, how panels are framed, the shape of large corners, line weight, button depth, small labels, icon stroke, motion, the heading typeface, leading, tags and dividing rules through tokens (`--st-table-rule`, `--st-control-bg`, `--st-header-bg`, `--st-heading-weight`, `--st-button-radius`, `--st-link-line`, `--st-nav-current-bg`, `--st-focus-width`, `--st-finish`, `--st-shell-columns`, `--st-panel-border`, `--st-surface-radius`, `--st-corner-shape`, `--st-line`, `--st-button-shadow`, `--st-label-case`, `--st-icon-stroke`, `--st-ease`, `--st-font-heading`, `--st-leading`, `--st-tag-radius`, `--st-rule-style` and their siblings). Every link style keeps an underline, and a finish is a faint grey grain that text on the page must clear at its strongest point. Component CSS reads those tokens rather than fixed values, so one markup serves every personality.

- **Buttons** come in primary, secondary, ghost and danger variants at three sizes. Use one primary action per group, placed at the trailing end. Pressed buttons shrink slightly as press feedback. Loading buttons stay focusable and announce busy.
- **Forms** wire every control to a visible label, hint and error. Invalid fields get a `danger` border and ring that never shifts layout, and an error summary above the form lists each error with a link to its field and takes focus after a failed submission.
- **Collections** put filters in a surface card with labels above controls, Clear before Apply at the trailing end, and range filters as two inputs that stack with visible labels in narrow containers. The table sits in its own card with a muted header row and sort arrows; the result count and pagination share a footer row.
- **Tags** show enumeration values as neutral pills carrying `data-value`, so applications can tone individual values.
- **Navigation** links in the sidebar are full-width rows with a `hover` fill; the current page uses `accent-soft` with `accent-text`.

## Do's and Don'ts

- Do reference semantic role tokens in component CSS.
- Don't write colour literals (hex, `rgb()`, `hsl()`, `oklch()`) or palette steps such as `--st-neutral-6` in component CSS; the component CSS check rejects them.
- Don't put component CSS outside the `stucco.*` cascade layers.
- Don't set a component's reserved attributes through passthrough attributes; debug builds panic and release builds ignore them.
