//! Tenant-scoped SQLite application services for the Orders reference.
//!
//! Callers resolve a trusted principal before entering these services. Drafts
//! never choose a tenant, price, line identity or saved revision.

use crate::{
    auth::Principal,
    domain::{
        DeleteError, Field, FieldError, LineItem, LoadError, Order, OrderDraft, OrderPage,
        PAGE_SIZE, SaveError, new_order, unsigned, validate,
    },
};
use sqlx::{
    SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{io, path::Path, time::Duration};

pub use crate::store::StoreError;
use crate::store::{UserRecord, fresh_generation, valid_identity_name, valid_order_record};
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Debug)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub async fn export_snapshot(
        &self,
        principal: &Principal,
        max_rows: usize,
    ) -> Result<Vec<Order>, StoreError> {
        let mut tx = self.pool.begin().await?;
        let limit = i64::try_from(max_rows)?
            .checked_add(1)
            .ok_or_else(|| invalid_data("Export limit is too large"))?;
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM orders WHERE tenant_id = ? ORDER BY id DESC LIMIT ?",
        )
        .bind(principal.key.tenant_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;
        if ids.len() > max_rows {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Export limit exceeded",
            )
            .into());
        }
        let mut orders = Vec::with_capacity(ids.len());
        for id in ids {
            orders.push(
                load_order(&mut tx, principal.key.tenant_id, id)
                    .await?
                    .ok_or_else(|| invalid_data("Export order disappeared"))?,
            );
        }
        tx.commit().await?;
        Ok(orders)
    }

    /// Wraps an already configured and migrated pool, shared with authentication.
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }

    pub async fn generation(&self) -> Result<u64, StoreError> {
        let generation: String =
            sqlx::query_scalar("SELECT value FROM database_metadata WHERE name = 'generation'")
                .fetch_one(&self.pool)
                .await?;
        parse_generation(&generation)
    }

    pub async fn rotate_generation(&self) -> Result<u64, StoreError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let previous: String =
            sqlx::query_scalar("SELECT value FROM database_metadata WHERE name = 'generation'")
                .fetch_one(&mut *tx)
                .await?;
        let generation = fresh_generation(parse_generation(&previous)?)?;
        sqlx::query("UPDATE database_metadata SET value = ? WHERE name = 'generation'")
            .bind(generation.to_string())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(generation)
    }

    pub async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserRecord>, StoreError> {
        let row = sqlx::query_as::<_, (i64, i64, String, String, String, bool, i64)>(
            "SELECT id, tenant_id, username, display_name, password_hash, enabled, session_epoch FROM users WHERE username = ?",
        ).bind(username).fetch_optional(&self.pool).await?;
        Ok(row.map(user_record))
    }

    pub async fn find_user_by_id(&self, id: i64) -> Result<Option<UserRecord>, StoreError> {
        let row = sqlx::query_as::<_, (i64, i64, String, String, String, bool, i64)>(
            "SELECT id, tenant_id, username, display_name, password_hash, enabled, session_epoch FROM users WHERE id = ?",
        ).bind(id).fetch_optional(&self.pool).await?;
        Ok(row.map(user_record))
    }

    pub async fn insert_user(
        &self,
        tenant: &str,
        username: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<Principal, StoreError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        sqlx::query("INSERT INTO tenants(name) VALUES (?) ON CONFLICT(name) DO NOTHING")
            .bind(tenant)
            .execute(&mut *tx)
            .await?;
        let tenant_id: i64 = sqlx::query_scalar("SELECT id FROM tenants WHERE name = ?")
            .bind(tenant)
            .fetch_one(&mut *tx)
            .await?;
        let id = sqlx::query("INSERT INTO users(tenant_id, username, display_name, password_hash) VALUES (?, ?, ?, ?)")
            .bind(tenant_id).bind(username).bind(display_name).bind(password_hash)
            .execute(&mut *tx).await?.last_insert_rowid();
        let generation: String =
            sqlx::query_scalar("SELECT value FROM database_metadata WHERE name = 'generation'")
                .fetch_one(&mut *tx)
                .await?;
        let generation = parse_generation(&generation)?;
        tx.commit().await?;
        Ok(Principal {
            key: crate::auth::PrincipalKey {
                user_id: id,
                tenant_id,
                generation,
            },
            display_name: display_name.into(),
        })
    }

    pub async fn revoke_sessions(&self, username: &str) -> Result<bool, StoreError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current: Option<i64> =
            sqlx::query_scalar("SELECT session_epoch FROM users WHERE username = ?")
                .bind(username)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(current) = current else {
            return Ok(false);
        };
        let next = current
            .checked_add(1)
            .ok_or_else(|| invalid_data("Session epoch exhausted"))?;
        sqlx::query("UPDATE users SET session_epoch = ? WHERE username = ?")
            .bind(next)
            .bind(username)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn set_user_enabled(
        &self,
        username: &str,
        enabled: bool,
    ) -> Result<bool, StoreError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let current: Option<(bool, i64)> =
            sqlx::query_as("SELECT enabled, session_epoch FROM users WHERE username = ?")
                .bind(username)
                .fetch_optional(&mut *tx)
                .await?;
        let Some((was_enabled, epoch)) = current else {
            return Ok(false);
        };
        if was_enabled != enabled {
            let next = epoch
                .checked_add(1)
                .ok_or_else(|| invalid_data("Session epoch exhausted"))?;
            sqlx::query("UPDATE users SET enabled = ?, session_epoch = ? WHERE username = ?")
                .bind(enabled)
                .bind(next)
                .bind(username)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(true)
    }

    /// Opens durable local storage and applies migrations before serving traffic.
    /// Creating or reopening a database never inserts demonstration records.
    pub async fn open(path: &Path) -> Result<Self, StoreError> {
        let mut file = std::fs::OpenOptions::new();
        file.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            file.mode(0o600);
        }
        match file.open(path) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let options = SqliteConnectOptions::new()
            .filename(path)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await?;
        if let Err(error) = MIGRATOR.run(&pool).await {
            pool.close().await;
            return Err(error.into());
        }
        let initialization: Result<(), StoreError> = async {
            let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
            let current: Option<String> =
                sqlx::query_scalar("SELECT value FROM database_metadata WHERE name = 'generation'")
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some(current) = current {
                parse_generation(&current)?;
            } else {
                sqlx::query("INSERT INTO database_metadata(name, value) VALUES ('generation', ?)")
                    .bind(fresh_generation(0)?.to_string())
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
            Ok(())
        }
        .await;
        if let Err(error) = initialization {
            pool.close().await;
            return Err(error);
        }
        Ok(Self::from_pool(pool))
    }

    pub async fn health(&self) -> Result<(), StoreError> {
        sqlx::query_scalar::<_, i64>("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn load_by_id(
        &self,
        principal: &Principal,
        id: u64,
    ) -> Result<Option<Order>, LoadError> {
        let Some(id) = database_id(id) else {
            return Ok(None);
        };
        let mut tx = self.pool.begin().await.map_err(|_| LoadError)?;
        let order = load_order(&mut tx, principal.key.tenant_id, id)
            .await
            .map_err(|_| LoadError)?;
        tx.commit().await.map_err(|_| LoadError)?;
        Ok(order)
    }

    /// Preserves Unicode-lowercase substring search and bounded five-row pages.
    /// Count, clamping and selected rows share a single read snapshot. The tenant
    /// and ID index supports traversal; arbitrary substring matching still scans.
    pub async fn list(
        &self,
        principal: &Principal,
        query: String,
        page: u32,
    ) -> Result<OrderPage, LoadError> {
        let query = query.trim().to_owned();
        let needle = query.to_lowercase();
        let tenant = principal.key.tenant_id;
        let mut tx = self.pool.begin().await.map_err(|_| LoadError)?;
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM orders WHERE tenant_id = ? AND instr(customer_search, ?) > 0",
        )
        .bind(tenant)
        .bind(&needle)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| LoadError)?;
        let total_count = u64::try_from(count).map_err(|_| LoadError)?;
        let total_pages = u32::try_from(total_count.div_ceil(u64::from(PAGE_SIZE)).max(1))
            .map_err(|_| LoadError)?;
        let page = page.clamp(1, total_pages);
        let offset = i64::from(page - 1) * i64::from(PAGE_SIZE);
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM orders WHERE tenant_id = ? AND instr(customer_search, ?) > 0 \
             ORDER BY id DESC LIMIT ? OFFSET ?",
        )
        .bind(tenant)
        .bind(&needle)
        .bind(i64::from(PAGE_SIZE))
        .bind(offset)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_| LoadError)?;
        let mut orders = Vec::with_capacity(ids.len());
        for id in ids {
            orders.push(
                load_order(&mut tx, tenant, id)
                    .await
                    .map_err(|_| LoadError)?
                    .ok_or(LoadError)?,
            );
        }
        tx.commit().await.map_err(|_| LoadError)?;
        Ok(OrderPage {
            orders,
            query,
            page,
            total_pages,
            total_count,
        })
    }

    pub async fn create(
        &self,
        principal: &Principal,
        draft: OrderDraft,
    ) -> Result<Order, SaveError> {
        let input = validate(&draft).map_err(|errors| SaveError::Validation {
            draft: draft.clone(),
            errors,
        })?;
        if input.expected_revision != 1 {
            return Err(SaveError::Validation {
                draft,
                errors: vec![FieldError {
                    field: Field::Revision,
                    message: "A new order must start at revision 1".into(),
                }],
            });
        }
        let mut order = new_order();
        order.customer = input.customer;
        order.items[0].quantity = input.quantity_one;
        order.items[1].quantity = input.quantity_two;
        checked_total(&order, &draft)?;
        let unavailable = |_| SaveError::Unavailable {
            draft: draft.clone(),
        };
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(unavailable)?;
        insert_order(&mut tx, principal.key.tenant_id, &mut order)
            .await
            .map_err(unavailable)?;
        tx.commit().await.map_err(unavailable)?;
        Ok(order)
    }

    /// Checks the expected revision and updates the complete order atomically.
    /// A different tenant's ID is indistinguishable from an absent order.
    pub async fn save_by_id(
        &self,
        principal: &Principal,
        id: u64,
        draft: OrderDraft,
    ) -> Result<Order, SaveError> {
        let input = validate(&draft).map_err(|errors| SaveError::Validation {
            draft: draft.clone(),
            errors,
        })?;
        let Some(id) = database_id(id) else {
            return Err(SaveError::NotFound { draft });
        };
        let tenant = principal.key.tenant_id;
        let unavailable = |_| SaveError::Unavailable {
            draft: draft.clone(),
        };
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(unavailable)?;
        let Some(mut order) = load_order(&mut tx, tenant, id).await.map_err(unavailable)? else {
            return Err(SaveError::NotFound { draft });
        };
        if order.revision != input.expected_revision {
            return Err(SaveError::Conflict {
                draft,
                current: Box::new(order),
            });
        }
        let expected = i64::try_from(order.revision).map_err(|_| SaveError::Unavailable {
            draft: draft.clone(),
        })?;
        let next = expected
            .checked_add(1)
            .ok_or_else(|| SaveError::Unavailable {
                draft: draft.clone(),
            })?;
        order.customer = input.customer;
        order.items[0].quantity = input.quantity_one;
        order.items[1].quantity = input.quantity_two;
        order.revision = next as u64;
        checked_total(&order, &draft)?;
        let changed = sqlx::query(
            "UPDATE orders SET customer = ?, customer_search = ?, revision = ? \
             WHERE tenant_id = ? AND id = ? AND revision = ?",
        )
        .bind(&order.customer)
        .bind(order.customer.to_lowercase())
        .bind(next)
        .bind(tenant)
        .bind(id)
        .bind(expected)
        .execute(&mut *tx)
        .await
        .map_err(unavailable)?;
        if changed.rows_affected() != 1 {
            return Err(SaveError::Unavailable { draft });
        }
        for (slot, item) in order.items.iter().enumerate() {
            let changed = sqlx::query(
                "UPDATE order_lines SET quantity = ? WHERE order_id = ? AND slot = ? \
                 AND EXISTS (SELECT 1 FROM orders WHERE id = order_lines.order_id AND tenant_id = ?)",
            )
            .bind(i64::from(item.quantity))
            .bind(id)
            .bind(slot as i64)
            .bind(tenant)
            .execute(&mut *tx)
            .await
            .map_err(unavailable)?;
            if changed.rows_affected() != 1 {
                return Err(SaveError::Unavailable { draft });
            }
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(order)
    }

    pub async fn delete(
        &self,
        principal: &Principal,
        id: u64,
        expected_revision: String,
    ) -> Result<(), DeleteError> {
        let Some(revision) = unsigned(&expected_revision).filter(|revision| *revision > 0) else {
            return Err(DeleteError::Validation {
                expected_revision,
                message: "Reload the order to obtain a valid revision".into(),
            });
        };
        let Some(id) = database_id(id) else {
            return Err(DeleteError::NotFound { expected_revision });
        };
        let tenant = principal.key.tenant_id;
        let unavailable = |_| DeleteError::Unavailable {
            expected_revision: expected_revision.clone(),
        };
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(unavailable)?;
        let Some(current) = load_order(&mut tx, tenant, id).await.map_err(unavailable)? else {
            return Err(DeleteError::NotFound { expected_revision });
        };
        if current.revision != revision {
            return Err(DeleteError::Conflict {
                expected_revision,
                current: Box::new(current),
            });
        }
        let removed =
            sqlx::query("DELETE FROM orders WHERE tenant_id = ? AND id = ? AND revision = ?")
                .bind(tenant)
                .bind(id)
                .bind(
                    i64::try_from(revision).map_err(|_| DeleteError::Unavailable {
                        expected_revision: expected_revision.clone(),
                    })?,
                )
                .execute(&mut *tx)
                .await
                .map_err(unavailable)?;
        if removed.rows_affected() != 1 {
            return Err(DeleteError::Unavailable { expected_revision });
        }
        tx.commit().await.map_err(unavailable)?;
        Ok(())
    }

    /// Explicit CLI/test fixture operation. Existing tenant records are untouched.
    pub async fn seed_orders(&self, principal: &Principal) -> Result<(), StoreError> {
        let tenant = principal.key.tenant_id;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM orders WHERE tenant_id = ?)")
                .bind(tenant)
                .fetch_one(&mut *tx)
                .await?;
        if !exists {
            for (index, customer) in [
                "Ada Lovelace",
                "Grace Hopper",
                "Alan Turing",
                "Katherine Johnson",
                "Margaret Hamilton",
                "Dorothy Vaughan",
                "Mary Jackson",
                "Annie Easley",
                "Evelyn Boyd Granville",
                "Joan Clarke",
                "Frances Allen",
                "Barbara Liskov",
            ]
            .into_iter()
            .enumerate()
            {
                let mut order = new_order();
                order.customer = customer.into();
                if index == 0 {
                    order.items[0].quantity = 2;
                }
                insert_order(&mut tx, tenant, &mut order).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    /// Makes a consistent local backup without copying an active WAL database.
    /// Removes session credentials and recovery drafts from the snapshot only.
    /// Refuses an existing destination, including an existing empty file.
    pub async fn backup_to(&self, path: &Path) -> Result<(), StoreError> {
        let filename = path
            .to_str()
            .ok_or_else(|| invalid_data("Backup path must be UTF-8"))?;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let destination = options.open(path)?;
        drop(destination);
        let result: Result<(), StoreError> = async {
            sqlx::query("VACUUM INTO ?")
                .bind(filename)
                .execute(&self.pool)
                .await?;
            scrub_backup_sessions(path).await?;
            std::fs::OpenOptions::new()
                .write(true)
                .open(path)?
                .sync_all()?;
            #[cfg(unix)]
            {
                let parent = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new("."));
                std::fs::File::open(parent)?.sync_all()?;
            }
            Ok(())
        }
        .await;
        if let Err(error) = result {
            let _ = std::fs::remove_file(path);
            return Err(error);
        }
        Ok(())
    }

    /// Checks a restore source without changing it. Restore while the server is
    /// stopped, then invalidate restored sessions before accepting requests.
    pub async fn validate_backup(path: &Path) -> Result<(), StoreError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let result = validate_database(&pool).await;
        pool.close().await;
        result
    }
}

