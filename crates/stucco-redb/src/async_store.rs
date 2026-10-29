use crate::{AsyncTable, IndexTable, ReadTx, Store, StoreError, WriteTx};
use std::sync::Arc;
use tokio::sync::Semaphore;

/// A [`Store`] for async handlers: each transaction runs on Tokio's blocking
/// pool and takes one of at most `concurrency` permits for its whole run, so a
/// write lock is never held across an `.await`.
///
/// Tables made with [`table`](Self::table), and the store of an
/// [`AsyncTable`] (see [`AsyncTable::store`]), draw on the same permits as the
/// transactions, so one budget covers all of them. Clones share it too.
///
/// Started transactions cannot be cancelled: a dropped call that has started
/// still commits or rolls back, and keeps its permit until then. A call
/// dropped while still waiting for a permit runs nothing. Wrap the future in
/// `tokio::time::timeout` for a deadline.
///
/// ```no_run
/// # async fn demo() -> Result<(), stucco_redb::StoreError> {
/// use stucco_redb::{AsyncStore, PostcardCodec, Store, StoreError, U64Key};
/// let store = Store::open("orders.redb")?;
/// let orders = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
/// let audit = store.table::<u64, String, _, _>("audit", U64Key, PostcardCodec::default())?;
/// let store = AsyncStore::new(store);
/// store
///     .write(move |tx| {
///         tx.put(&orders, &1, &"Ada".to_string())?;
///         tx.put(&audit, &1, &"order 1 created".to_string())?;
///         Ok::<_, StoreError>(())
///     })
///     .await?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct AsyncStore {
    store: Store,
    permits: Arc<Semaphore>,
}

impl std::fmt::Debug for AsyncStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncStore")
            .field("available", &self.permits.available_permits())
            .finish_non_exhaustive()
    }
}

impl AsyncStore {
    /// Wraps `store`, running at most four calls at once.
    pub fn new(store: Store) -> Self {
        Self::with_permits(store, Arc::new(Semaphore::new(4)))
    }

    pub(crate) fn with_permits(store: Store, permits: Arc<Semaphore>) -> Self {
        Self { store, permits }
    }

    /// Sets the limit before sharing the store. Zero means one.
    pub fn concurrency(mut self, limit: usize) -> Self {
        self.permits = Arc::new(Semaphore::new(limit.max(1)));
        self
    }

    /// The synchronous store, for startup work.
    pub fn blocking(&self) -> &Store {
        &self.store
    }

    /// An [`AsyncTable`] over `table` that shares this store's permits.
    pub fn table<K, V, KC, VC>(&self, table: IndexTable<K, V, KC, VC>) -> AsyncTable<K, V, KC, VC> {
        AsyncTable::with_permits(table, self.permits.clone())
    }

    /// Runs `work` in one write transaction on the blocking pool. See
    /// [`Store::write`] for commit, rollback and the error type `E`.
    pub async fn write<T, E>(
        &self,
        work: impl FnOnce(&mut WriteTx) -> Result<T, E> + Send + 'static,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<StoreError> + Send + 'static,
    {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(StoreError::new)?;
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            store.write(work)
        })
        .await
        .map_err(StoreError::new)?
    }

    /// Runs `work` against one snapshot on the blocking pool. See
    /// [`Store::read`].
    pub async fn read<T, E>(
        &self,
        work: impl FnOnce(&ReadTx) -> Result<T, E> + Send + 'static,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<StoreError> + Send + 'static,
    {
        let permit = self
            .permits
            .clone()
            .acquire_owned()
            .await
            .map_err(StoreError::new)?;
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            store.read(work)
        })
        .await
        .map_err(StoreError::new)?
    }
}
