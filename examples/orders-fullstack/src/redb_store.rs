//! Application-owned redb transactions; no generic persistence framework.

use crate::{
    auth::{Principal, PrincipalKey},
    domain::{
        DeleteError, Field, FieldError, LoadError, Order, OrderDraft, OrderPage, PAGE_SIZE,
        SaveError, new_order, unsigned, validate,
    },
    store::{StoreError, UserRecord, fresh_generation, valid_identity_name, valid_order_record},
};
use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition, TableHandle,
};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::Semaphore;

const SCHEMA_VERSION: u64 = 4;
const META: TableDefinition<&str, u64> = TableDefinition::new("metadata");
const ORDERS: TableDefinition<u64, &[u8]> = TableDefinition::new("orders");
const TENANT_ORDERS: TableDefinition<(i64, u64), ()> = TableDefinition::new("orders_by_tenant");
const TENANTS: TableDefinition<i64, &str> = TableDefinition::new("tenants");
const TENANT_NAMES: TableDefinition<&str, i64> = TableDefinition::new("tenants_by_name");
const USERS: TableDefinition<i64, &[u8]> = TableDefinition::new("users");
const USER_NAMES: TableDefinition<&str, i64> = TableDefinition::new("users_by_username");
const SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");

#[derive(Clone, Serialize, Deserialize)]
struct StoredOrder {
    tenant_id: i64,
    order: Order,
}

#[derive(Clone, Debug)]
pub(crate) struct RedbStore {
    database: Arc<Mutex<Option<Arc<Database>>>>,
    permits: Arc<Semaphore>,
    identity_permits: Arc<Semaphore>,
}

