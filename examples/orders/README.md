# Orders

A complete, server-rendered admin for one resource. Run `cargo run -p orders`,
then open http://localhost:4181/orders. Override PORT or ORDERS_DB (default
target/orders.redb) through the environment.

## What it does

Every workflow works without JavaScript, through ordinary links, GET forms
and POST forms:

- **Browse**: search, filter by status, total and date, sort, and page
  through orders, with each active filter removable on its own. Archived
  orders leave the list; filter by the Archived status to see them.
- **View**: an order's page shows its status, properties and history, with
  breadcrumbs back to the exact list it was opened from (search, filters and
  page kept).
- **Create and edit**: an invalid submission comes back with status 422,
  everything typed kept (including text that didn't parse) and each problem
  linked from an error summary. A save redirects (303) to the order with a
  one-time "Order 68 was created." notice. An edit based on an old copy of
  the order is refused with status 409 and the user's changes kept.
- **Archive, restore and delete**: each asks first on a page of its own.
  Archiving is reversible and hides the order; deleting is permanent and
  says so.
- **Bulk actions**: tick orders in the list, choose Archive or Delete, and a
  confirmation names the action and how many orders it affects. Each order
  is checked again when the action runs, and the outcome says if some had
  already changed.

`tests/http.rs` drives each of these through the router, and
`browser/tests/orders.spec.ts` runs the whole create-to-delete flow in real
browsers with JavaScript turned off.

## What it leaves to a real application

- **Sign-in and permissions.** Every change is recorded as "Demo user", and
  anyone can do anything. A real app checks, in every handler, that the
  signed-in user may read or change *this* order: hiding a link is not a
  check. Opaque cursors are locators, not authorization tokens.
- **CSRF.** The example refuses cross-site posts by checking the
  `Sec-Fetch-Site` and `Origin` headers (`same_origin` in `src/lib.rs`).
  Use your framework's CSRF defence or an established library in a real
  app; stucco renders a token for you with `Form::csrf`.
- **Migrations.** Records live in the `orders-v2` table. A schema change
  here means a new table name; a real app needs migrations.
- **Writes at scale.** Writes take one process-wide lock, so a read, check
  and write (the version check, the next id) can't interleave. New ids come
  from scanning every key. Both are fine for a demo and wrong for a busy
  service.
- **Time zones.** Times are stored and shown in UTC, and say so.

## Storage costs

The app seeds 67 deterministic records only when the orders table is empty.
Customer names have duplicate values; the index uses the primary key as a tie
breaker. Startup rebuilds the customer index, preserving persisted records.
Both pagination modes share one application-wide budget of four blocking
scans.

Default mode scans in primary-key or customer-index order with exclusive,
query-scoped cursors and an unknown total. Filtering may traverse many records
to find a page. Numbered mode at /orders?mode=pages deliberately reads all
matching records and counts them before slicing: it is for small collections,
not a scalable counted-query engine.

Amounts are stored in integer cents. The Total column displays and filters
in dollars; `Col::display` formats the value without changing how it sorts or
filters.

Core/components/Tower retain Rust 1.85. This example and stucco-redb require
Rust 1.90 because they use redb 4.3.
