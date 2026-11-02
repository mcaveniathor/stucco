use crate::{BlobFuture, BlobKey, BlobStore, Bytes};
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

    fn get<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, Option<Bytes>> {
        Box::pin(async move { Ok(self.blobs().get(key).cloned()) })
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
            let lower = after.map_or(Bound::Unbounded, Bound::Excluded);
            let blobs = self.blobs();
            let keys = blobs.range::<BlobKey, _>((lower, Bound::Unbounded));
            Ok(keys
                .filter(|(key, _)| key.has_prefix(prefix))
                .map(|(key, _)| key.clone())
                .take(limit)
                .collect())
        })
    }
}
