use crate::{AsyncStore, Codec, IndexTable, KeyCodec, PostcardCodec, Revision, StoreError, U64Key};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// An [`IndexTable`] for async handlers: each call runs on Tokio's blocking
/// pool, at most `concurrency` at a time. A [`RedbCollection`](crate::RedbCollection)
/// built on it shares the same limit for its scans. Clones share it too.
///
/// Started calls cannot be cancelled; a dropped call keeps its permit until
/// the blocking work finishes.
pub struct AsyncTable<K, V, KC = U64Key, VC = PostcardCodec<V>> {
    table: IndexTable<K, V, KC, VC>,
    permits: Arc<Semaphore>,
}

impl<K, V, KC, VC> Clone for AsyncTable<K, V, KC, VC> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            permits: self.permits.clone(),
        }
    }
}

impl<K, V, KC, VC> std::fmt::Debug for AsyncTable<K, V, KC, VC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncTable")
            .field("table", &self.table)
            .field("available", &self.permits.available_permits())
            .finish()
    }
}

impl<K, V, KC, VC> AsyncTable<K, V, KC, VC> {
    /// Wraps `table`, running at most four calls at once.
    pub fn new(table: IndexTable<K, V, KC, VC>) -> Self {
        Self {
            table,
            permits: Arc::new(Semaphore::new(4)),
        }
    }

    pub(crate) fn with_permits(table: IndexTable<K, V, KC, VC>, permits: Arc<Semaphore>) -> Self {
        Self { table, permits }
    }

    /// An [`AsyncStore`] sharing this table's permits, for transactions that
    /// span several tables.
    pub fn store(&self) -> AsyncStore {
        AsyncStore::with_permits(self.table.table.store.clone(), self.permits.clone())
    }

    /// Sets the limit before sharing the table. Zero means one.
    pub fn concurrency(mut self, limit: usize) -> Self {
        self.permits = Arc::new(Semaphore::new(limit.max(1)));
        self
    }

    /// The synchronous table, for startup work such as seeding or `rebuild`.
    pub fn blocking(&self) -> &IndexTable<K, V, KC, VC> {
        &self.table
    }

    pub(crate) fn permits(&self) -> &Arc<Semaphore> {
        &self.permits
    }
}

impl<K: Send + 'static, V: Send + 'static, KC: KeyCodec<K>, VC: Codec<V>> AsyncTable<K, V, KC, VC> {
    pub(crate) async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&IndexTable<K, V, KC, VC>) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(StoreError::new)?;
        let table = self.table.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work(&table)
        })
        .await
        .map_err(StoreError::new)?
    }

    /// Reads one value.
    pub async fn get(&self, key: K) -> Result<Option<V>, StoreError> {
        self.run(move |t| t.get(&key)).await
    }

    /// Reads a value and its revision.
    pub async fn get_with_revision(&self, key: K) -> Result<Option<(V, Revision)>, StoreError> {
        self.run(move |t| t.get_with_revision(&key)).await
    }

    /// Inserts or replaces a value without a revision check.
    pub async fn put(&self, key: K, value: V) -> Result<(), StoreError> {
        self.run(move |t| t.put(&key, &value)).await
    }

    /// See [`IndexTable::update_if`].
    pub async fn update_if(
        &self,
        key: K,
        expected: Revision,
        change: impl FnOnce(V) -> V + Send + 'static,
    ) -> Result<Revision, StoreError> {
        self.run(move |t| t.update_if(&key, &expected, change))
            .await
    }

    /// Removes a value without a revision check; returns whether it existed.
    pub async fn remove(&self, key: K) -> Result<bool, StoreError> {
        self.run(move |t| t.remove(&key)).await
    }

    /// See [`IndexTable::remove_if`].
    pub async fn remove_if(&self, key: K, expected: Revision) -> Result<(), StoreError> {
        self.run(move |t| t.remove_if(&key, &expected)).await
    }

    /// See [`IndexTable::revision_of`]. No I/O.
    pub fn revision_of(&self, value: &V) -> Result<Revision, StoreError> {
        self.table.revision_of(value)
    }
}

impl<V: Send + 'static, VC: Codec<V>> AsyncTable<u64, V, U64Key, VC> {
    /// See [`IndexTable::create`].
    pub async fn create(
        &self,
        build: impl FnOnce(u64) -> V + Send + 'static,
    ) -> Result<(u64, Revision), StoreError> {
        self.run(move |t| t.create(build)).await
    }
}
