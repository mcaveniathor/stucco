//! Typed redb storage.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
mod codec;
mod error;
mod index;
mod store;
mod table;
pub use codec::{Codec, KeyCodec, PostcardCodec, U64Key, Utf8Key};
pub use error::StoreError;
pub use index::IndexTable;
pub use store::Store;
pub use table::Table;