async fn scrub_backup_sessions(path: &Path) -> Result<(), StoreError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .journal_mode(SqliteJournalMode::Delete)
                .synchronous(SqliteSynchronous::Full),
        )
        .await?;
    let result: Result<(), StoreError> = async {
        let has_sessions: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'tower_sessions')",
        )
        .fetch_one(&pool)
        .await?;
        for table in ["export_results", "export_jobs"] {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?)")
                .bind(table).fetch_one(&pool).await?;
            if exists { sqlx::query(&format!("DELETE FROM {table}")).execute(&pool).await?; }
        }
        if has_sessions {
            sqlx::query("DELETE FROM tower_sessions").execute(&pool).await?;
            // Rebuild the copy so deleted bearer IDs and draft bytes are not
            // left in free pages. The live database and its sessions are intact.
        }
        sqlx::query("VACUUM").execute(&pool).await?;
        Ok(())
    }
    .await;
    pool.close().await;
    result
}

fn database_id(id: u64) -> Option<i64> {
    i64::try_from(id).ok().filter(|id| *id > 0)
}

fn user_record(row: (i64, i64, String, String, String, bool, i64)) -> UserRecord {
    let (id, tenant_id, username, display_name, password_hash, enabled, session_epoch) = row;
    UserRecord {
        id,
        tenant_id,
        username,
        display_name,
        password_hash,
        enabled,
        session_epoch,
    }
}

