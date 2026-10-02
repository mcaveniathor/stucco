# Changelog

## Unreleased

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
