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
