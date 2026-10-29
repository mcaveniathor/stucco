//! Revision-checked writes and id allocation.
use crate::transaction::{Sealed, conditional_row, create_row, get_row_with_revision};
use crate::{Codec, IndexTable, KeyCodec, Revision, StoreError, U64Key};
use redb::ReadableDatabase;

fn bytes(name: &str) -> redb::TableDefinition<'_, &'static [u8], &'static [u8]> {
    redb::TableDefinition::new(name)
}

impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> IndexTable<K, V, KC, VC> {
    /// Reads a value and its revision from one snapshot.
    pub fn get_with_revision(&self, key: &K) -> Result<Option<(V, Revision)>, StoreError> {
        let parts = self.parts();
        let tx = self.table.store.db.begin_read().map_err(StoreError::new)?;
        let data = tx
            .open_table(bytes(&self.table.name))
            .map_err(StoreError::new)?;
        get_row_with_revision(&data, &parts, key)
    }

    /// The revision `value` has once stored. No I/O: use it for rows that
    /// came from a scan. It matches the stored revision whenever the stored
    /// bytes are what the codec encodes today, which holds for every write
    /// and, after a codec or schema change, once [`rebuild`](Self::rebuild)
    /// has run.
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
        let parts = self.parts();
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        let revision = conditional_row(&tx, &parts, key, expected, change, &mut false)?;
        tx.commit().map_err(StoreError::new)?;
        Ok(revision)
    }
}

impl<V, VC: Codec<V>> IndexTable<u64, V, U64Key, VC> {
    /// Stores `build(id)` under the next id (one more than the highest key,
    /// or 1 when empty), allocated in the same transaction, so concurrent
    /// creates never share an id. Deleting the highest record frees its id
    /// for reuse; a stale revision for the old record conflicts with the new
    /// one unless the two have byte-identical values, in which case the edit
    /// was based on exactly the data stored and goes ahead.
    pub fn create(&self, build: impl FnOnce(u64) -> V) -> Result<(u64, Revision), StoreError> {
        let parts = self.parts();
        let tx = self.table.store.db.begin_write().map_err(StoreError::new)?;
        let created = create_row(&tx, &parts, build, &mut false)?;
        tx.commit().map_err(StoreError::new)?;
        Ok(created)
    }
}
