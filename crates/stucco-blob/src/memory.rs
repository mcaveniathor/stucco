use crate::{BlobError, BlobFuture, BlobKey, BlobStore, Bytes};
use std::{
    collections::BTreeMap,
    ops::Bound,
    sync::{Arc, Mutex, PoisonError},
};

/// Blobs in memory, for tests. Clones share the same contents.
#[derive(Clone, Debug, Default)]
pub struct MemoryStore(Arc<Mutex<BTreeMap<BlobKey, Bytes>>>);

impl MemoryStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    fn blobs(&self) -> std::sync::MutexGuard<'_, BTreeMap<BlobKey, Bytes>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl BlobStore for MemoryStore {
    fn put<'a>(&'a self, key: &'a BlobKey, bytes: Bytes) -> BlobFuture<'a, ()> {
        Box::pin(async move {
            self.blobs().insert(key.clone(), bytes);
            Ok(())
        })
    }

    fn get_bounded<'a>(
        &'a self,
        key: &'a BlobKey,
        max_bytes: usize,
    ) -> BlobFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            let blobs = self.blobs();
            let found = blobs.get(key);
            if found.is_some_and(|bytes| bytes.len() > max_bytes) {
                return Err(BlobError::new("blob exceeds byte limit"));
            }
            Ok(found.cloned())
        })
    }

    fn delete<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, ()> {
        Box::pin(async move {
            self.blobs().remove(key);
            Ok(())
        })
    }

    fn list<'a>(
        &'a self,
        prefix: &'a BlobKey,
        after: Option<&'a BlobKey>,
        limit: usize,
    ) -> BlobFuture<'a, Vec<BlobKey>> {
        Box::pin(async move {
            if limit == 0 {
                return Ok(Vec::new());
            }
            let blobs = self.blobs();
            let mut keys = Vec::new();
            if after.is_none_or(|after| after < prefix) && blobs.contains_key(prefix) {
                keys.push(prefix.clone());
            }
            let from = format!("{prefix}/");
            let to = format!("{prefix}0");
            if after.is_some_and(|after| after.as_str() >= to.as_str()) {
                return Ok(keys);
            }
            let lower = match after {
                Some(after) if after.as_str() >= from.as_str() => Bound::Excluded(after.as_str()),
                _ => Bound::Included(from.as_str()),
            };
            let remaining = limit - keys.len();
            keys.extend(
                blobs
                    .range::<str, _>((lower, Bound::Excluded(to.as_str())))
                    .take(remaining)
                    .map(|(key, _)| key.clone()),
            );
            Ok(keys)
        })
    }
}
