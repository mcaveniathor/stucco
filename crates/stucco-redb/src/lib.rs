//! Typed redb storage.
//!
//! ```no_run
//! use stucco_redb::{Store, U64Key, PostcardCodec, ScanRequest};
//! use stucco_core::{Direction, Window};
//! let store = Store::open("orders.redb")?;
//! let table = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
//! let indexed = table.index("customer", |name| name.as_bytes().to_vec())?;
//! indexed.rebuild()?; // Required when opening an index on existing rows.
//! indexed.put_many(&[(1, "Ada".into()), (2, "Ada".into())])?;
//! let page = indexed.scan(&ScanRequest { index: Some("customer".into()),
//!     direction: Direction::Asc, window: Window::default(), per_page: 25,
//!     scope: "all".into(), offset_mode: false }, |_| true)?;
//! assert_eq!(page.rows.len(), 2);
//! # Ok::<(), stucco_redb::StoreError>(())
//! ```
//!
//! Use only the indexed handle for writes once an index exists. Custom codecs
//! implement Codec; key codecs additionally implement KeyCodec and must preserve
//! the intended byte ordering. U64Key uses big endian bytes; Utf8Key uses UTF-8
//! lexicographic ordering. PostcardCodec serializes owned values.
//!
//! Cursor scans hold one read snapshot, decode owned values and materialize a
//! page plus one matching record. Predicates can still visit every record.
//! Offset mode explicitly materializes all matching rows. With the optional
//! tokio feature, AsyncTable runs reads and writes on a bounded blocking pool,
//! CollectionMapping defines predicates/order, and RedbCollection scans through
//! the AsyncTable's budget. Share clones or with_mapping views across requests;
//! creating a fresh AsyncTable per request creates a fresh budget.
//!
//! Revisions (hashes of stored bytes) make edits conditional:
//! get_with_revision, update_if and remove_if return a conflict when the
//! record changed in between.
//!
//! Transactions span tables: `Store::write` runs a closure against a `WriteTx`
//! and commits when it returns `Ok`; an `Err` or a panic writes nothing.
//! `Store::read` gives one snapshot across tables. `WriteTx` offers `get`,
//! `get_with_revision`, `put`, `remove`, `update_if`, `remove_if` and `create`
//! for any `Table` or `IndexTable`; an `IndexTable` keeps its index in step.
//! With the tokio feature, `AsyncStore` runs the same closures on the blocking
//! pool, one permit per transaction, sharing its budget with `AsyncTable`.
//!
//! ```no_run
//! # fn main() -> Result<(), stucco_redb::StoreError> {
//! use stucco_redb::{PostcardCodec, Revision, Store, StoreError, U64Key};
//! #[derive(serde::Serialize, serde::Deserialize)]
//! struct Account { orders: u32 }
//!
//! let store = Store::open("shop.redb")?;
//! let orders = store
//!     .table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?
//!     .index("customer", |name| name.as_bytes().to_vec())?;
//! let audit = store.table::<u64, String, _, _>("audit", U64Key, PostcardCodec::default())?;
//! let accounts = store.table::<u64, Account, _, _>("accounts", U64Key, PostcardCodec::default())?;
//!
//! // The account's revision came back from the form that is being submitted.
//! let submitted: Revision = "AAAAAAAAAAAAAAAAAAAAAA".parse().expect("valid");
//! let saved = store.write(|tx| {
//!     let (id, _) = tx.create(&orders, |id| format!("customer of order {id}"))?;
//!     tx.put(&audit, &id, &"created".to_string())?;
//!     tx.update_if(&accounts, &7, &submitted, |account| Account { orders: account.orders + 1 })?;
//!     Ok::<_, StoreError>(id)
//! });
//! match saved {
//!     Ok(_id) => {}                              // all three tables changed
//!     Err(e) if e.is_conflict() => {}            // account changed or missing: nothing was written
//!     Err(e) => return Err(e),
//! }
//! # Ok(()) }
//! ```
//!
//! Cursors are query-scoped locators, not authorization tokens. The table and
//! indexes are not automatically migrated when changing codecs/schema.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#[cfg(feature = "tokio")]
mod async_store;
#[cfg(feature = "tokio")]
mod async_table;
mod codec;
#[cfg(feature = "tokio")]
mod collection;
mod error;
mod index;
mod revision;
mod scan;
mod store;
mod table;
mod transaction;
mod write;
#[cfg(feature = "tokio")]
pub use async_store::AsyncStore;
#[cfg(feature = "tokio")]
pub use async_table::AsyncTable;
pub use codec::{Codec, KeyCodec, PostcardCodec, U64Key, Utf8Key};
#[cfg(feature = "tokio")]
pub use collection::{CollectionMapping, RedbCollection};
pub use error::StoreError;
pub use index::IndexTable;
pub use revision::{InvalidRevision, Revision};
pub use scan::ScanRequest;
pub use store::Store;
pub use table::Table;
pub use transaction::{Creatable, Handle, ReadTx, WriteTx};