fn checked_total(order: &Order, draft: &OrderDraft) -> Result<(), SaveError> {
    order
        .total_cents()
        .map(|_| ())
        .map_err(|error| SaveError::Validation {
            draft: draft.clone(),
            errors: vec![FieldError {
                field: Field::Order,
                message: error.to_string(),
            }],
        })
}

fn invalid_data(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn parse_generation(value: &str) -> Result<u64, StoreError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value != 0)
        .ok_or_else(|| invalid_data("Invalid database generation").into())
}

fn decode_number(value: i64) -> Result<u64, sqlx::Error> {
    u64::try_from(value).map_err(|error| sqlx::Error::Decode(Box::new(error)))
}

async fn load_order(
    connection: &mut SqliteConnection,
    tenant: i64,
    id: i64,
) -> Result<Option<Order>, sqlx::Error> {
    let Some((id, revision, customer)) = sqlx::query_as::<_, (i64, i64, String)>(
        "SELECT id, revision, customer FROM orders WHERE tenant_id = ? AND id = ?",
    )
    .bind(tenant)
    .bind(id)
    .fetch_optional(&mut *connection)
    .await?
    else {
        return Ok(None);
    };
    let lines = sqlx::query_as::<_, (i64, String, i64, i64)>(
        "SELECT l.catalog_id, l.name, l.unit_price_cents, l.quantity FROM order_lines l \
         JOIN orders o ON o.id = l.order_id WHERE o.tenant_id = ? AND o.id = ? ORDER BY l.slot",
    )
    .bind(tenant)
    .bind(id)
    .fetch_all(&mut *connection)
    .await?;
    let items = lines
        .into_iter()
        .map(|(id, name, price, quantity)| {
            Ok(LineItem {
                id: decode_number(id)?,
                name,
                unit_price_cents: decode_number(price)?,
                quantity: u32::try_from(quantity)
                    .map_err(|error| sqlx::Error::Decode(Box::new(error)))?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;
    let items = items.try_into().map_err(|_| {
        sqlx::Error::Decode(Box::new(invalid_data(
            "An order must have exactly two lines",
        )))
    })?;
    Ok(Some(Order {
        id: decode_number(id)?,
        revision: decode_number(revision)?,
        customer,
        items,
    }))
}

async fn insert_order(
    connection: &mut SqliteConnection,
    tenant: i64,
    order: &mut Order,
) -> Result<(), sqlx::Error> {
    let id = sqlx::query(
        "INSERT INTO orders (tenant_id, revision, customer, customer_search) VALUES (?, 1, ?, ?)",
    )
    .bind(tenant)
    .bind(&order.customer)
    .bind(order.customer.to_lowercase())
    .execute(&mut *connection)
    .await?
    .last_insert_rowid();
    order.id = decode_number(id)?;
    for (slot, item) in order.items.iter().enumerate() {
        let result = sqlx::query(
            "INSERT INTO order_lines (order_id, slot, catalog_id, name, unit_price_cents, quantity) \
             SELECT id, ?, ?, ?, ?, ? FROM orders WHERE tenant_id = ? AND id = ?",
        )
        .bind(slot as i64)
        .bind(i64::try_from(item.id).map_err(|error| sqlx::Error::Encode(Box::new(error)))?)
        .bind(&item.name)
        .bind(i64::try_from(item.unit_price_cents).map_err(|error| sqlx::Error::Encode(Box::new(error)))?)
        .bind(i64::from(item.quantity))
        .bind(tenant)
        .bind(id)
        .execute(&mut *connection)
        .await?;
        if result.rows_affected() != 1 {
            return Err(sqlx::Error::RowNotFound);
        }
    }
    Ok(())
}

async fn validate_database(pool: &SqlitePool) -> Result<(), StoreError> {
    let integrity: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_all(pool)
        .await?;
    if integrity != ["ok"] {
        return Err(invalid_data("Database integrity check failed").into());
    }
    if !sqlx::query("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await?
        .is_empty()
    {
        return Err(invalid_data("Database foreign key check failed").into());
    }
    let applied = sqlx::query_as::<_, (i64, bool, Vec<u8>)>(
        "SELECT version, success, checksum FROM _sqlx_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await?;
    if applied.is_empty() {
        return Err(invalid_data("Database has no application migrations").into());
    }
    let requires_generation = applied.iter().any(|(version, _, _)| *version >= 3);
    let requires_attachments = applied.iter().any(|(version, _, _)| *version >= 4);
    for (version, success, checksum) in applied {
        let known = MIGRATOR
            .iter()
            .find(|migration| migration.version == version);
        if !success || !known.is_some_and(|migration| migration.checksum.as_ref() == checksum) {
            return Err(invalid_data("Database migration history is incompatible").into());
        }
    }
    if requires_generation {
        let generation: String =
            sqlx::query_scalar("SELECT value FROM database_metadata WHERE name = 'generation'")
                .fetch_one(pool)
                .await?;
        parse_generation(&generation)?;
    }
    crate::attachments::sqlite_validate(pool, requires_attachments).await?;
    let tenants: Vec<String> = sqlx::query_scalar("SELECT name FROM tenants")
        .fetch_all(pool)
        .await?;
    if tenants.iter().any(|name| !valid_identity_name(name)) {
        return Err(invalid_data("Invalid tenant name in backup").into());
    }
    let users: Vec<(String, String, i64)> =
        sqlx::query_as("SELECT username, display_name, session_epoch FROM users")
            .fetch_all(pool)
            .await?;
    if users.iter().any(|(username, display, epoch)| {
        !valid_identity_name(username) || !valid_identity_name(display) || *epoch < 0
    }) {
        return Err(invalid_data("Invalid user record in backup").into());
    }
    let orders: Vec<(i64, i64, String)> =
        sqlx::query_as("SELECT id, tenant_id, customer_search FROM orders")
            .fetch_all(pool)
            .await?;
    let mut connection = pool.acquire().await?;
    for (id, tenant, search) in orders {
        let order = load_order(&mut connection, tenant, id)
            .await?
            .ok_or_else(|| invalid_data("Missing order in backup"))?;
        if !valid_order_record(&order) || order.customer.to_lowercase() != search {
            return Err(invalid_data("Invalid order record or catalogue in backup").into());
        }
    }
    Ok(())
}
