//! Private blob storage: the same small interface over a directory, memory, or
//! (with the `s3` feature) any S3-compatible service.
//!
//! An application keeps its metadata and access rules in its own records and its
//! bytes here, under keys it chooses. Every backend promises the same things,
//! and the shared contract test holds each one to them:
//!
//! - A reader sees all of a blob or none of it, never a partial write.
//! - `put` replaces. `delete` succeeds whether or not the blob exists, so a
//!   cleanup job can repeat it after a crash.
//! - `list` returns keys in byte order, so a job can page through them and
//!   compare them with its records to find orphans.
//!
//! Nothing here is publicly readable. Serve blobs from a handler that checks
//! authorization on every request. There are deliberately no signed URLs: one
//! can remain authorized until it expires despite changed application access rules.
//!
//! ```
//! use stucco_blob::{BlobKey, BlobStore, FilesystemStore};
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), stucco_blob::BlobError> {
//! # let mut builder = tempfile::Builder::new();
//! # #[cfg(unix)] {
//! # use std::os::unix::fs::PermissionsExt;
//! # builder.permissions(std::fs::Permissions::from_mode(0o700));
//! # }
//! # let dir = builder.tempdir().unwrap();
//! let store = FilesystemStore::open(dir.path())?;
//! while store.recover_staging(100).await? != 0 {}
//! let key = BlobKey::new("tenants/7/files/12/original")?;
//! store.put_bounded(&key, "hello".into(), 1024).await?;
//! assert_eq!(store.get_bounded(&key, 1024).await?.as_deref(), Some(b"hello".as_slice()));
//! store.delete(&key).await?;
//! store.delete(&key).await?; // Deleting again is not an error.
//! # Ok(()) }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod filesystem;
mod key;
mod memory;
#[cfg(feature = "s3")]
mod s3;

pub use bytes::Bytes;
pub use filesystem::FilesystemStore;
pub use key::{BlobKey, InvalidKey};
pub use memory::MemoryStore;
#[cfg(feature = "s3")]
pub use s3::{S3Config, S3Store};

use std::{fmt, future::Future, pin::Pin};

/// The future every [`BlobStore`] method returns.
pub type BlobFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, BlobError>> + Send + 'a>>;

/// Private storage for whole blobs. Implement it for another backend, and run
/// it through the same checks as `tests/contract.rs`.
pub trait BlobStore: Send + Sync + 'static {
    /// Stores `bytes` under `key`, replacing any earlier blob.
    fn put<'a>(&'a self, key: &'a BlobKey, bytes: Bytes) -> BlobFuture<'a, ()>;
    /// The blob under `key`, if there is one.
    fn get<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, Option<Bytes>> {
        self.get_bounded(key, usize::MAX)
    }
    /// Reads at most `max_bytes`, refusing an oversized object before allocating
    /// its declared size. Implementations also enforce the limit during reads.
    /// Applications should prefer this over unbounded `get` for uploaded data.
    fn get_bounded<'a>(
        &'a self,
        key: &'a BlobKey,
        max_bytes: usize,
    ) -> BlobFuture<'a, Option<Bytes>>;
    /// Refuses oversized bytes before any write. This cannot undo allocation of
    /// the supplied buffer: enforce HTTP/body limits before collecting uploads.
    fn put_bounded<'a>(
        &'a self,
        key: &'a BlobKey,
        bytes: Bytes,
        max_bytes: usize,
    ) -> BlobFuture<'a, ()> {
        Box::pin(async move {
            if bytes.len() > max_bytes {
                return Err(BlobError::new("blob exceeds byte limit"));
            }
            self.put(key, bytes).await
        })
    }
    /// Removes the blob under `key`. A missing blob is not an error.
    fn delete<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, ()>;
    /// Up to `limit` keys starting with `prefix` and sorting after `after`, in
    /// byte order. A prefix matches whole path segments: `a/b` finds `a/b/c`
    /// but not `a/bc`.
    fn list<'a>(
        &'a self,
        prefix: &'a BlobKey,
        after: Option<&'a BlobKey>,
        limit: usize,
    ) -> BlobFuture<'a, Vec<BlobKey>>;
}

/// A storage failure. Messages name the operation and never carry credentials.
#[derive(Debug)]
pub struct BlobError(Box<dyn std::error::Error + Send + Sync>);

impl BlobError {
    /// Wraps a backend's error.
    pub fn new(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self(error.into())
    }
}

impl fmt::Display for BlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "blob storage failed: {}", self.0)
    }
}

impl std::error::Error for BlobError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

impl From<std::io::Error> for BlobError {
    fn from(error: std::io::Error) -> Self {
        Self::new(error)
    }
}

impl From<InvalidKey> for BlobError {
    fn from(error: InvalidKey) -> Self {
        Self::new(error)
    }
}

// Keep the retained payload allocation within the explicit byte budget, including
// growth after a stale metadata size. Transport/stack buffers are separate.
pub(crate) fn append_bounded(
    target: &mut Vec<u8>,
    bytes: &[u8],
    max: usize,
) -> Result<(), BlobError> {
    if bytes.len() > max.saturating_sub(target.len()) {
        return Err(BlobError::new("blob exceeds byte limit"));
    }
    target
        .try_reserve_exact(bytes.len())
        .map_err(BlobError::new)?;
    target.extend_from_slice(bytes);
    Ok(())
}
