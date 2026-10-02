use crate::{Codec, KeyCodec, PostcardCodec, Store, StoreError, U64Key};
use redb::ReadableDatabase;
use std::{marker::PhantomData, sync::Arc};
/// A typed handle. Values are owned; guards never escape transactions.
pub struct Table<K, V, KC = U64Key, VC = PostcardCodec<V>> {
    pub(crate) store: Store,
    pub(crate) name: String,
    pub(crate) keys: Arc<KC>,
    pub(crate) values: Arc<VC>,
    pub(crate) marker: PhantomData<fn() -> (K, V)>,
}
impl<K, V, KC, VC> Clone for Table<K, V, KC, VC> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            name: self.name.clone(),
            keys: self.keys.clone(),
            values: self.values.clone(),
            marker: PhantomData,
        }
    }
}
impl<K, V, KC, VC> std::fmt::Debug for Table<K, V, KC, VC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Table")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}
impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> Table<K, V, KC, VC> {
    /// Scans primary-key order without creating an index. Secondary index
    /// requests require an IndexTable handle.
    pub fn scan(
        &self,
        request: &crate::ScanRequest,
        keep: impl Fn(&V) -> bool,
    ) -> Result<stucco_core::CollectionPage<V>, StoreError> {
        if request.index.is_some() {
            return Err(StoreError::invalid("primary table has no secondary index"));
        }
        // Reuse the same snapshot/cursor algorithm. With index=None it only
        // opens the primary table, and never invokes the secondary projection.
        crate::IndexTable {
            table: self.clone(),
            name: self.name.clone(),
            key: String::new(),
            projection: Arc::new(|_| Vec::new()),
        }
        .scan(request, keep)
    }
    /// Reads one value from a snapshot.
    pub fn get(&self, key: &K) -> Result<Option<V>, StoreError> {
        let key = self.keys.encode(key)?;
        let tx = self.store.db.begin_read().map_err(StoreError::new)?;
        let table = tx
            .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
            .map_err(StoreError::new)?;
        table
            .get(key.as_slice())
            .map_err(StoreError::new)?
            .map(|v| self.values.decode(v.value()))
            .transpose()
    }
    /// Inserts or replaces a value, committing durably before returning.
    pub fn put(&self, key: &K, value: &V) -> Result<(), StoreError> {
        let key = self.keys.encode(key)?;
        let value = self.values.encode(value)?;
        let tx = self.store.db.begin_write().map_err(StoreError::new)?;
        {
            let mut t = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
                .map_err(StoreError::new)?;
            t.insert(key.as_slice(), value.as_slice())
                .map_err(StoreError::new)?;
        }
        tx.commit().map_err(StoreError::new)
    }
    /// Removes a value; returns whether it existed.
    pub fn remove(&self, key: &K) -> Result<bool, StoreError> {
        let key = self.keys.encode(key)?;
        let tx = self.store.db.begin_write().map_err(StoreError::new)?;
        let existed = {
            let mut t = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
                .map_err(StoreError::new)?;
            t.remove(key.as_slice()).map_err(StoreError::new)?.is_some()
        };
        tx.commit().map_err(StoreError::new)?;
        Ok(existed)
    }
}
