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

The default `tokio` feature adds `AsyncTable`, `RedbCollection` and
`CollectionMapping`. `AsyncTable` runs reads and writes on Tokio's blocking
pool, at most four at a time (`concurrency` changes it); a `RedbCollection`
built on it shares that limit for its scans. Share clones or `with_mapping`
views across requests. Disable default features for synchronous storage alone.

## Writing

Edits that might race use revisions: a hash of the record's stored bytes.
Read the record with its revision, put the revision in the edit form as a
hidden input, and pass it back with the change:

```rust,no_run
# async fn edit(orders: stucco_redb::AsyncTable<u64, String>) -> Result<(), stucco_redb::StoreError> {
let (_name, revision) = orders.get_with_revision(1).await?.expect("exists");
// …render a form carrying `revision.to_string()`, then on submit:
let submitted: stucco_redb::Revision = revision.to_string().parse().expect("valid");
match orders.update_if(1, submitted, |_| "Grace".into()).await {
    Ok(_new_revision) => {}                    // saved
    Err(e) if e.is_conflict() => {}            // changed meanwhile: show a conflict
    Err(e) => return Err(e),
}
# Ok(()) }
```

`conflict_revision()` gives the current revision, or `Some(None)` when the
record was deleted meanwhile. `remove_if` checks the same way. For rows from a
scan, `revision_of(&row)` computes the revision without reading again. It
re-encodes the row, so after changing a codec or schema run `rebuild()` once
at startup: it rewrites old records in the new encoding, and their revisions
then match.

`create(|id| value)` on a `u64` table allocates one more than the highest id
in the same transaction, so concurrent creates never collide. Deleting the
highest record frees its id for the next create; a form still holding the
deleted record's revision gets a conflict, not the new record, unless the new
record's value is byte-for-byte the same (then the edit was based on exactly
the data stored, and it goes ahead).

Cursor scans return an owned page with an unknown total. Predicates may still
visit many records. Numbered mode deliberately materializes all matching records,
so use it for small collections. Cursors are locators, not authorization tokens.

## Transactions

`Store::write` runs a closure against one redb write transaction that can touch
any number of tables and indexes. It commits when the closure returns `Ok`;
an `Err` or a panic writes nothing. `Store::read` gives one consistent snapshot
across tables.

```rust,no_run
use stucco_redb::{PostcardCodec, Revision, Store, StoreError, U64Key};
#[derive(serde::Serialize, serde::Deserialize)]
struct Account { orders: u32 }

let store = Store::open("shop.redb")?;
let orders = store
    .table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?
    .index("customer", |name| name.as_bytes().to_vec())?;
let audit = store.table::<u64, String, _, _>("audit", U64Key, PostcardCodec::default())?;
let accounts = store.table::<u64, Account, _, _>("accounts", U64Key, PostcardCodec::default())?;

// The account's revision came back from the form that is being submitted.
let submitted: Revision = "AAAAAAAAAAAAAAAAAAAAAA".parse().expect("valid");
let saved = store.write(|tx| {
    let (id, _) = tx.create(&orders, |id| format!("customer of order {id}"))?;
    tx.put(&audit, &id, &"created".to_string())?;
    tx.update_if(&accounts, &7, &submitted, |account| Account { orders: account.orders + 1 })?;
    Ok::<_, StoreError>(id)
});
match saved {
    Ok(_id) => {}                              // all three tables changed
    Err(e) if e.is_conflict() => {}            // account changed or missing: nothing was written
    Err(e) => return Err(e),
}
# Ok::<(), StoreError>(())
```

- `WriteTx` has `get`, `get_with_revision`, `put`, `remove`, `update_if`,
  `remove_if` and (on `u64` tables) `create`, for any `Table` or `IndexTable`.
  An `IndexTable` keeps its index in step; a plain `Table` writes the record
  only, as before.
- A failed `update_if` or `remove_if` writes nothing and leaves the
  transaction usable: match on `is_conflict()` and carry on, or return the
  error with `?` to roll everything back.
- The closure may return your own error type, which only needs
  `From<StoreError>`. A closure that only ever returns `Ok` needs
  `Ok::<_, StoreError>(..)` so the compiler can choose it.
- Do not call `Store::write` or a single-table write from inside the closure
  on the same thread: redb allows one writer, so it would wait forever.
- `AsyncStore::new(store)` (or `AsyncTable::store()`) is the async form. A
  transaction runs whole on the blocking pool under one permit, so the write
  lock is never held across an `.await`, and the permits are shared with every
  `AsyncTable` made from it. Wrap the future in `tokio::time::timeout` for a
  deadline; a call dropped while waiting for a permit runs nothing.

Minimum Rust: **1.90**. The API is experimental.
See [the orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders)
for persistence, query mapping, and server-rendered controls.

Licensed under MIT or Apache-2.0.
