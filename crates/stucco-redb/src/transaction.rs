//! Multi-table transactions: [`Store::write`](crate::Store::write) and
//! [`Store::read`](crate::Store::read) hand a closure one redb transaction.
use crate::index::compound;
use crate::{Codec, IndexTable, KeyCodec, Revision, StoreError, Table};
use redb::ReadableTable;
use std::sync::Arc;

mod sealed {
    use crate::Codec;
    use std::sync::Arc;

    /// What a transaction needs to know about a table handle.
    pub struct Parts<'a, K, V> {
        pub(crate) db: &'a Arc<redb::Database>,
        pub(crate) table: &'a str,
        pub(crate) keys: &'a dyn Codec<K>,
        pub(crate) values: &'a dyn Codec<V>,
        pub(crate) index: Option<IndexParts<'a, V>>,
    }

    /// The secondary index a handle maintains.
    pub struct IndexParts<'a, V> {
        pub(crate) name: &'a str,
        pub(crate) projection: &'a (dyn Fn(&V) -> Vec<u8> + Send + Sync),
    }

    pub trait Sealed<K, V> {
        fn parts(&self) -> Parts<'_, K, V>;
    }
}
use sealed::{Parts, Sealed};

/// A [`Table`] or [`IndexTable`] that a transaction can read and write.
///
/// An `IndexTable` keeps its secondary index in step; a plain `Table` writes
/// the record only. This trait is sealed.
pub trait Handle<K, V>: sealed::Sealed<K, V> {}

impl<K: 'static, V: 'static, KC: KeyCodec<K>, VC: Codec<V>> Sealed<K, V> for Table<K, V, KC, VC> {
    fn parts(&self) -> Parts<'_, K, V> {
        Parts {
            db: &self.store.db,
            table: &self.name,
            keys: &*self.keys,
            values: &*self.values,
            index: None,
        }
    }
}
impl<K: 'static, V: 'static, KC: KeyCodec<K>, VC: Codec<V>> Handle<K, V> for Table<K, V, KC, VC> {}

impl<K: 'static, V: 'static, KC: KeyCodec<K>, VC: Codec<V>> Sealed<K, V>
    for IndexTable<K, V, KC, VC>
{
    fn parts(&self) -> Parts<'_, K, V> {
        Parts {
            db: &self.table.store.db,
            table: &self.table.name,
            keys: &*self.table.keys,
            values: &*self.table.values,
            index: Some(sealed::IndexParts {
                name: &self.name,
                projection: &*self.projection,
            }),
        }
    }
}
impl<K: 'static, V: 'static, KC: KeyCodec<K>, VC: Codec<V>> Handle<K, V>
    for IndexTable<K, V, KC, VC>
{
}

fn bytes(name: &str) -> redb::TableDefinition<'_, &'static [u8], &'static [u8]> {
    redb::TableDefinition::new(name)
}

fn same_store<K, V>(db: &Arc<redb::Database>, parts: &Parts<'_, K, V>) -> Result<(), StoreError> {
    if Arc::ptr_eq(db, parts.db) {
        Ok(())
    } else {
        Err(StoreError::invalid("table belongs to a different store"))
    }
}

/// Reads and decodes one record from an already opened record table.
pub(crate) fn get_row<K: 'static, V: 'static>(
    data: &impl ReadableTable<&'static [u8], &'static [u8]>,
    parts: &Parts<'_, K, V>,
    key: &K,
) -> Result<Option<V>, StoreError> {
    let key = parts.keys.encode(key)?;
    data.get(key.as_slice())
        .map_err(StoreError::new)?
        .map(|stored| parts.values.decode(stored.value()))
        .transpose()
}

