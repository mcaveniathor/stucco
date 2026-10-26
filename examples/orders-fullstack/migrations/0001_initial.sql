CREATE TABLE tenants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id INTEGER NOT NULL REFERENCES tenants(id),
    username TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    session_epoch INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1))
) STRICT;

CREATE TABLE orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id INTEGER NOT NULL REFERENCES tenants(id),
    revision INTEGER NOT NULL CHECK (revision > 0),
    customer TEXT NOT NULL,
    customer_search TEXT NOT NULL
) STRICT;

CREATE TABLE order_lines (
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL CHECK (slot IN (0, 1)),
    catalog_id INTEGER NOT NULL CHECK (catalog_id > 0),
    name TEXT NOT NULL,
    unit_price_cents INTEGER NOT NULL CHECK (unit_price_cents >= 0),
    quantity INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 999),
    PRIMARY KEY (order_id, slot)
) STRICT;
