# Orders

Run `cargo run -p orders`, then open http://localhost:4181/orders.
Override PORT or ORDERS_DB (default target/orders.redb) through the environment.

The app seeds 67 deterministic records only when the orders table is empty.
Customer names have duplicate values; the index uses the primary key as a tie
breaker. Startup rebuilds the customer index, preserving persisted records.
All controls work through ordinary GET requests without JavaScript.
Both pagination modes share one application-wide budget of four blocking scans.

Default mode scans in primary-key or customer-index order with exclusive,
query-scoped cursors and an unknown total. Filtering may traverse many records
to find a page. Numbered mode at /orders?mode=pages deliberately reads all
matching records and counts them before slicing: it is for small collections,
not a scalable counted-query engine.

Amounts are stored and filtered in integer cents; the table labels that unit
explicitly. No mutation, authentication, or authorization endpoints are included.
Opaque cursors are locators, not authorization tokens.

Core/components/Tower retain Rust 1.85. This example and stucco-redb require
Rust 1.90 because they use redb 4.3.
