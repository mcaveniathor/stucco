use crate::{Codec, KeyCodec, PostcardCodec, StoreError, Table, U64Key};
use redb::{ReadableDatabase, ReadableTable};
use std::sync::Arc;
type Projection<V> = Arc<dyn Fn(&V) -> Vec<u8> + Send + Sync>;
/// A table with one transactionally maintained secondary order.
///
/// After creating an index, write through this handle exclusively. Writes
/// through a previously cloned unindexed Table do not maintain the index.
pub struct IndexTable<K, V, KC = U64Key, VC = PostcardCodec<V>> {
    pub(crate) table: Table<K, V, KC, VC>,
    pub(crate) name: String,
    pub(crate) key: String,
    pub(crate) projection: Projection<V>,
}
impl<K, V, KC, VC> Clone for IndexTable<K, V, KC, VC> {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            name: self.name.clone(),
            key: self.key.clone(),
            projection: self.projection.clone(),
        }
    }
}
impl<K, V, KC, VC> std::fmt::Debug for IndexTable<K, V, KC, VC> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexTable")
            .field("table", &self.table)
            .field("index", &self.key)
            .finish_non_exhaustive()
    }
}
impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> Table<K, V, KC, VC> {
    /// Opens an index. Call rebuild when indexing an already-populated table.
    pub fn index(
        self,
        name: &str,
        projection: impl Fn(&V) -> Vec<u8> + Send + Sync + 'static,
    ) -> Result<IndexTable<K, V, KC, VC>, StoreError> {
        crate::store::validate_name(name)?;
        let physical = format!("{}--{name}", self.name);
        crate::store::validate_name(&physical)?;
        let tx = self.store.db.begin_write().map_err(StoreError::new)?;
        {
            tx.open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&physical))
                .map_err(StoreError::new)?;
        }
        tx.commit().map_err(StoreError::new)?;
        Ok(IndexTable {
            table: self,
            name: physical,
            key: name.to_owned(),
            projection: Arc::new(projection),
        })
    }
}
impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> IndexTable<K, V, KC, VC> {
    /// Reads a record by primary identity.
    pub fn get(&self, key: &K) -> Result<Option<V>, StoreError> {
        self.table.get(key)
    }
    /// Inserts one value while updating the secondary index atomically.
    pub fn put(&self, key: &K, value: &V) -> Result<(), StoreError> {
        self.write(std::iter::once((key, value)))
    }
    /// Commits a batch atomically. Useful for startup seeding.
    pub fn put_many(&self, rows: &[(K, V)]) -> Result<(), StoreError> {
        self.write(rows.iter().map(|(k, v)| (k, v)))
    }
    fn write<'a>(&self, rows: impl Iterator<Item = (&'a K, &'a V)>) -> Result<(), StoreError>
    where
        K: 'a,
        V: 'a,
    {
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        {
            let mut data = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.table.name))
                .map_err(StoreError::new)?;
            let mut index = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
                .map_err(StoreError::new)?;
            for (key, value) in rows {
                let key = self.table.keys.encode(key)?;
                let encoded = self.table.values.encode(value)?;
                let old = data
                    .get(key.as_slice())
                    .map_err(StoreError::new)?
                    .map(|v| self.table.values.decode(v.value()))
                    .transpose()?;
                if let Some(old) = old {
                    let previous = compound(&(self.projection)(&old), &key);
                    index.remove(previous.as_slice()).map_err(StoreError::new)?;
                }
                let order = compound(&(self.projection)(value), &key);
                data.insert(key.as_slice(), encoded.as_slice())
                    .map_err(StoreError::new)?;
                index
                    .insert(order.as_slice(), key.as_slice())
                    .map_err(StoreError::new)?;
            }
        }
        tx.commit().map_err(StoreError::new)
    }
    /// Deletes both record and index entry in one transaction.
    pub fn remove(&self, key: &K) -> Result<bool, StoreError> {
        let key = self.table.keys.encode(key)?;
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        let existed = {
            let mut data = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.table.name))
                .map_err(StoreError::new)?;
            let mut index = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
                .map_err(StoreError::new)?;
            let old = data
                .get(key.as_slice())
                .map_err(StoreError::new)?
                .map(|v| self.table.values.decode(v.value()))
                .transpose()?;
            if let Some(old) = old {
                index
                    .remove(compound(&(self.projection)(&old), &key).as_slice())
                    .map_err(StoreError::new)?;
                data.remove(key.as_slice()).map_err(StoreError::new)?;
                true
            } else {
                false
            }
        };
        tx.commit().map_err(StoreError::new)?;
        Ok(existed)
    }
    /// Rebuilds the entire index in one transaction. This is an O(n) operation.
    pub fn rebuild(&self) -> Result<(), StoreError> {
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        {
            let data = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.table.name))
                .map_err(StoreError::new)?;
            let mut index = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
                .map_err(StoreError::new)?;
            index.retain(|_, _| false).map_err(StoreError::new)?;
            for row in data.iter().map_err(StoreError::new)? {
                let (key, value) = row.map_err(StoreError::new)?;
                let value = self.table.values.decode(value.value())?;
                let order = compound(&(self.projection)(&value), key.value());
                index
                    .insert(order.as_slice(), key.value())
                    .map_err(StoreError::new)?;
            }
        }
        tx.commit().map_err(StoreError::new)
    }
    /// Reads all rows and their order keys. Explicitly O(n), for small-data
    /// numbered pagination or maintenance; use scan for bounded cursor reads.
    pub fn all(&self, indexed: bool) -> Result<Vec<(Vec<u8>, V)>, StoreError> {
        let tx = self.table.store.db.begin_read().map_err(StoreError::new)?;
        let data = tx
            .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.table.name))
            .map_err(StoreError::new)?;
        let mut rows = Vec::new();
        if indexed {
            let index = tx
                .open_table(redb::TableDefinition::<&[u8], &[u8]>::new(&self.name))
                .map_err(StoreError::new)?;
            for entry in index.iter().map_err(StoreError::new)? {
                let (order, key) = entry.map_err(StoreError::new)?;
                let value = data
                    .get(key.value())
                    .map_err(StoreError::new)?
                    .ok_or_else(|| StoreError::invalid("index points to missing record"))?;
                rows.push((
                    order.value().to_vec(),
                    self.table.values.decode(value.value())?,
                ));
            }
        } else {
            for entry in data.iter().map_err(StoreError::new)? {
                let (key, value) = entry.map_err(StoreError::new)?;
                rows.push((
                    key.value().to_vec(),
                    self.table.values.decode(value.value())?,
                ));
            }
        }
        Ok(rows)
    }
}
/// Escaped fields retain byte order and cannot collide when containing zeros.
pub(crate) fn compound(sort: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for field in [sort, key] {
        for b in field {
            if *b == 0 {
                out.extend_from_slice(&[0, 255]);
            } else {
                out.push(*b);
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}
