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
//! tokio feature, CollectionMapping defines predicates/order and RedbCollection
//! moves scans onto a bounded blocking pool. Share clones or with_mapping views
//! across requests; creating a fresh source per request creates a fresh budget.
//! Cursors are query-scoped locators, not authorization tokens. The table and
//! indexes are not automatically migrated when changing codecs/schema.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
mod codec;
#[cfg(feature = "tokio")]
mod collection;
mod error;
mod index;
mod scan;
mod store;
mod table;
pub use codec::{Codec, KeyCodec, PostcardCodec, U64Key, Utf8Key};
#[cfg(feature = "tokio")]
pub use collection::{CollectionMapping, RedbCollection};
pub use error::StoreError;
pub use index::IndexTable;
pub use scan::ScanRequest;
pub use store::Store;
pub use table::Table;
