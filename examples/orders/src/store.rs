//! Reads and writes from async handlers. redb is blocking, so each call
//! runs on Tokio's blocking pool. Writes take one lock, so a read, check
//! and write (the version check, the next id) can't interleave with
//! another; this is a single-process demo, not a scalable write path.

use std::sync::{Arc, Mutex};

use stucco_redb::{IndexTable, StoreError};

use crate::model::{Order, OrderInput};

pub(crate) type OrdersTable = IndexTable<u64, Order>;

#[derive(Clone)]
pub(crate) struct Orders {
    table: OrdersTable,
    writes: Arc<Mutex<()>>,
}

/// The result of saving an edit.
pub(crate) enum Update {
    Saved(Order),
    /// Someone saved the order after this edit started; here is theirs.
    Stale(Order),
    Missing,
    /// Archived orders are read-only.
    Archived(Order),
}

impl Orders {
    pub fn new(table: OrdersTable) -> Orders {
        Orders {
            table,
            writes: Arc::new(Mutex::new(())),
        }
    }

    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(&OrdersTable) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        let table = self.table.clone();
        let writes = self.writes.clone();
        tokio::task::spawn_blocking(move || {
            let _guard = writes.lock().unwrap_or_else(|e| e.into_inner());
            work(&table)
        })
        .await
        .map_err(StoreError::new)?
    }

    pub async fn get(&self, id: u64) -> Result<Option<Order>, StoreError> {
        self.blocking(move |t| t.get(&id)).await
    }

    /// The orders among `ids` that still exist, in the order given.
    pub async fn get_many(&self, ids: Vec<u64>) -> Result<Vec<Order>, StoreError> {
        self.blocking(move |t| {
            let mut found = Vec::new();
            for id in ids {
                if let Some(order) = t.get(&id)? {
                    found.push(order);
                }
            }
            Ok(found)
        })
        .await
    }

    pub async fn create(&self, input: OrderInput) -> Result<Order, StoreError> {
        self.blocking(move |t| {
            // Reads every key: fine for a demo, not for a large table, where
            // a counter or the storage's own sequence belongs.
            let id = t.all(false)?.iter().map(|(_, o)| o.id).max().unwrap_or(0) + 1;
            let mut order = Order {
                id,
                customer: String::new(),
                status: String::new(),
                total_cents: 0,
                created: String::new(),
                priority: false,
                note: String::new(),
                version: 1,
                restore_status: None,
                history: Vec::new(),
            };
            input.apply(&mut order);
            order.record("created the order", None);
            t.put(&id, &order)?;
            Ok(order)
        })
        .await
    }

    /// Saves `input` over the order if it is still at `version`.
    pub async fn update(
        &self,
        id: u64,
        version: u64,
        input: OrderInput,
    ) -> Result<Update, StoreError> {
        self.blocking(move |t| {
            let Some(mut order) = t.get(&id)? else {
                return Ok(Update::Missing);
            };
            if order.archived() {
                return Ok(Update::Archived(order));
            }
            if order.version != version {
                return Ok(Update::Stale(order));
            }
            let changes = input.changes(&order);
            if changes.is_empty() {
                return Ok(Update::Saved(order));
            }
            input.apply(&mut order);
            order.version += 1;
            order.record(
                "edited the order",
                Some(format!("Changed {}", changes.join(", "))),
            );
            t.put(&id, &order)?;
            Ok(Update::Saved(order))
        })
        .await
    }

    /// Archives or restores each of `ids` that exists and isn't already in
    /// that state; returns the orders changed.
    pub async fn set_archived(
        &self,
        ids: Vec<u64>,
        archived: bool,
    ) -> Result<Vec<Order>, StoreError> {
        self.blocking(move |t| {
            let mut changed = Vec::new();
            for id in ids {
                let Some(mut order) = t.get(&id)? else {
                    continue;
                };
                if order.archived() == archived {
                    continue;
                }
                if archived {
                    order.restore_status =
                        Some(std::mem::replace(&mut order.status, "archived".into()));
                    order.record("archived the order", None);
                } else {
                    order.status = order
                        .restore_status
                        .take()
                        .unwrap_or_else(|| "pending".into());
                    order.record("restored the order", None);
                }
                order.version += 1;
                t.put(&id, &order)?;
                changed.push(order);
            }
            Ok(changed)
        })
        .await
    }

    /// Deletes each of `ids` that exists; returns how many were deleted.
    pub async fn delete(&self, ids: Vec<u64>) -> Result<usize, StoreError> {
        self.blocking(move |t| {
            let mut deleted = 0;
            for id in ids {
                if t.remove(&id)? {
                    deleted += 1;
                }
            }
            Ok(deleted)
        })
        .await
    }
}
