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
require explicit migrations; the optional `migrations` feature coordinates them
at startup while application-specific codecs and transforms stay in the app.

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

## Single-submit forms

With the `submit-ledger` feature, `SubmitLedger` records a keyed outcome in the
transaction that applies a form's effect, so a double tap takes effect once and
the repeat can answer as the first did. Pair it with `SingleSubmit` from
`stucco-tower`, which issues the keys; the
[forms guide](https://github.com/mcaveniathor/stucco/blob/main/docs/guide/forms.md)
shows the whole flow. Run `purge_expired` on a timer.

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
  error with `?` to roll everything back. A codec error does the same. A write
  that fails after redb has started applying it (a value over redb's size
  limit, say) makes the transaction rollback-only: further calls error, and
  `write` rolls back even if your closure handled the error and returned `Ok`.
- The closure may return your own error type, which only needs
  `From<StoreError>`. A closure that only ever returns `Ok` needs
  `Ok::<_, StoreError>(..)` so the compiler can choose it.
- Do not call `Store::write` or a single-table write from inside the closure
  on the same thread: redb allows one writer, so it would wait forever.
- `ReadTx::index_range` (and the same on `WriteTx`) reads up to a limit of records whose indexed value falls
  in a byte range, in either direction, visiting only the records it returns.
  Project a fixed-width scope first (`tenant.to_be_bytes()`, then a sort value)
  and range over the scope to read one tenant without filtering every row.
- `AsyncStore::new(store)` (or `AsyncTable::store()`) is the async form. A
  transaction runs whole on the blocking pool under one permit, so the write
  lock is never held across an `.await`, and the permits are shared with every
  `AsyncTable` made from it. Wrap the future in `tokio::time::timeout` for a
  deadline; a call dropped while waiting for a permit runs nothing.

## Opt-in startup migrations

Enable `migrations` to use `Migrations` and `Migration`. Open typed handles first,
then run the complete contiguous plan (versions 1, 2, …) **before serving traffic**.
Each migration has an immutable author-supplied identity/revision or checksum.
The runner refuses a future stored version or changed historical identity and
runs all pending transforms plus history in one writer transaction. Failure or
panic rolls back the entire pending batch; reopening does not repeat applied work.

The runner does not infer codecs, rebuild indexes or compute source checksums.
Keep legacy types/decoders in the application, preserve index consistency and
change the identity whenever editing an unapplied transform. Never change an
applied transform; append a new version. Migrating an existing application that
already has a schema marker requires a first bridge step that checks that marker
(including refusal of newer legacy versions), transforms/backfills as needed,
and updates it inside the supplied transaction. Do not blindly stamp old data as
current. Keep fixture/rollback tests for that bridge.

Callbacks must use only their supplied `WriteTx` and already-opened handles.
Opening a table or starting a nested write inside a callback deadlocks on the
single writer. No filesystem/network I/O in callbacks. Startup migration work
holds the writer lock and may be expensive; this runner does not promise automatic
batching or bounded memory for application transforms.

## Opt-in durable jobs

Enable `jobs` to use `JobQueue<V>` with dedicated `name` and `name_due` tables.
Enqueue inside the same `WriteTx` as the application change. A caller-chosen
permanent key deduplicates forever, including terminal Done/Failed history;
duplicate enqueue does not replace payload, schedule or attempt budget. Scope
keys by tenant and operation, and do not reuse them for new work.

`claim(tx, now, lease_seconds)` returns one due job or expired attempt. It examines
at most 64 due entries, retiring exhausted attempts; `None` can therefore mean
maintenance made progress. Poll again later with a bounded runtime. Claims count
against the nonzero attempt budget, including workers that crash. Only a live
matching lease can `acknowledge` or `retry`; leases expire exactly at their
deadline. Acknowledgement and app metadata effects belong in the same transaction;
**do not commit the app effect when acknowledgement returns false**. Propagate
command errors with `?` rather than trying to commit a partial workflow.

`retry` takes a caller-selected future schedule (no hidden backoff policy). At
budget exhaustion it retains Failed for inspection; recovery with a fresh job
identity must account for possible repeated external effects. Successful jobs
retain Done. Terminal jobs are absent from the due index, so historical growth
does not slow due lookup; archival/retention is deliberately not automatic.

No worker process, scheduler, clock, authentication, cancellation or lease-renewal
runtime is included. Use the existing AsyncStore blocking budget or run sync work
off the async executor. Choose leases for bounded work; get a fresh attempt after
expiry. Timestamps are supplied by a trusted server clock, not client input.
Payloads and indexes are persisted schemas: version application payloads before
changing them. Lease values bind to their queue and open database handle; they
are not portable authorization credentials.

**At least once, not exactly once:** do blob/network I/O outside transactions.
Lease tokens fence database metadata, not a remote write already in flight.
External effects must be idempotent and reconciled after crash/late completion.
Never release quota or delete audit data merely because a job lease expired.

See the runnable [durable work example](examples/durable_work.rs):
`cargo run -p stucco-redb --features migrations,jobs --example durable_work`.
The [durability guide](../../docs/guide/durable-work.md) explains integration and
legacy migration boundaries. Neither feature expands facade defaults.

Minimum Rust: **1.90**. The API is experimental.
See [the orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders)
for persistence, query mapping, and server-rendered controls.

Licensed under MIT or Apache-2.0.

### Bounded pages within one indexed value

`ReadTx::index_equal_after` and `WriteTx::index_equal_after` read at most `limit`
rows from one exact secondary-index value, using an optional exclusive primary-key
cursor. They seek directly into the existing compound index; no preceding rows or
other index values are materialized. Ascending order returns keys greater than the
cursor; descending order returns keys less than it. A deleted/nonexistent cursor
remains a valid position. Zero limit decodes no records. Order follows the key
codec's encoded order, not insertion order. Write transactions see their own writes.

For a household-only index, pass the encoded household value, an optional last
primary ID and `limit + 1` to implement a bounded next-page indicator. Authorization
can run in the same transaction; cursor positions are not authorization. No new
index format, migration or feature flag is required.
