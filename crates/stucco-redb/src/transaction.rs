//! Multi-table transactions: [`Store::write`](crate::Store::write) and
//! [`Store::read`](crate::Store::read) hand a closure one redb transaction.
use crate::index::compound;
use crate::{Codec, IndexTable, KeyCodec, Revision, StoreError, Table, U64Key};
use redb::ReadableTable;
use std::sync::Arc;

mod sealed {
    use crate::Codec;
    use std::sync::Arc;

    /// What a transaction needs to know about a table handle.
    pub struct Parts<'a, V, KC, VC> {
        pub(crate) db: &'a Arc<redb::Database>,
        pub(crate) table: &'a str,
        pub(crate) keys: &'a KC,
        pub(crate) values: &'a VC,
        pub(crate) index: Option<IndexParts<'a, V>>,
    }

    /// The secondary index a handle maintains.
    pub struct IndexParts<'a, V> {
        pub(crate) name: &'a str,
        pub(crate) projection: &'a (dyn Fn(&V) -> Vec<u8> + Send + Sync),
    }

    pub trait Sealed<K, V> {
        type Keys: Codec<K>;
        type Values: Codec<V>;
        fn parts(&self) -> Parts<'_, V, Self::Keys, Self::Values>;
    }
}
pub(crate) use sealed::{Parts, Sealed};

/// A [`Table`] or [`IndexTable`] that a transaction can read and write.
///
/// An `IndexTable` keeps its secondary index in step; a plain `Table` writes
/// the record only. This trait is sealed.
pub trait Handle<K, V>: sealed::Sealed<K, V> {}

impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> Sealed<K, V> for Table<K, V, KC, VC> {
    type Keys = KC;
    type Values = VC;
    fn parts(&self) -> Parts<'_, V, KC, VC> {
        Parts {
            db: &self.store.db,
            table: &self.name,
            keys: &*self.keys,
            values: &*self.values,
            index: None,
        }
    }
}
impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> Handle<K, V> for Table<K, V, KC, VC> {}

impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> Sealed<K, V> for IndexTable<K, V, KC, VC> {
    type Keys = KC;
    type Values = VC;
    fn parts(&self) -> Parts<'_, V, KC, VC> {
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
impl<K, V, KC: KeyCodec<K>, VC: Codec<V>> Handle<K, V> for IndexTable<K, V, KC, VC> {}

fn bytes(name: &str) -> redb::TableDefinition<'_, &'static [u8], &'static [u8]> {
    redb::TableDefinition::new(name)
}

fn same_store<V, KC, VC>(
    db: &Arc<redb::Database>,
    parts: &Parts<'_, V, KC, VC>,
) -> Result<(), StoreError> {
    if Arc::ptr_eq(db, parts.db) {
        Ok(())
    } else {
        Err(StoreError::invalid("table belongs to a different store"))
    }
}

/// Reads and decodes one record from an already opened record table.
pub(crate) fn get_row<K, V, KC: Codec<K>, VC: Codec<V>>(
    data: &impl ReadableTable<&'static [u8], &'static [u8]>,
    parts: &Parts<'_, V, KC, VC>,
    key: &K,
) -> Result<Option<V>, StoreError> {
    let key = parts.keys.encode(key)?;
    data.get(key.as_slice())
        .map_err(StoreError::new)?
        .map(|stored| parts.values.decode(stored.value()))
        .transpose()
}

/// Inserts or replaces a record, keeping its index (if any) in step.
///
/// `touched` is true from just before the first write until the last one
/// succeeds, so a caller can tell a failure that left the transaction
/// half-applied from one that changed nothing.
pub(crate) fn put_row<K, V, KC: Codec<K>, VC: Codec<V>>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, V, KC, VC>,
    key: &K,
    value: &V,
    touched: &mut bool,
) -> Result<Revision, StoreError> {
    let key = parts.keys.encode(key)?;
    let encoded = parts.values.encode(value)?;
    let mut data = tx.open_table(bytes(parts.table)).map_err(StoreError::new)?;
    let Some(index) = &parts.index else {
        *touched = true;
        data.insert(key.as_slice(), encoded.as_slice())
            .map_err(StoreError::new)?;
        *touched = false;
        return Ok(Revision::of(&encoded));
    };
    let mut order = tx.open_table(bytes(index.name)).map_err(StoreError::new)?;
    let old = data
        .get(key.as_slice())
        .map_err(StoreError::new)?
        .map(|stored| parts.values.decode(stored.value()))
        .transpose()?;
    let previous = old.map(|old| compound(&(index.projection)(&old), &key));
    let current = compound(&(index.projection)(value), &key);
    *touched = true;
    if let Some(previous) = previous {
        order.remove(previous.as_slice()).map_err(StoreError::new)?;
    }
    data.insert(key.as_slice(), encoded.as_slice())
        .map_err(StoreError::new)?;
    order
        .insert(current.as_slice(), key.as_slice())
        .map_err(StoreError::new)?;
    *touched = false;
    Ok(Revision::of(&encoded))
}

