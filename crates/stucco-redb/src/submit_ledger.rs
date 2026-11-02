//! Durable records of one-time submissions, written with their effects.
use crate::{IndexTable, PostcardCodec, Store, StoreError, Utf8Key, WriteTx};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use stucco_core::Direction;

#[derive(Serialize, Deserialize)]
struct Entry<V> {
    key: String,
    expires_at: u64,
    outcome: V,
}

type Entries<V> = IndexTable<String, Entry<V>, Utf8Key, PostcardCodec<Entry<V>>>;

/// Remembers what happened for each key until it expires, so a repeated
/// request takes effect once and can be answered with the first outcome.
///
/// Check and record inside the write transaction that applies the effect.
/// redb runs one writer at a time, so of two racing requests the second sees
/// the first's record, and if the effect rolls back so does the record.
///
/// Keys are any strings. Use `SubmitKey::id` from `stucco_tower::SingleSubmit`
/// to give each rendered form one effect, or a key of your own, such as a user
/// and a target with a short expiry, to absorb repeats from separate forms.
/// Times are seconds since the Unix epoch.
///
/// Requires the `submit-ledger` feature.
///
/// ```no_run
/// use stucco_redb::{PostcardCodec, Store, StoreError, SubmitLedger, U64Key};
/// let store = Store::open("orders.redb")?;
/// let orders = store.table::<u64, String, _, _>("orders", U64Key, PostcardCodec::default())?;
/// let submits = SubmitLedger::<u64>::open(&store, "order_submits")?;
/// let (key, expires_at, now) = ("id from SubmitKey::id", 1_900_003_600, 1_900_000_000);
///
/// let order = store.write(|tx| {
///     if let Some(first) = submits.replay(tx, key, now)? {
///         return Ok::<_, StoreError>(first); // a double tap: nothing changes
///     }
///     let (id, _) = tx.create(&orders, |id| format!("order {id}"))?;
///     submits.record(tx, key, expires_at, id)?;
///     Ok(id)
/// })?;
/// # Ok::<(), StoreError>(())
/// ```
pub struct SubmitLedger<V: 'static> {
    entries: Entries<V>,
}

impl<V> Clone for SubmitLedger<V> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
        }
    }
}

impl<V> std::fmt::Debug for SubmitLedger<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubmitLedger")
            .field("entries", &self.entries)
            .finish()
    }
}

impl<V: Serialize + DeserializeOwned + 'static> SubmitLedger<V> {
    /// Opens the ledger stored in table `name`.
    pub fn open(store: &Store, name: &str) -> Result<Self, StoreError> {
        let entries = store
            .table(name, Utf8Key, PostcardCodec::default())?
            .index("expires", |entry: &Entry<V>| {
                entry.expires_at.to_be_bytes().to_vec()
            })?;
        Ok(Self { entries })
    }

    /// The outcome recorded for `key`, unless it has expired by `now`.
    pub fn replay(&self, tx: &WriteTx, key: &str, now: u64) -> Result<Option<V>, StoreError> {
        let entry = tx.get(&self.entries, &key.to_owned())?;
        Ok(entry
            .filter(|entry| entry.expires_at > now)
            .map(|entry| entry.outcome))
    }

    /// Records `outcome` for `key` until `expires_at`, replacing any entry
    /// that [`replay`](Self::replay) no longer returns.
    pub fn record(
        &self,
        tx: &mut WriteTx,
        key: &str,
        expires_at: u64,
        outcome: V,
    ) -> Result<(), StoreError> {
        let key = key.to_owned();
        let entry = Entry {
            key: key.clone(),
            expires_at,
            outcome,
        };
        tx.put(&self.entries, &key, &entry)
    }

    /// Deletes up to `limit` expired entries and returns how many went. Run it
    /// on a timer, off the async executor; it visits only the entries it
    /// deletes and holds the write lock briefly.
    pub fn purge_expired(&self, now: u64, limit: usize) -> Result<usize, StoreError> {
        let store = &self.entries.table.store;
        // Inclusive of `now`: replay treats an entry as gone at that moment.
        let upper = now.to_be_bytes();
        let expired = store
            .read(|tx| tx.index_range(&self.entries, ..=upper.as_slice(), Direction::Asc, limit))?;
        store.write(|tx| {
            let mut purged = 0;
            for Entry { key, .. } in &expired {
                // A key can be recorded again after the snapshot above.
                if tx
                    .get(&self.entries, key)?
                    .is_some_and(|entry| entry.expires_at <= now)
                {
                    tx.remove(&self.entries, key)?;
                    purged += 1;
                }
            }
            Ok::<_, StoreError>(purged)
        })
    }
}
