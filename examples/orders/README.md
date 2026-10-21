# Orders

Run `cargo run -p orders`, then open http://localhost:4181/orders.
Override PORT or ORDERS_DB (default target/orders.redb) through the environment.
Set `STUCCO_SECRET_KEY` (64 hex digits) for the CSRF and flash cookies; debug
builds generate one with a warning. `ORDERS_ORIGIN` is the public origin
(default `http://localhost:$PORT`); cookies are `Secure` when it is HTTPS.

The app seeds 67 deterministic records when it creates a new database; one
emptied by deleting every order stays empty.
Customer names have duplicate values; the index uses the primary key as a tie
breaker. Startup rebuilds the customer index, preserving persisted records.
Every page works without JavaScript: lists, filters and pagination are
ordinary GET requests, and writes are ordinary POST forms. Unknown or
malformed order addresses get an HTML not-found page.
Both pagination modes and every write share one `AsyncTable` budget of four
blocking calls.

Default mode scans in primary-key or customer-index order with exclusive,
query-scoped cursors and an unknown total. Filtering may traverse many records
to find a page. Numbered mode at /orders?mode=pages deliberately reads all
matching records and counts them before slicing: it is for small collections,
not a scalable counted-query engine.

Amounts are stored in integer cents. The Total column displays and filters
in dollars; `Col::display` formats the value without changing how it sorts or
filters.

Orders can be created (`/orders/new`), edited (`/orders/{id}/edit`) and
deleted (`/orders/{id}/delete`, or the row's Actions menu). Every form posts
with a CSRF token and redirects with a flash message. Edits and deletes carry
the record's revision: if someone else changed the order meanwhile, the save
is refused with 409 and a conflict notice, keeping what you typed. No
authentication or authorization is included.
Opaque cursors are locators, not authorization tokens.

Core/components/Tower retain Rust 1.85. This example and stucco-redb require
Rust 1.90 because they use redb 4.3.
