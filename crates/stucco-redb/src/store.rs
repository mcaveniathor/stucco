use crate::{Codec, KeyCodec, StoreError, Table};
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
