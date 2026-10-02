# stucco-redb

Typed embedded storage for stucco using redb 4.3. Key codecs define byte ordering;
value codecs serialize owned records. Indexed writes update records and one
secondary index in the same transaction.

```rust,no_run
use stucco_redb::{Store, U64Key, PostcardCodec};
let store = Store::open("orders.redb")?;
let table = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
let indexed = table.index("customer", |name| name.as_bytes().to_vec())?;
indexed.rebuild()?; // Needed when adding an index to an existing table.
indexed.put(&1, &"Ada".into())?;
assert_eq!(indexed.get(&1)?, Some("Ada".into()));
# Ok::<(), stucco_redb::StoreError>(())
```

Write through the indexed handle after creating an index; a previously cloned
unindexed Table does not maintain secondary indexes. Codec/schema changes
require application-managed migration.

The default `tokio` feature adds `RedbCollection` and `CollectionMapping`,
moving synchronous scans onto a bounded blocking pool. Share source clones or
`with_mapping` views across requests so they share the concurrency limit.
Disable default features for synchronous storage alone.

Cursor scans return an owned page with an unknown total. Predicates may still
visit many records. Numbered mode deliberately materializes all matching records,
so use it for small collections. Cursors are locators, not authorization tokens.

Minimum Rust: **1.90**. The 0.2 API is experimental.
See [the orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders)
for persistence, query mapping, and server-rendered controls.

Licensed under MIT or Apache-2.0.
