use crate::{Codec, KeyCodec, ReadTx, StoreError, Table, WriteTx};
use redb::ReadableDatabase;
use std::{path::Path, sync::Arc};
/// An embedded database; clones share the same open handle.
#[derive(Clone, Debug)]
pub struct Store {
    pub(crate) db: Arc<redb::Database>,
}
impl Store {
    /// Opens or creates a database. redb 4.x files need explicit migration
    /// when changing incompatible file-format versions.
    ///
    /// ```no_run
    /// let store = stucco_redb::Store::open("state.redb")?;
    /// # Ok::<(),stucco_redb::StoreError>(())
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Ok(Self {
            db: Arc::new(redb::Database::create(path).map_err(StoreError::new)?),
        })
    }
    /// Runs `work` in one write transaction across any number of tables.
    ///
    /// The transaction commits when `work` returns `Ok`. If it returns `Err`
    /// or panics, nothing is written (a panic is resumed after the rollback).
    /// It also rolls back, returning an error, when a write failed partway
    /// (see [`WriteTx`]) even if `work` handled that error and returned `Ok`.
    /// `E` may be your own error type; it only needs `From<StoreError>`, and a
    /// closure that only ever returns `Ok` needs `Ok::<_, StoreError>(..)` so
    /// the compiler can pick `E`. Calling `write` (or any single-table write)
    /// from inside `work` on the same thread deadlocks on redb's writer lock.
    ///
    /// ```no_run
    /// use stucco_redb::{PostcardCodec, Store, StoreError, U64Key};
    /// let store = Store::open("orders.redb")?;
    /// let orders = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
    /// let audit = store.table::<u64, String, _, _>("audit", U64Key, PostcardCodec::default())?;
    /// store.write(|tx| {
    ///     tx.put(&orders, &1, &"Ada".to_string())?;
    ///     tx.put(&audit, &1, &"order 1 created".to_string())?;
    ///     Ok::<_, StoreError>(()) // both rows commit together
    /// })?;
    /// # Ok::<(), StoreError>(())
    /// ```
    pub fn write<T, E: From<StoreError>>(
        &self,
        work: impl FnOnce(&mut WriteTx) -> Result<T, E>,
    ) -> Result<T, E> {
        let tx = self.db.begin_write().map_err(StoreError::new)?;
        let mut tx = WriteTx::new(tx, self.db.clone());
        // redb skips its own abort while unwinding and flags the file for
        // repair, so roll back explicitly before resuming a panic.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&mut tx))) {
            Ok(Ok(_)) if tx.is_poisoned() => {
                tx.abort();
                Err(StoreError::invalid("transaction rolled back: a write failed partway").into())
            }
            Ok(Ok(value)) => {
                tx.commit()?;
                Ok(value)
            }
            Ok(Err(error)) => {
                tx.abort();
                Err(error)
            }
            Err(panic) => {
                tx.abort();
                std::panic::resume_unwind(panic)
            }
        }
    }

    /// Runs `work` against one consistent snapshot of any number of tables.
    pub fn read<T, E: From<StoreError>>(
        &self,
        work: impl FnOnce(&ReadTx) -> Result<T, E>,
    ) -> Result<T, E> {
        let tx = self.db.begin_read().map_err(StoreError::new)?;
        work(&ReadTx::new(tx, self.db.clone()))
    }

    /// Opens a byte-backed typed table. Caller supplies an order-preserving key codec.
    pub fn table<K, V, KC: KeyCodec<K>, VC: Codec<V>>(
        &self,
        name: &str,
        keys: KC,
        values: VC,
    ) -> Result<Table<K, V, KC, VC>, StoreError> {
        validate_name(name)?;
        let tx = self.db.begin_write().map_err(StoreError::new)?;
        {
            tx.open_table(redb::TableDefinition::<&[u8], &[u8]>::new(name))
                .map_err(StoreError::new)?;
        }
        tx.commit().map_err(StoreError::new)?;
        Ok(Table {
            store: self.clone(),
            name: name.to_owned(),
            keys: Arc::new(keys),
            values: Arc::new(values),
            marker: std::marker::PhantomData,
        })
    }
}
pub(crate) fn validate_name(name: &str) -> Result<(), StoreError> {
    if name.is_empty()
        || name.len() > 64
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        Err(StoreError::invalid("invalid table name"))
    } else {
        Ok(())
    }
}