impl RedbStore {
    pub(crate) async fn export_snapshot(
        &self,
        principal: &Principal,
        max_rows: usize,
    ) -> Result<Vec<Order>, StoreError> {
        let tenant = principal.key.tenant_id;
        self.run(move |database| {
            let read = database.begin_read()?;
            let index = read.open_table(TENANT_ORDERS)?;
            let records = read.open_table(ORDERS)?;
            let mut orders = Vec::new();
            for entry in index.range((tenant, 0)..=(tenant, u64::MAX))?.rev() {
                if orders.len() == max_rows {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Export limit exceeded",
                    )
                    .into());
                }
                let (key, _) = entry?;
                let order = decode_scoped(
                    records
                        .get(key.value().1)?
                        .as_ref()
                        .map(|value| value.value()),
                    tenant,
                )?
                .ok_or_else(|| invalid("Order index does not match its record"))?;
                orders.push(order);
            }
            Ok(orders)
        })
        .await
    }

    pub(crate) async fn open(path: &Path) -> Result<Self, StoreError> {
        let path = path.to_owned();
        let database = tokio::task::spawn_blocking(move || -> Result<Database, StoreError> {
            let database = match private_file(&path) {
                Ok(file) => Database::builder().create_file(file)?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    Database::open(&path)?
                }
                Err(error) => return Err(error.into()),
            };
            migrate(&database)?;
            Ok(database)
        })
        .await??;
        Ok(Self {
            database: Arc::new(Mutex::new(Some(Arc::new(database)))),
            permits: Arc::new(Semaphore::new(4)),
            identity_permits: Arc::new(Semaphore::new(4)),
        })
    }

    pub(crate) fn database(&self) -> Option<Arc<Database>> {
        self.database.lock().ok()?.clone()
    }

    pub(crate) async fn close(&self) {
        let Ok(_permits) = self.permits.clone().acquire_many_owned(4).await else {
            return;
        };
        let Ok(_identity_permits) = self.identity_permits.clone().acquire_many_owned(4).await
        else {
            return;
        };
        if let Ok(mut database) = self.database.lock() {
            database.take();
        }
    }

    async fn run<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Database) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        self.run_with(self.permits.clone(), work).await
    }

    async fn run_identity<T: Send + 'static>(
        &self,
        work: impl FnOnce(&Database) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        self.run_with(self.identity_permits.clone(), work).await
    }

    async fn run_with<T: Send + 'static>(
        &self,
        permits: Arc<Semaphore>,
        work: impl FnOnce(&Database) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, StoreError> {
        // Wait within a deadline before starting bounded blocking work. Identity
        // reads have independent capacity. Cancellation cannot cancel a commit.
        let permit = tokio::time::timeout(Duration::from_secs(5), permits.acquire_owned())
            .await
            .map_err(|_| invalid("Storage is busy"))?
            .map_err(|_| invalid("Storage is closed"))?;
        let database = self
            .database()
            .ok_or_else(|| invalid("Storage is closed"))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work(&database)
        })
        .await?
    }

    pub(crate) async fn generation(&self) -> Result<u64, StoreError> {
        self.run_identity(|database| {
            let read = database.begin_read()?;
            read.open_table(META)?
                .get("generation")?
                .map(|value| value.value())
                .filter(|value| *value != 0)
                .ok_or_else(|| invalid("Missing database generation").into())
        })
        .await
    }

    pub(crate) async fn rotate_generation(&self) -> Result<u64, StoreError> {
        self.run(|database| {
            let tx = database.begin_write()?;
            let previous = tx
                .open_table(META)?
                .get("generation")?
                .map(|value| value.value())
                .filter(|value| *value != 0)
                .ok_or_else(|| invalid("Missing database generation"))?;
            let generation = fresh_generation(previous)?;
            tx.open_table(META)?.insert("generation", generation)?;
            tx.commit()?;
            Ok(generation)
        })
        .await
    }

    pub(crate) async fn health(&self) -> Result<(), StoreError> {
        self.run(|database| {
            let read = database.begin_read()?;
            let metadata = read.open_table(META)?;
            if metadata.get("schema_version")?.map(|v| v.value()) != Some(SCHEMA_VERSION) {
                return Err(invalid("Unsupported database schema").into());
            }
            Ok(())
        })
        .await
    }

    pub(crate) async fn load_by_id(
        &self,
        principal: &Principal,
        id: u64,
    ) -> Result<Option<Order>, LoadError> {
        if !valid_id(id) {
            return Ok(None);
        }
        let tenant = principal.key.tenant_id;
        self.run(move |database| {
            let read = database.begin_read()?;
            if read.open_table(TENANT_ORDERS)?.get((tenant, id))?.is_none() {
                return Ok(None);
            }
            let orders = read.open_table(ORDERS)?;
            decode_scoped(orders.get(id)?.as_ref().map(|v| v.value()), tenant)
        })
        .await
        .map_err(|_| LoadError)
    }

    pub(crate) async fn list(
        &self,
        principal: &Principal,
        query: String,
        page: u32,
    ) -> Result<OrderPage, LoadError> {
        let tenant = principal.key.tenant_id;
        let query = query.trim().to_owned();
        self.run(move |database| {
            let needle = query.to_lowercase();
            let read = database.begin_read()?;
            let index = read.open_table(TENANT_ORDERS)?;
            let records = read.open_table(ORDERS)?;
            let mut matches = Vec::new();
            // This bounded example uses numbered pages and scans one tenant's
            // records for literal Unicode-lowercase substring matching.
            for entry in index.range((tenant, 0)..=(tenant, u64::MAX))?.rev() {
                let (key, _) = entry?;
                let id = key.value().1;
                let row = decode_scoped(records.get(id)?.as_ref().map(|v| v.value()), tenant)?
                    .ok_or_else(|| invalid("Order index does not match its record"))?;
                if row.customer.to_lowercase().contains(&needle) {
                    matches.push(row);
                }
            }
            let total_count = u64::try_from(matches.len())?;
            let total_pages = u32::try_from(total_count.div_ceil(u64::from(PAGE_SIZE)).max(1))?;
            let page = page.clamp(1, total_pages);
            let offset = u64::from(page - 1) * u64::from(PAGE_SIZE);
            let orders = matches
                .into_iter()
                .skip(usize::try_from(offset)?)
                .take(PAGE_SIZE as usize)
                .collect();
            Ok(OrderPage {
                orders,
                query,
                page,
                total_pages,
                total_count,
            })
        })
        .await
        .map_err(|_| LoadError)
    }

    pub(crate) async fn create(
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
        let tenant = principal.key.tenant_id;
        let mut order = new_order();
        order.customer = input.customer;
        order.items[0].quantity = input.quantity_one;
        order.items[1].quantity = input.quantity_two;
        checked_total(&order, &draft)?;
        self.run(move |database| {
            let tx = database.begin_write()?;
            ensure_tenant(&tx, tenant)?;
            insert_order(&tx, tenant, &mut order)?;
            tx.commit()?;
            Ok(order)
        })
        .await
        .map_err(|error| save_error(error, draft))
    }

    pub(crate) async fn save_by_id(
        &self,
        principal: &Principal,
        id: u64,
        draft: OrderDraft,
    ) -> Result<Order, SaveError> {
        if !valid_id(id) {
            return Err(SaveError::NotFound { draft });
        }
        let input = validate(&draft).map_err(|errors| SaveError::Validation {
            draft: draft.clone(),
            errors,
        })?;
        let tenant = principal.key.tenant_id;
        let original = draft.clone();
        self.run(move |database| {
            let tx = database.begin_write()?;
            let Some(mut current) = load_write(&tx, tenant, id)? else {
                return Err(SaveError::NotFound { draft }.into());
            };
            if current.revision != input.expected_revision {
                return Err(SaveError::Conflict {
                    draft,
                    current: Box::new(current),
                }
                .into());
            }
            current.revision = current
                .revision
                .checked_add(1)
                .filter(|value| valid_id(*value))
                .ok_or_else(|| invalid("Revision exhausted"))?;
            current.customer = input.customer;
            current.items[0].quantity = input.quantity_one;
            current.items[1].quantity = input.quantity_two;
            checked_total(&current, &draft)?;
            let encoded = serde_json::to_vec(&StoredOrder {
                tenant_id: tenant,
                order: current.clone(),
            })?;
            tx.open_table(ORDERS)?.insert(id, encoded.as_slice())?;
            tx.commit()?;
            Ok(current)
        })
        .await
        .map_err(|error| save_error(error, original))
    }

    pub(crate) async fn delete(
        &self,
        principal: &Principal,
        id: u64,
        expected_revision: String,
    ) -> Result<(), DeleteError> {
        if !valid_id(id) {
            return Err(DeleteError::NotFound { expected_revision });
        }
        let revision = unsigned(&expected_revision)
            .filter(|revision| *revision > 0)
            .ok_or_else(|| DeleteError::Validation {
                expected_revision: expected_revision.clone(),
                message: "The order revision is invalid. Reload the saved order before deleting."
                    .into(),
            })?;
        let tenant = principal.key.tenant_id;
        let original = expected_revision.clone();
        self.run(move |database| {
            let tx = database.begin_write()?;
            let Some(current) = load_write(&tx, tenant, id)? else {
                return Err(DeleteError::NotFound { expected_revision }.into());
            };
            if current.revision != revision {
                return Err(DeleteError::Conflict {
                    expected_revision,
                    current: Box::new(current),
                }
                .into());
            }
            tx.open_table(ORDERS)?.remove(id)?;
            tx.open_table(TENANT_ORDERS)?.remove((tenant, id))?;
            crate::attachments::redb_delete_order(&tx, id)?;
            tx.commit()?;
            Ok(())
        })
        .await
        .map_err(|error| match error.downcast::<DeleteError>() {
            Ok(error) => *error,
            Err(_) => DeleteError::Unavailable {
                expected_revision: original,
            },
        })
    }

    pub(crate) async fn seed_orders(&self, principal: &Principal) -> Result<(), StoreError> {
        let tenant = principal.key.tenant_id;
        self.run(move |database| {
            let tx = database.begin_write()?;
            ensure_tenant(&tx, tenant)?;
            if tx
                .open_table(TENANT_ORDERS)?
                .range((tenant, 0)..=(tenant, u64::MAX))?
                .next()
                .transpose()?
                .is_some()
            {
                return Ok(());
            }
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
                insert_order(&tx, tenant, &mut order)?;
            }
            tx.commit()?;
            Ok(())
        })
        .await
    }

    pub(crate) async fn find_user_by_username(
        &self,
        username: &str,
    ) -> Result<Option<UserRecord>, StoreError> {
        let username = username.to_owned();
        self.run_identity(move |database| {
            let read = database.begin_read()?;
            let Some(id) = read
                .open_table(USER_NAMES)?
                .get(username.as_str())?
                .map(|value| value.value())
            else {
                return Ok(None);
            };
            let users = read.open_table(USERS)?;
            let record = users
                .get(id)?
                .ok_or_else(|| invalid("User index does not match its record"))?;
            Ok(Some(serde_json::from_slice(record.value())?))
        })
        .await
    }

    pub(crate) async fn find_user_by_id(&self, id: i64) -> Result<Option<UserRecord>, StoreError> {
        self.run_identity(move |database| {
            let read = database.begin_read()?;
            read.open_table(USERS)?
                .get(id)?
                .map(|value| serde_json::from_slice(value.value()).map_err(Into::into))
                .transpose()
        })
        .await
    }

    pub(crate) async fn insert_user(
        &self,
        tenant: &str,
        username: &str,
        display_name: &str,
        password_hash: &str,
    ) -> Result<Principal, StoreError> {
        let (tenant, username, display_name, password_hash) = (
            tenant.to_owned(),
            username.to_owned(),
            display_name.to_owned(),
            password_hash.to_owned(),
        );
        self.run(move |database| {
            let tx = database.begin_write()?;
            if tx.open_table(USER_NAMES)?.get(username.as_str())?.is_some() {
                return Err(invalid("Username already exists").into());
            }
            let existing_tenant = tx
                .open_table(TENANT_NAMES)?
                .get(tenant.as_str())?
                .map(|value| value.value());
            let tenant_id = match existing_tenant {
                Some(id) => id,
                None => {
                    let id = i64::try_from(next_id(&tx, "last_tenant_id")?)?;
                    if tx
                        .open_table(TENANTS)?
                        .insert(id, tenant.as_str())?
                        .is_some()
                        || tx
                            .open_table(TENANT_NAMES)?
                            .insert(tenant.as_str(), id)?
                            .is_some()
                    {
                        return Err(
                            invalid("Tenant allocation collided with an existing record").into(),
                        );
                    }
                    id
                }
            };
            let id = i64::try_from(next_id(&tx, "last_user_id")?)?;
            let user = UserRecord {
                id,
                tenant_id,
                username,
                display_name: display_name.clone(),
                password_hash,
                enabled: true,
                session_epoch: 0,
            };
            let encoded = serde_json::to_vec(&user)?;
            if tx
                .open_table(USERS)?
                .insert(id, encoded.as_slice())?
                .is_some()
                || tx
                    .open_table(USER_NAMES)?
                    .insert(user.username.as_str(), id)?
                    .is_some()
            {
                return Err(invalid("User allocation collided with an existing record").into());
            }
            let generation = tx
                .open_table(META)?
                .get("generation")?
                .map(|value| value.value())
                .filter(|value| *value != 0)
                .ok_or_else(|| invalid("Missing database generation"))?;
            tx.commit()?;
            Ok(Principal {
                key: PrincipalKey {
                    user_id: id,
                    tenant_id,
                    generation,
                },
                display_name,
            })
        })
        .await
    }

    pub(crate) async fn revoke_sessions(&self, username: &str) -> Result<bool, StoreError> {
        let username = username.to_owned();
        self.run(move |database| {
            let tx = database.begin_write()?;
            let Some(id) = tx
                .open_table(USER_NAMES)?
                .get(username.as_str())?
                .map(|value| value.value())
            else {
                return Ok(false);
            };
            let mut users = tx.open_table(USERS)?;
            let mut user: UserRecord = serde_json::from_slice(
                users
                    .get(id)?
                    .ok_or_else(|| invalid("Missing user"))?
                    .value(),
            )?;
            user.session_epoch = user
                .session_epoch
                .checked_add(1)
                .ok_or_else(|| invalid("Session epoch exhausted"))?;
            let encoded = serde_json::to_vec(&user)?;
            users.insert(id, encoded.as_slice())?;
            drop(users);
            tx.commit()?;
            Ok(true)
        })
        .await
    }

    pub(crate) async fn set_user_enabled(
        &self,
        username: &str,
        enabled: bool,
    ) -> Result<bool, StoreError> {
        let username = username.to_owned();
        self.run(move |database| {
            let tx = database.begin_write()?;
            let Some(id) = tx
                .open_table(USER_NAMES)?
                .get(username.as_str())?
                .map(|value| value.value())
            else {
                return Ok(false);
            };
            let mut users = tx.open_table(USERS)?;
            let mut user: UserRecord = serde_json::from_slice(
                users
                    .get(id)?
                    .ok_or_else(|| invalid("Missing user"))?
                    .value(),
            )?;
            if user.enabled != enabled {
                user.session_epoch = user
                    .session_epoch
                    .checked_add(1)
                    .ok_or_else(|| invalid("Session epoch exhausted"))?;
                user.enabled = enabled;
            }
            let encoded = serde_json::to_vec(&user)?;
            users.insert(id, encoded.as_slice())?;
            drop(users);
            tx.commit()?;
            Ok(true)
        })
        .await
    }

    pub(crate) async fn backup_to(&self, path: &Path) -> Result<(), StoreError> {
        let path = path.to_owned();
        self.run(move |database| {
            let file = private_file(&path)?;
            let result = (|| -> Result<(), StoreError> {
                let copy = Database::builder().create_file(file)?;
                let read = database.begin_read()?;
                let write = copy.begin_write()?;
                macro_rules! copy_table {
                    ($definition:expr) => {{
                        let source = read.open_table($definition)?;
                        let mut destination = write.open_table($definition)?;
                        for row in source.iter()? {
                            let (key, value) = row?;
                            destination.insert(key.value(), value.value())?;
                        }
                    }};
                }
                copy_table!(META);
                copy_table!(ORDERS);
                copy_table!(TENANT_ORDERS);
                copy_table!(TENANTS);
                copy_table!(TENANT_NAMES);
                copy_table!(USERS);
                copy_table!(USER_NAMES);
                crate::attachments::redb_copy(&read, &write)?;
                // Session credentials and recovery data never enter the copy.
                write.commit()?;
                drop(copy);
                sync_created_file(&path)?;
                Ok(())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(&path);
            }
            result
        })
        .await
    }

    pub(crate) async fn validate_backup(path: &Path) -> Result<(), StoreError> {
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || {
            let database = redb::ReadOnlyDatabase::open(path)?;
            validate_snapshot(&database.begin_read()?)
        })
        .await?
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn valid_id(id: u64) -> bool {
    id > 0 && id <= i64::MAX as u64
}
fn private_file(path: &Path) -> io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
fn sync_created_file(path: &Path) -> io::Result<()> {
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
fn save_error(error: StoreError, draft: OrderDraft) -> SaveError {
    match error.downcast::<SaveError>() {
        Ok(error) => *error,
        Err(_) => SaveError::Unavailable { draft },
    }
}
fn decode_scoped(bytes: Option<&[u8]>, tenant: i64) -> Result<Option<Order>, StoreError> {
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let stored: StoredOrder = serde_json::from_slice(bytes)?;
    Ok((stored.tenant_id == tenant).then_some(stored.order))
}
fn load_write(
    tx: &redb::WriteTransaction,
    tenant: i64,
    id: u64,
) -> Result<Option<Order>, StoreError> {
    if tx.open_table(TENANT_ORDERS)?.get((tenant, id))?.is_none() {
        return Ok(None);
    }
    let records = tx.open_table(ORDERS)?;
    decode_scoped(records.get(id)?.as_ref().map(|value| value.value()), tenant)
}
fn ensure_tenant(tx: &redb::WriteTransaction, tenant: i64) -> Result<(), StoreError> {
    if tenant <= 0 || tx.open_table(TENANTS)?.get(tenant)?.is_none() {
        return Err(invalid("Tenant does not exist").into());
    }
    Ok(())
}
fn next_id(tx: &redb::WriteTransaction, counter: &str) -> Result<u64, StoreError> {
    let mut metadata = tx.open_table(META)?;
    let current = metadata
        .get(counter)?
        .map(|value| value.value())
        .unwrap_or(0);
    let next = current
        .checked_add(1)
        .filter(|id| valid_id(*id))
        .ok_or_else(|| invalid("ID counter exhausted"))?;
    metadata.insert(counter, next)?;
    Ok(next)
}
fn insert_order(
    tx: &redb::WriteTransaction,
    tenant: i64,
    order: &mut Order,
) -> Result<(), StoreError> {
    order.id = next_id(tx, "last_order_id")?;
    let encoded = serde_json::to_vec(&StoredOrder {
        tenant_id: tenant,
        order: order.clone(),
    })?;
    if tx
        .open_table(ORDERS)?
        .insert(order.id, encoded.as_slice())?
        .is_some()
        || tx
            .open_table(TENANT_ORDERS)?
            .insert((tenant, order.id), ())?
            .is_some()
    {
        return Err(invalid("Order allocation collided with an existing record").into());
    }
    Ok(())
}
fn migrate(database: &Database) -> Result<(), StoreError> {
    let tx = database.begin_write()?;
    let names: Vec<_> = tx
        .list_tables()?
        .map(|table| table.name().to_owned())
        .collect();
    let version = if names.iter().any(|name| name == "metadata") {
        tx.open_table(META)?
            .get("schema_version")?
            .map(|value| value.value())
    } else {
        None
    };
    if version.is_none()
        && (names.iter().any(|name| name != "metadata")
            || tx.list_multimap_tables()?.next().is_some())
    {
        return Err(invalid("Foreign database schema; use a new application database").into());
    }
    let version = version.unwrap_or(0);
    if version > SCHEMA_VERSION {
        return Err(invalid("Database schema is newer than this application").into());
    }
    tx.open_table(TENANTS)?;
    tx.open_table(TENANT_NAMES)?;
    tx.open_table(USERS)?;
    tx.open_table(USER_NAMES)?;
    tx.open_table(ORDERS)?;
    if version < 2 {
        let orders = tx.open_table(ORDERS)?;
        let mut index = tx.open_table(TENANT_ORDERS)?;
        for row in orders.iter()? {
            let (id, bytes) = row?;
            let stored: StoredOrder = serde_json::from_slice(bytes.value())?;
            index.insert((stored.tenant_id, id.value()), ())?;
        }
    }
    // An old or damaged counter must not cause an insert to replace a record.
    let order_max = tx
        .open_table(ORDERS)?
        .last()?
        .map(|(key, _)| key.value())
        .unwrap_or(0);
    let user_max = u64::try_from(
        tx.open_table(USERS)?
            .last()?
            .map(|(key, _)| key.value())
            .unwrap_or(0),
    )?;
    let tenant_max = u64::try_from(
        tx.open_table(TENANTS)?
            .last()?
            .map(|(key, _)| key.value())
            .unwrap_or(0),
    )?;
    {
        let mut metadata = tx.open_table(META)?;
        for (name, highest) in [
            ("last_order_id", order_max),
            ("last_user_id", user_max),
            ("last_tenant_id", tenant_max),
        ] {
            let existing = metadata.get(name)?.map(|value| value.value()).unwrap_or(0);
            let next = existing.max(highest);
            if next > i64::MAX as u64 {
                return Err(invalid("Invalid allocation counter").into());
            }
            metadata.insert(name, next)?;
        }
        let generation = metadata.get("generation")?.map(|value| value.value());
        match generation {
            Some(0) => return Err(invalid("Invalid database generation").into()),
            Some(_) => {}
            None => {
                metadata.insert("generation", fresh_generation(0)?)?;
            }
        }
    }
    crate::attachments::redb_initialize(&tx)?;
    tx.open_table(META)?
        .insert("schema_version", SCHEMA_VERSION)?;
    tx.commit()?;
    Ok(())
}

fn validate_snapshot(read: &redb::ReadTransaction) -> Result<(), StoreError> {
    let metadata = read.open_table(META)?;
    let version = metadata.get("schema_version")?.map(|v| v.value());
    if !matches!(version, Some(2 | 3 | SCHEMA_VERSION)) {
        return Err(invalid("Unsupported backup schema").into());
    }
    let generation = metadata.get("generation")?.map(|value| value.value());
    if generation == Some(0)
        || (version.is_some_and(|version| version >= 3) && generation.is_none())
    {
        return Err(invalid("Invalid backup generation").into());
    }
    let tenants = read.open_table(TENANTS)?;
    let tenant_names = read.open_table(TENANT_NAMES)?;
    let users = read.open_table(USERS)?;
    let user_names = read.open_table(USER_NAMES)?;
    let orders = read.open_table(ORDERS)?;
    let index = read.open_table(TENANT_ORDERS)?;
    if tenants.len()? != tenant_names.len()?
        || users.len()? != user_names.len()?
        || orders.len()? != index.len()?
    {
        return Err(invalid("Backup index does not match its records").into());
    }
    let mut max_tenant = 0;
    for row in tenants.iter()? {
        let (id, name) = row?;
        if id.value() <= 0
            || !valid_identity_name(name.value())
            || tenant_names.get(name.value())?.map(|v| v.value()) != Some(id.value())
        {
            return Err(invalid("Invalid tenant index").into());
        }
        max_tenant = max_tenant.max(id.value() as u64);
    }
    let mut max_user = 0;
    for row in users.iter()? {
        let (id, bytes) = row?;
        let user: UserRecord = serde_json::from_slice(bytes.value())?;
        if id.value() <= 0
            || user.id != id.value()
            || tenants.get(user.tenant_id)?.is_none()
            || user_names.get(user.username.as_str())?.map(|v| v.value()) != Some(user.id)
            || user.session_epoch < 0
            || !valid_identity_name(&user.username)
            || !valid_identity_name(&user.display_name)
        {
            return Err(invalid("Invalid user record").into());
        }
        max_user = max_user.max(user.id as u64);
    }
    let mut max_order = 0;
    for row in orders.iter()? {
        let (id, bytes) = row?;
        let stored: StoredOrder = serde_json::from_slice(bytes.value())?;
        if !valid_order_record(&stored.order)
            || id.value() != stored.order.id
            || tenants.get(stored.tenant_id)?.is_none()
            || index.get((stored.tenant_id, id.value()))?.is_none()
        {
            return Err(invalid("Invalid order record").into());
        }
        max_order = max_order.max(id.value());
    }
    for (name, maximum) in [
        ("last_order_id", max_order),
        ("last_user_id", max_user),
        ("last_tenant_id", max_tenant),
    ] {
        let counter = metadata.get(name)?.map(|value| value.value()).unwrap_or(0);
        if counter < maximum || counter > i64::MAX as u64 {
            return Err(invalid("Invalid allocation counter").into());
        }
    }
    crate::attachments::redb_validate(read, version.is_some_and(|version| version >= 4))?;
    for name in ["export_jobs", "export_results"] {
        match read.open_table(TableDefinition::<&str, &[u8]>::new(name)) {
            Ok(table) if !table.is_empty()? => {
                return Err(invalid("Backup contains private export data").into());
            }
            Ok(_) | Err(redb::TableError::TableDoesNotExist(_)) => {}
            Err(error) => return Err(error.into()),
        }
    }
    // Backups made by this application omit the session table. An empty table
    // is also acceptable after a restore has initialized its session adapter.
    match read.open_table(SESSIONS) {
        Ok(sessions) if !sessions.is_empty()? => {
            return Err(invalid("Backup contains active session data").into());
        }
        Ok(_) | Err(redb::TableError::TableDoesNotExist(_)) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn saturated_order_capacity_waits_and_does_not_block_identity_reads() {
        let directory = tempfile::tempdir().unwrap();
        let store = RedbStore::open(&directory.path().join("capacity.redb"))
            .await
            .unwrap();
        store
            .insert_user("team", "alice", "Alice", "test-hash")
            .await
            .unwrap();
        let held = store.permits.clone().acquire_many_owned(4).await.unwrap();
        let queued_store = store.clone();
        let (started, ready) = tokio::sync::oneshot::channel();
        let queued = tokio::spawn(async move {
            started.send(()).unwrap();
            queued_store.health().await
        });
        ready.await.unwrap();
        tokio::task::yield_now().await;
        assert!(!queued.is_finished());
        assert!(
            store
                .find_user_by_username("alice")
                .await
                .unwrap()
                .is_some()
        );
        drop(held);
        queued.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn colliding_counters_cannot_replace_existing_orders_users_or_tenants() {
        let directory = tempfile::tempdir().unwrap();
        let store = RedbStore::open(&directory.path().join("collisions.redb"))
            .await
            .unwrap();
        let alice = store
            .insert_user("team", "alice", "Alice", "original-hash")
            .await
            .unwrap();
        let order = store
            .create(
                &alice,
                OrderDraft {
                    customer: "Original".into(),
                    ..new_order().draft()
                },
            )
            .await
            .unwrap();
        let database = store.database().unwrap();
        let tx = database.begin_write().unwrap();
        tx.open_table(META)
            .unwrap()
            .insert("last_order_id", 0)
            .unwrap();
        tx.commit().unwrap();
        assert!(
            store
                .create(
                    &alice,
                    OrderDraft {
                        customer: "Must not replace".into(),
                        ..new_order().draft()
                    }
                )
                .await
                .is_err()
        );
        assert_eq!(
            store.load_by_id(&alice, order.id).await.unwrap(),
            Some(order)
        );
        let tx = database.begin_write().unwrap();
        tx.open_table(META)
            .unwrap()
            .insert("last_user_id", 0)
            .unwrap();
        tx.commit().unwrap();
        assert!(
            store
                .insert_user("new-team", "bob", "Bob", "new-hash")
                .await
                .is_err()
        );
        let original = store.find_user_by_username("alice").await.unwrap().unwrap();
        assert_eq!(original.password_hash, "original-hash");
        assert!(store.find_user_by_username("bob").await.unwrap().is_none());
        let tx = database.begin_write().unwrap();
        tx.open_table(META)
            .unwrap()
            .insert("last_user_id", 1)
            .unwrap();
        tx.open_table(META)
            .unwrap()
            .insert("last_tenant_id", 0)
            .unwrap();
        tx.commit().unwrap();
        assert!(
            store
                .insert_user("new-team", "bob", "Bob", "new-hash")
                .await
                .is_err()
        );
        let read = database.begin_read().unwrap();
        assert_eq!(read.open_table(TENANTS).unwrap().len().unwrap(), 1);
        assert_eq!(
            read.open_table(TENANTS)
                .unwrap()
                .get(alice.key.tenant_id)
                .unwrap()
                .unwrap()
                .value(),
            "team"
        );
    }
}