/// Removes a record and its index entry; returns whether it existed.
/// `touched` is as for [`put_row`].
pub(crate) fn remove_row<K, V, KC: Codec<K>, VC: Codec<V>>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, V, KC, VC>,
    key: &K,
    touched: &mut bool,
) -> Result<bool, StoreError> {
    let key = parts.keys.encode(key)?;
    let mut data = tx.open_table(bytes(parts.table)).map_err(StoreError::new)?;
    let Some(index) = &parts.index else {
        *touched = true;
        let existed = data
            .remove(key.as_slice())
            .map_err(StoreError::new)?
            .is_some();
        *touched = false;
        return Ok(existed);
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
    let previous = compound(&(index.projection)(&old), &key);
    *touched = true;
    order.remove(previous.as_slice()).map_err(StoreError::new)?;
    data.remove(key.as_slice()).map_err(StoreError::new)?;
    *touched = false;
    Ok(true)
}

/// Reads one record with the revision of its stored bytes.
pub(crate) fn get_row_with_revision<K, V, KC: Codec<K>, VC: Codec<V>>(
    data: &impl ReadableTable<&'static [u8], &'static [u8]>,
    parts: &Parts<'_, V, KC, VC>,
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
/// on `None`) with its index. Everything but redb's own writes runs before the
/// first write, so a conflict or codec failure leaves the transaction exactly
/// as it was; `touched` is as for [`put_row`].
pub(crate) fn conditional_row<K, V, KC: Codec<K>, VC: Codec<V>>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, V, KC, VC>,
    key: &K,
    expected: &Revision,
    change: impl FnOnce(V) -> Option<V>,
    touched: &mut bool,
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
    *touched = true;
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
            *touched = false;
            Ok(Some(Revision::of(&encoded)))
        }
        None => {
            data.remove(key.as_slice()).map_err(StoreError::new)?;
            *touched = false;
            Ok(None)
        }
    }
}

/// Stores `build(id)` under one more than the highest key (1 when empty).
pub(crate) fn create_row<V, KC: Codec<u64>, VC: Codec<V>>(
    tx: &redb::WriteTransaction,
    parts: &Parts<'_, V, KC, VC>,
    build: impl FnOnce(u64) -> V,
    touched: &mut bool,
) -> Result<(u64, Revision), StoreError> {
    let last = {
        let data = tx.open_table(bytes(parts.table)).map_err(StoreError::new)?;
        data.last()
            .map_err(StoreError::new)?
            .map(|(key, _)| U64Key.decode(key.value()))
            .transpose()?
            .unwrap_or(0)
    };
    let id = last
        .checked_add(1)
        .ok_or_else(|| StoreError::invalid("no ids left"))?;
    let revision = put_row(tx, parts, &id, &build(id), touched)?;
    Ok((id, revision))
}

/// A `u64`-keyed [`Table`] or [`IndexTable`] that can allocate ids with
/// [`WriteTx::create`]. This trait is sealed.
pub trait Creatable<V>: Handle<u64, V> {}
impl<V, VC: Codec<V>> Creatable<V> for Table<u64, V, U64Key, VC> {}
impl<V, VC: Codec<V>> Creatable<V> for IndexTable<u64, V, U64Key, VC> {}

