//! Concrete storage choices for the Orders reference application.
//!
//! The embedded redb store is the default. SQLite is an explicit optional
//! adapter; both preserve the same tenant-scoped application service contract.

use crate::{
    auth::Principal,
    domain::{DeleteError, LoadError, Order, OrderDraft, OrderPage, SaveError},
    redb_store::RedbStore,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, path::Path, sync::Arc};

pub type StoreError = Box<dyn Error + Send + Sync>;

/// Persisted identity data. Deliberately omits Debug to avoid logging hashes.
#[derive(Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: i64,
    pub tenant_id: i64,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub enabled: bool,
    pub session_epoch: i64,
}

#[derive(Clone, Debug)]
pub struct OrderStore {
    backend: Backend,
}

#[derive(Clone, Debug)]
enum Backend {
    Redb(RedbStore),
    #[cfg(feature = "sqlite")]
    Sqlite(crate::sqlite_store::SqliteStore),
}

macro_rules! dispatch {
    ($self:expr, $method:ident($($argument:expr),* $(,)?)) => {
        match &$self.backend {
            Backend::Redb(store) => store.$method($($argument),*).await,
            #[cfg(feature = "sqlite")]
            Backend::Sqlite(store) => store.$method($($argument),*).await,
        }
    };
}

impl OrderStore {
    /// A bounded, consistent snapshot for an authorized tenant export.
    pub async fn export_snapshot(
        &self,
        principal: &Principal,
        max_rows: usize,
    ) -> Result<Vec<Order>, StoreError> {
        dispatch!(self, export_snapshot(principal, max_rows))
    }

    /// Identifies this database history, including across a restored snapshot.
    pub async fn generation(&self) -> Result<u64, StoreError> {
        dispatch!(self, generation())
    }

    /// Invalidates identities retained from an older database history.
    pub async fn rotate_generation(&self) -> Result<u64, StoreError> {
        dispatch!(self, rotate_generation())
    }

    /// Opens or initializes the default embedded NoSQL database without seeding.
    /// Operators must explicitly authorize creation before calling this method.
    pub async fn open(path: &Path) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::Redb(RedbStore::open(path).await?),
        })
    }

    #[cfg(feature = "sqlite")]
    pub async fn open_sqlite(path: &Path) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::Sqlite(crate::sqlite_store::SqliteStore::open(path).await?),
        })
    }

    /// Invalidates shared service handles. Drop the router and session adapter
    /// before reopening redb, because they can own additional database handles.
    pub async fn close(&self) {
        dispatch!(self, close())
    }

    pub(crate) fn redb_database(&self) -> Option<Arc<redb::Database>> {
        match &self.backend {
            Backend::Redb(store) => store.database(),
            #[cfg(feature = "sqlite")]
            Backend::Sqlite(_) => None,
        }
    }

    #[cfg(feature = "sqlite")]
    pub(crate) fn sqlite_pool(&self) -> Option<sqlx::SqlitePool> {
        match &self.backend {
            Backend::Redb(_) => None,
            Backend::Sqlite(store) => Some(store.pool().clone()),
        }
    }

    pub async fn health(&self) -> Result<(), StoreError> {
        dispatch!(self, health())
    }
    pub async fn load_by_id(
        &self,
        principal: &Principal,
        id: u64,
    ) -> Result<Option<Order>, LoadError> {
        dispatch!(self, load_by_id(principal, id))
    }
    pub async fn list(
        &self,
        principal: &Principal,
        query: String,
        page: u32,
    ) -> Result<OrderPage, LoadError> {
        dispatch!(self, list(principal, query, page))
    }
    pub async fn create(
        &self,
        principal: &Principal,
        draft: OrderDraft,
    ) -> Result<Order, SaveError> {
        dispatch!(self, create(principal, draft))
    }
    pub async fn save_by_id(
        &self,
        principal: &Principal,
        id: u64,
        draft: OrderDraft,
    ) -> Result<Order, SaveError> {
        dispatch!(self, save_by_id(principal, id, draft))
    }
    pub async fn delete(
        &self,
        principal: &Principal,
        id: u64,
        expected_revision: String,
    ) -> Result<(), DeleteError> {
        dispatch!(self, delete(principal, id, expected_revision))
    }
    pub async fn seed_orders(&self, principal: &Principal) -> Result<(), StoreError> {
        dispatch!(self, seed_orders(principal))
    }
    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserRecord>, StoreError> {
        dispatch!(self, find_user_by_username(username))
    }
    pub async fn find_user_by_id(&self, id: i64) -> Result<Option<UserRecord>, StoreError> {
        dispatch!(self, find_user_by_id(id))
    }
    pub async fn insert_user(
        &self,
        tenant: &str,
        username: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<Principal, StoreError> {
        dispatch!(
            self,
            insert_user(tenant, username, display_name, password_hash)
        )
    }
    pub async fn revoke_sessions(&self, username: &str) -> Result<bool, StoreError> {
        dispatch!(self, revoke_sessions(username))
    }
    pub async fn set_user_enabled(
        &self,
        username: &str,
        enabled: bool,
    ) -> Result<bool, StoreError> {
        dispatch!(self, set_user_enabled(username, enabled))
    }
    pub async fn backup_to(&self, path: &Path) -> Result<(), StoreError> {
        dispatch!(self, backup_to(path))
    }
    pub async fn validate_backup(path: &Path) -> Result<(), StoreError> {
        RedbStore::validate_backup(path).await
    }
    #[cfg(feature = "sqlite")]
    pub async fn validate_sqlite_backup(path: &Path) -> Result<(), StoreError> {
        crate::sqlite_store::SqliteStore::validate_backup(path).await
    }
}

pub(crate) fn fresh_generation(previous: u64) -> Result<u64, StoreError> {
    loop {
        let mut bytes = [0; 8];
        getrandom::fill(&mut bytes).map_err(|error| {
            std::io::Error::other(format!(
                "Database generation randomness unavailable: {error}"
            ))
        })?;
        let generation = u64::from_le_bytes(bytes);
        if generation != 0 && generation != previous {
            return Ok(generation);
        }
    }
}

pub(crate) fn valid_identity_name(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 128
}

pub(crate) fn valid_order_record(order: &Order) -> bool {
    let valid_numbers = order.id > 0
        && order.id <= i64::MAX as u64
        && order.revision > 0
        && order.revision <= i64::MAX as u64;
    let valid_fields =
        crate::domain::validate(&order.draft()).is_ok_and(|input| input.customer == order.customer);
    let catalogue = crate::domain::new_order();
    let valid_catalogue = order
        .items
        .iter()
        .zip(catalogue.items)
        .all(|(actual, expected)| {
            actual.id == expected.id
                && actual.name == expected.name
                && actual.unit_price_cents == expected.unit_price_cents
        });
    valid_numbers && valid_fields && valid_catalogue && order.total_cents().is_ok()
}