/// Inserts or replaces a record, keeping its index (if any) in step.
pub(crate) fn put_row<K: 'static, V: 'static>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, K, V>,
    key: &K,
    value: &V,
) -> Result<(), StoreError> {
    let key = parts.keys.encode(key)?;
    let encoded = parts.values.encode(value)?;
    let mut data = tx.open_table(bytes(parts.table)).map_err(StoreError::new)?;
    let Some(index) = &parts.index else {
        data.insert(key.as_slice(), encoded.as_slice())
            .map_err(StoreError::new)?;
        return Ok(());
    };
    let mut order = tx.open_table(bytes(index.name)).map_err(StoreError::new)?;
    let old = data
        .get(key.as_slice())
        .map_err(StoreError::new)?
        .map(|stored| parts.values.decode(stored.value()))
        .transpose()?;
    if let Some(old) = old {
        order
            .remove(compound(&(index.projection)(&old), &key).as_slice())
            .map_err(StoreError::new)?;
    }
    data.insert(key.as_slice(), encoded.as_slice())
        .map_err(StoreError::new)?;
    order
        .insert(
            compound(&(index.projection)(value), &key).as_slice(),
            key.as_slice(),
        )
        .map_err(StoreError::new)?;
    Ok(())
}

/// Removes a record and its index entry; returns whether it existed.
pub(crate) fn remove_row<K: 'static, V: 'static>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, K, V>,
    key: &K,
) -> Result<bool, StoreError> {
    let key = parts.keys.encode(key)?;
    let mut data = tx.open_table(bytes(parts.table)).map_err(StoreError::new)?;
    let Some(index) = &parts.index else {
        return Ok(data
            .remove(key.as_slice())
            .map_err(StoreError::new)?
            .is_some());
    };
    let mut order = tx.open_table(bytes(index.name)).map_err(StoreError::new)?;
    let old = data
        .get(key.as_slice())
        .map_err(StoreError::new)?
        .map(|stored| parts.values.decode(stored.value()))
        .transpose()?;
    let Some(old) = old else {
        return Ok(false);
    };
    order
        .remove(compound(&(index.projection)(&old), &key).as_slice())
        .map_err(StoreError::new)?;
    data.remove(key.as_slice()).map_err(StoreError::new)?;
    Ok(true)
}

/// Reads one record with the revision of its stored bytes.
pub(crate) fn get_row_with_revision<K: 'static, V: 'static>(
    data: &impl ReadableTable<&'static [u8], &'static [u8]>,
    parts: &Parts<'_, K, V>,
    key: &K,
) -> Result<Option<(V, Revision)>, StoreError> {
    let key = parts.keys.encode(key)?;
    let Some(stored) = data.get(key.as_slice()).map_err(StoreError::new)? else {
        return Ok(None);
    };
    let stored = stored.value();
    Ok(Some((parts.values.decode(stored)?, Revision::of(stored))))
}

/// Checks the revision, then writes `change`'s result (or removes the record
/// on `None`) with its index. Everything fallible runs before the first write,
/// so a failure leaves the transaction exactly as it was.
pub(crate) fn conditional_row<K: 'static, V: 'static>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, K, V>,
    key: &K,
    expected: &Revision,
    change: impl FnOnce(V) -> Option<V>,
) -> Result<Option<Revision>, StoreError> {
    let key = parts.keys.encode(key)?;
    let mut data = tx.open_table(bytes(parts.table)).map_err(StoreError::new)?;
    let current = data
        .get(key.as_slice())
        .map_err(StoreError::new)?
        .map(|stored| stored.value().to_vec());
    let Some(current) = current else {
        return Err(StoreError::conflict(None));
    };
    let actual = Revision::of(&current);
    if actual != *expected {
        return Err(StoreError::conflict(Some(actual)));
    }
    let old = parts.values.decode(&current)?;
    let previous = parts
        .index
        .as_ref()
        .map(|index| compound(&(index.projection)(&old), &key));
    let written = match change(old) {
        Some(new) => {
            let encoded = parts.values.encode(&new)?;
            let order = parts
                .index
                .as_ref()
                .map(|index| compound(&(index.projection)(&new), &key));
            Some((encoded, order))
        }
        None => None,
    };
    let mut order_table = parts
        .index
        .as_ref()
        .map(|index| tx.open_table(bytes(index.name)).map_err(StoreError::new))
        .transpose()?;
    if let (Some(table), Some(previous)) = (&mut order_table, &previous) {
        table.remove(previous.as_slice()).map_err(StoreError::new)?;
    }
    match written {
        Some((encoded, order)) => {
            data.insert(key.as_slice(), encoded.as_slice())
                .map_err(StoreError::new)?;
            if let (Some(table), Some(order)) = (&mut order_table, &order) {
                table
                    .insert(order.as_slice(), key.as_slice())
                    .map_err(StoreError::new)?;
            }
            Ok(Some(Revision::of(&encoded)))
        }
        None => {
            data.remove(key.as_slice()).map_err(StoreError::new)?;
            Ok(None)
        }
    }
}

