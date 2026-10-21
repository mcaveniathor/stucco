//! Revision-checked writes and id allocation.
use crate::index::compound;
use crate::{Codec, IndexTable, KeyCodec, Revision, StoreError, U64Key};
use redb::{ReadableDatabase, ReadableTable};

fn bytes(name: &str) -> redb::TableDefinition<'_, &'static [u8], &'static [u8]> {
    redb::TableDefinition::new(name)
}

impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> IndexTable<K, V, KC, VC> {
    /// Reads a value and its revision from one snapshot.
    pub fn get_with_revision(&self, key: &K) -> Result<Option<(V, Revision)>, StoreError> {
        let key = self.table.keys.encode(key)?;
        let tx = self.table.store.db.begin_read().map_err(StoreError::new)?;
        let data = tx
            .open_table(bytes(&self.table.name))
            .map_err(StoreError::new)?;
        let Some(bytes) = data.get(key.as_slice()).map_err(StoreError::new)? else {
            return Ok(None);
        };
        let bytes = bytes.value();
        Ok(Some((
            self.table.values.decode(bytes)?,
            Revision::of(bytes),
        )))
    }

    /// The revision `value` has once stored. No I/O: use it for rows that
    /// came from a scan.
    pub fn revision_of(&self, value: &V) -> Result<Revision, StoreError> {
        Ok(Revision::of(&self.table.values.encode(value)?))
    }

    /// Replaces the value with `change(current)` if its revision is still
    /// `expected`, and returns the new revision. Otherwise nothing is written
    /// and the error is a conflict carrying the current revision (`None`
    /// when the record no longer exists).
    pub fn update_if(
        &self,
        key: &K,
        expected: &Revision,
        change: impl FnOnce(V) -> V,
    ) -> Result<Revision, StoreError> {
        self.conditional(key, expected, |old| Some(change(old)))
            .map(|rev| rev.expect("update writes a value"))
    }

    /// Removes the record if its revision is still `expected`; otherwise a
    /// conflict as for [`update_if`](Self::update_if).
    pub fn remove_if(&self, key: &K, expected: &Revision) -> Result<(), StoreError> {
        self.conditional(key, expected, |_| None).map(|_| ())
    }

    /// Checks the revision and writes `change`'s result (or removes the
    /// record on `None`), with the index, in one transaction.
    fn conditional(
        &self,
        key: &K,
        expected: &Revision,
        change: impl FnOnce(V) -> Option<V>,
    ) -> Result<Option<Revision>, StoreError> {
        let key = self.table.keys.encode(key)?;
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        let revision = {
            let mut data = tx
                .open_table(bytes(&self.table.name))
                .map_err(StoreError::new)?;
            let mut index = tx.open_table(bytes(&self.name)).map_err(StoreError::new)?;
            let current = data
                .get(key.as_slice())
                .map_err(StoreError::new)?
                .map(|v| v.value().to_vec());
            let Some(current) = current else {
                return Err(StoreError::conflict(None));
            };
            let actual = Revision::of(&current);
            if actual != *expected {
                return Err(StoreError::conflict(Some(actual)));
            }
            let old = self.table.values.decode(&current)?;
            index
                .remove(compound(&(self.projection)(&old), &key).as_slice())
                .map_err(StoreError::new)?;
            match change(old) {
                Some(new) => {
                    let encoded = self.table.values.encode(&new)?;
                    data.insert(key.as_slice(), encoded.as_slice())
                        .map_err(StoreError::new)?;
                    index
                        .insert(
                            compound(&(self.projection)(&new), &key).as_slice(),
                            key.as_slice(),
                        )
                        .map_err(StoreError::new)?;
                    Some(Revision::of(&encoded))
                }
                None => {
                    data.remove(key.as_slice()).map_err(StoreError::new)?;
                    None
                }
            }
        };
        tx.commit().map_err(StoreError::new)?;
        Ok(revision)
    }
}

impl<V, VC: Codec<V>> IndexTable<u64, V, U64Key, VC> {
    /// Stores `build(id)` under the next id (one more than the highest key,
    /// or 1 when empty), allocated in the same transaction, so concurrent
    /// creates never share an id. Deleting the highest record frees its id
    /// for reuse; a stale revision for the old record still conflicts.
    pub fn create(&self, build: impl FnOnce(u64) -> V) -> Result<(u64, Revision), StoreError> {
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        let created = {
            let mut data = tx
                .open_table(bytes(&self.table.name))
                .map_err(StoreError::new)?;
            let mut index = tx.open_table(bytes(&self.name)).map_err(StoreError::new)?;
            let last = data
                .last()
                .map_err(StoreError::new)?
                .map(|(k, _)| U64Key.decode(k.value()))
                .transpose()?
                .unwrap_or(0);
            let id = last
                .checked_add(1)
                .ok_or_else(|| StoreError::invalid("no ids left"))?;
            let value = build(id);
            let key = U64Key.encode(&id)?;
            let encoded = self.table.values.encode(&value)?;
            data.insert(key.as_slice(), encoded.as_slice())
                .map_err(StoreError::new)?;
            index
                .insert(
                    compound(&(self.projection)(&value), &key).as_slice(),
                    key.as_slice(),
                )
                .map_err(StoreError::new)?;
            (id, Revision::of(&encoded))
        };
        tx.commit().map_err(StoreError::new)?;
        Ok(created)
    }
}