/// A write transaction spanning any number of tables of one [`Store`](crate::Store).
///
/// Created by [`Store::write`](crate::Store::write); everything done through
/// it commits or rolls back together.
///
/// If a write fails after redb has started applying it (for example a value
/// over redb's size limit), the record and its index may disagree. The
/// transaction then becomes rollback-only: every further call returns an
/// error and [`Store::write`](crate::Store::write) rolls back even if the
/// closure returns `Ok`. Conflicts and codec errors change nothing and do not
/// have this effect.
pub struct WriteTx {
    tx: redb::WriteTransaction,
    db: Arc<redb::Database>,
    poisoned: bool,
}

impl WriteTx {
    pub(crate) fn new(tx: redb::WriteTransaction, db: Arc<redb::Database>) -> Self {
        Self {
            tx,
            db,
            poisoned: false,
        }
    }

    pub(crate) fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    fn live(&self) -> Result<(), StoreError> {
        if self.poisoned {
            Err(StoreError::invalid(
                "transaction is rollback-only after a write failed partway",
            ))
        } else {
            Ok(())
        }
    }

    /// Runs one write; a failure after redb started writing poisons `self`.
    fn mutate<T>(
        &mut self,
        write: impl FnOnce(&redb::WriteTransaction, &mut bool) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        self.live()?;
        let mut touched = false;
        let result = write(&self.tx, &mut touched);
        if result.is_err() && touched {
            self.poisoned = true;
        }
        result
    }

    pub(crate) fn commit(self) -> Result<(), StoreError> {
        self.tx.commit().map_err(StoreError::new)
    }

    pub(crate) fn abort(self) {
        let _ = self.tx.abort();
    }

    /// Reads a value, including this transaction's own earlier writes.
    pub fn get<K, V, H: Handle<K, V>>(&self, table: &H, key: &K) -> Result<Option<V>, StoreError> {
        self.live()?;
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        let data = self
            .tx
            .open_table(bytes(parts.table))
            .map_err(StoreError::new)?;
        get_row(&data, &parts, key)
    }

    /// Inserts or replaces a value, updating the table's index if it has one.
    pub fn put<K, V, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
        value: &V,
    ) -> Result<(), StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        self.mutate(|tx, touched| put_row(tx, &parts, key, value, touched))
            .map(|_| ())
    }

    /// Removes a value and its index entry; returns whether it existed.
    pub fn remove<K, V, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
    ) -> Result<bool, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        self.mutate(|tx, touched| remove_row(tx, &parts, key, touched))
    }

    /// Reads a value and its revision, including this transaction's own writes.
    pub fn get_with_revision<K, V, H: Handle<K, V>>(
        &self,
        table: &H,
        key: &K,
    ) -> Result<Option<(V, Revision)>, StoreError> {
        self.live()?;
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
    pub fn update_if<K, V, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
        expected: &Revision,
        change: impl FnOnce(V) -> V,
    ) -> Result<Revision, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        self.mutate(|tx, touched| {
            conditional_row(tx, &parts, key, expected, |old| Some(change(old)), touched)
        })
        .map(|revision| revision.expect("update writes a value"))
    }

    /// Removes the record if its revision is still `expected`; otherwise a
    /// conflict as for [`update_if`](Self::update_if).
    pub fn remove_if<K, V, H: Handle<K, V>>(
        &mut self,
        table: &H,
        key: &K,
        expected: &Revision,
    ) -> Result<(), StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        self.mutate(|tx, touched| conditional_row(tx, &parts, key, expected, |_| None, touched))
            .map(|_| ())
    }

    /// Stores `build(id)` under the next id (one more than the highest key, or
    /// 1 when empty), allocated inside this transaction, and returns the id
    /// with the value's revision. See [`IndexTable::create`] for how ids are
    /// reused after deleting the highest record.
    pub fn create<V, H: Creatable<V>>(
        &mut self,
        table: &H,
        build: impl FnOnce(u64) -> V,
    ) -> Result<(u64, Revision), StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        self.mutate(|tx, touched| create_row(tx, &parts, build, touched))
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
    pub fn get<K, V, H: Handle<K, V>>(&self, table: &H, key: &K) -> Result<Option<V>, StoreError> {
        let parts = table.parts();
        same_store(&self.db, &parts)?;
        let data = self
            .tx
            .open_table(bytes(parts.table))
            .map_err(StoreError::new)?;
        get_row(&data, &parts, key)
    }

    /// Reads a value and its revision from the snapshot.
    pub fn get_with_revision<K, V, H: Handle<K, V>>(
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
