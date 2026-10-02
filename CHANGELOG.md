# Changelog

## Unreleased

- `FormState::from_urlencoded` parses a form body (dropping `_csrf`), and
  `FormState::field_errors` lists fields with errors.
- `Validator` checks a submitted `FormState` and builds typed values, or returns
  the state with one message per invalid field.
- `ErrorSummary` lists a failed submission's errors above the form, links them
  to their controls, and takes focus on load.
- Collections look finished by default: the filter bar is a card built from
  the standard inputs, selects and buttons, with all controls one height; range
  filters sit side by side and stack with visible labels in narrow containers;
  the table is a card with a muted header row, sort indicators, end-aligned
  tabular numbers, and enumeration values shown as tags (`.st-tag[data-value]`);
  the result count and pagination share a footer row. Filter and sort labels
  come from column labels (`FilterBar::column_label`), and the reset link now
  reads "Clear" in the bar and "Clear filters" in the empty state.
- `Col::display` formats a column's cells without changing how it sorts or
  filters; `Row::cell_with_attrs` sets attributes on a data cell.
- The application shell has a fixed-width sidebar that stacks on narrow
  screens, styled navigation links (with `aria-current="page"`), a surface
  header and a quieter footer. `PageHeader` wraps its description in
  `.st-page-header-description`.
- Pagination links are bordered buttons with hover and current states.
- Text inputs and selects share `--st-control-h` as a fixed height.
- `ResultCount` says "Showing 1 result" instead of "Showing 1 results".
- Fieldsets and radio groups no longer reset their margins, so layout
  primitives space them like any other child.
- Pages emit `theme-color` meta tags for the light and dark background.
- Buttons scale to 0.96 while pressed; danger buttons keep their tint on
  hover; controls set `touch-action: manipulation`; icons read their stroke
  width from `--st-icon-stroke` (default 2).
- Panel titles render at the `lg` size; `PageHeader::size` and
  `SectionHeader::size` set a heading's size independently of its level;
  keyboard keys in `Kbd` are spaced.
- `DESIGN.md` records the design language and default tokens.
- `stucco-tower`: the `Submission` axum extractor (415 for other content types)
  and the `SeeOther` 303 redirect for post-redirect-get.
- docs.rs now labels items that require a cargo feature.
- Document that the reserved `overlay` feature is enabled by default and adds nothing yet.
- CI checks each cargo feature on its own and runs `cargo-semver-checks` against
  the published release.

## 0.1.0 — 2026-10-02

Initial release:

- OKLCH themes, semantic color tokens, presets, and contrast validation.
- Escaped HTML rendering, checked URLs, page/fragment output, and asset bundles.
- Layout, typography, actions, forms, feedback, navigation, data, and app components.
- Typed collection columns with native GET search, filters, sorting, and pagination.
- Tower/Axum services, responses, request context, and collection sources.
- redb 4.3 storage with transactional indexes and bounded blocking collection scans.
- Hello, persistent orders, and static gallery examples.

Core, components, and Tower require Rust 1.85. The redb adapter requires Rust 1.90.
The API is experimental; later 0.x releases may introduce breaking changes.
