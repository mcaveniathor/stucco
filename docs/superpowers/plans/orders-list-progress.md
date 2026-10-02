# Execution ledger — plan: 2026-10-01-stucco-orders-list.md

Base: 8f1aaf6. Native implementation in codex/orders-list.

Pre-flight: Tasks 1→2,5,7,8 share query/page/capability types; page size centralized.
Pre-flight: Tasks 3→4→5 share transaction and index ownership; writes through indexed handles.
Pre-flight: Tasks 6→7→8 share semantic components; no storage in rendering.
Pre-flight: Tasks 8→9 share deterministic seed/port/query labels.
Ruling: Record progress in this committed ledger instead of shell-only skill scripts — Windows-native tooling and durable handoff — cost: manual task bookkeeping.
Baseline passed (workspace tests). Task 1 complete: collection regression suite 4/4; Rust 1.85 core check passed. Tasks 2–9 pending.
Task 2 complete: source/context regression passed, all five Tower feature checks passed.
Task 3 complete: storage persistence/codec tests passed in default and synchronous modes; Rust 1.90 check passed.
Ruling: optional adapter Tower dependency uses a direct path declaration to disable defaults — Cargo rejects overriding inherited default features — cost: maintain its version alongside workspace declaration.
Task 4 complete: index atomicity, duplicate sorts, update/delete/rebuild and rollback tests passed; adapter Clippy passed.
Ruling: expose explicitly O(n) IndexTable::all for numbered mode/maintenance — needed to exercise index integrity and small-mode scans — cost: callers must heed documented scan cost.
Ruling: ScanRequest adds offset_mode bool — Window::Offset(page=1) also represents cursor first page, so window alone cannot select counted mode — cost: one additional configuration field.
Task 5 complete: all storage/scan/source tests passed; no-default tests, Rust 1.90 and adapter Clippy passed.
Task 6 complete: table/shell/count/feedback markup tests and all-feature CSS checks passed.
Task 7 complete: typed controls and DataTable markup, CSS, all-feature UI tests and Clippy passed.
Task 8 complete: persistent orders HTTP acceptance and stored-HTML escaping tests passed; example Clippy and Rust 1.90 checks passed.
Task 9 complete: gallery snapshots, documented compositions, CI storage MSRV/features and cache coverage, and three-browser acceptance checks passed.

Final review: two P1 findings fixed and correction-only inspection confirmed the fixes. Indexes now use a disjoint @index namespace; namespace collision regression reproduced deletion before the fix and passes afterward. Persistent cursor/pages sources share one four-scan budget through with_mapping; cancellation regression confirms a started blocking scan retains capacity after its awaiting request is aborted.
Final regressions: descending default sort survives links; invalid duplicate enum/number/date filters preserve prior valid values; duplicate customer keys traverse without omissions; primary-only Table::scan requires no index. Gallery pagination supports distinct names and panel/table regions have distinct labels.
Ruling: add RedbCollection::with_mapping — cursor and numbered sources must share the same application concurrency budget — cost: another adapter constructor-like API.
Ruling: index storage names use @index:table:index — public names disallow both separators, preventing collisions and destructive index rebuilds — cost: internal index names differ from the original plan; existing primary records remain intact, and the example rebuilds its index at startup.
Ruling: module-level executable recipes demonstrate related new component types together — compositions are more useful than isolated builder examples — cost: recipes live on module documentation.
Ruling: monetary cells explicitly display and filter integer cents — retains exact persisted units and avoids a custom column implicitly providing numeric filtering — cost: demonstration displays cents rather than currency formatting.
Ruling: WebKit builds on this platform skip links during Tab traversal; browser checks explicitly focus the skip link before keyboard activation there, and verify Tab order on Chromium/Firefox — cost: WebKit link-tab preference is not asserted.

Final validation:
- Formatting and workspace Clippy (all targets, warnings denied) passed.
- Workspace debug and release tests passed, including executable documentation.
- Facade 19 feature configurations, UI per-feature tests, Tower/redb per-feature builds and synchronous redb tests passed.
- Locked Rust 1.85 workspace check excluding storage/orders and locked Rust 1.90 storage/orders check passed.
- Playwright: 108/108 passed across Chromium, Firefox and WebKit, including JavaScript-disabled filtering, cursor/numbered navigation, light/dark axe checks and 320px layouts.
- Two pre-existing release-only dead-code warnings (LOOSE and UNSTYLED test assets) remain in stucco-core; no new release warnings.
- Independent review has no remaining actionable findings. Numbered mode remains O(n); schema migrations, authorization and mutations remain outside this read-only slice.
