//! Typed redb storage.
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
