CREATE TABLE order_attachments (
    order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    id INTEGER NOT NULL CHECK (id > 0),
    file_name TEXT NOT NULL,
    bytes BLOB NOT NULL CHECK (length(bytes) BETWEEN 1 AND 1048576),
    PRIMARY KEY (order_id, id)
) STRICT;
CREATE TABLE export_jobs (key TEXT PRIMARY KEY, record TEXT NOT NULL) STRICT;
CREATE TABLE export_results (key TEXT PRIMARY KEY REFERENCES export_jobs(key) ON DELETE CASCADE, data BLOB NOT NULL) STRICT;