/// A write transaction spanning any number of tables of one [`Store`](crate::Store).
///
/// Created by [`Store::write`](crate::Store::write); everything done through
/// it commits or rolls back together.
pub struct WriteTx {
    tx: redb::WriteTransaction,
    db: Arc<redb::Database>,
}

impl WriteTx {
    pub(crate) fn new(tx: redb::WriteTransaction, db: Arc<redb::Database>) -> Self {
        Self { tx, db }
    }

    pub(crate) fn commit(self) -> Result<(), StoreError> {
        self.tx.commit().map_err(StoreError::new)
    }

    pub(crate) fn abort(self) {
        let _ = self.tx.abort();
    }

    /// Reads a value, including this transaction's own earlier writes.
    pub fn get<K: 'static, V: 'static, H: Handle<K, V>>(
        &self,
        table: &H,
        key: &K,
    ) -> Result<Option<V>, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        let data = self
            .tx
            .open_table(bytes(parts.table))
            .map_err(StoreError::new)?;
        get_row(&data, &parts, key)
    }

    /// Inserts or replaces a value, updating the table's index if it has one.
    pub fn put<K: 'static, V: 'static, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
        value: &V,
    ) -> Result<(), StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        put_row(&self.tx, &parts, key, value)
    }

    /// Removes a value and its index entry; returns whether it existed.
    pub fn remove<K: 'static, V: 'static, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
    ) -> Result<bool, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        remove_row(&self.tx, &parts, key)
    }

    /// Reads a value and its revision, including this transaction's own writes.
    pub fn get_with_revision<K: 'static, V: 'static, H: Handle<K, V>>(
        &self,
        table: &H,
        key: &K,
    ) -> Result<Option<(V, Revision)>, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        let data = self
            .tx
            .open_table(bytes(parts.table))
            .map_err(StoreError::new)?;
        get_row_with_revision(&data, &parts, key)
    }

    /// Replaces the value with `change(current)` if its revision is still
    /// `expected`, and returns the new revision. Otherwise nothing is written
    /// and the error is a conflict (see [`IndexTable::update_if`]); the
    /// transaction stays usable, so you can handle the conflict or return it
    /// to abort everything.
    pub fn update_if<K: 'static, V: 'static, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
        expected: &Revision,
        change: impl FnOnce(V) -> V,
    ) -> Result<Revision, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        conditional_row(&self.tx, &parts, key, expected, |old| Some(change(old)))
            .map(|revision| revision.expect("update writes a value"))
    }

    /// Removes the record if its revision is still `expected`; otherwise a
    /// conflict as for [`update_if`](Self::update_if).
    pub fn remove_if<K: 'static, V: 'static, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
        expected: &Revision,
    ) -> Result<(), StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        conditional_row(&self.tx, &parts, key, expected, |_| None).map(|_| ())
    }
}

/// A read-only snapshot spanning any number of tables of one [`Store`](crate::Store).
///
/// Created by [`Store::read`](crate::Store::read).
pub struct ReadTx {
    tx: redb::ReadTransaction,
    db: Arc<redb::Database>,
}

impl ReadTx {
    pub(crate) fn new(tx: redb::ReadTransaction, db: Arc<redb::Database>) -> Self {
        Self { tx, db }
    }

    /// Reads a value from the snapshot.
    pub fn get<K: 'static, V: 'static, H: Handle<K, V>>(
        &self,
        table: &H,
        key: &K,
    ) -> Result<Option<V>, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        let data = self
            .tx
            .open_table(bytes(parts.table))
            .map_err(StoreError::new)?;
        get_row(&data, &parts, key)
    }

    /// Reads a value and its revision from the snapshot.
    pub fn get_with_revision<K: 'static, V: 'static, H: Handle<K, V>>(
        &self,
        table: &H,
        key: &K,
    ) -> Result<Option<(V, Revision)>, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        let data = self
            .tx
            .open_table(bytes(parts.table))
            .map_err(StoreError::new)?;
        get_row_with_revision(&data, &parts, key)
    }
}
