//! Bounded order attachments. A complete upload and its metadata commit together
//! in the application database, so interrupted parsing leaves nothing to clean up
//! and backups include the exact attachment bytes from the same snapshot.

use serde::{Deserialize, Serialize};

pub const MAX_ATTACHMENT_BYTES: usize = 1024 * 1024;
pub const MAX_ATTACHMENTS: usize = 3;
pub const MAX_MULTIPART_BYTES: usize = MAX_ATTACHMENT_BYTES + 16 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Attachment {
    pub id: u64,
    pub file_name: String,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AttachmentError {
    Invalid,
    TooLarge,
    TooMany,
    NotFound,
    IdentityChanged,
    Unavailable,
}

impl std::fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "Select one nonempty file with a name of at most 180 bytes.",
            Self::TooLarge => "The attachment must be at most 1 MiB.",
            Self::TooMany => "An order can have at most three attachments. Remove one before adding another.",
            Self::NotFound => "The order or attachment is no longer available.",
            Self::IdentityChanged => "The signed-in account changed. Reload the order before uploading.",
            Self::Unavailable => "Attachments are temporarily unavailable. Reload the order before retrying; the upload may have completed.",
        })
    }
}
impl std::error::Error for AttachmentError {}

#[cfg(feature = "ssr")]
mod server {
    use super::*;
    use crate::{
        auth::{AuthSession, Principal, PrincipalKey, authorize},
        store::{OrderStore, StoreError, UserRecord},
    };
    use axum::{
        Form, Json, Router,
        extract::{DefaultBodyLimit, FromRequest, Multipart, Path, Request, State},
        http::{HeaderMap, StatusCode, header},
        response::{Html, IntoResponse, Redirect, Response},
        routing::{get as get_route, post},
    };
    use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
    use std::{
        sync::{Arc, LazyLock},
        time::Duration,
    };
    use tokio::sync::Semaphore;

    const FILES: TableDefinition<(u64, u64), &[u8]> = TableDefinition::new("order_attachments");
    const ORDERS: TableDefinition<u64, &[u8]> = TableDefinition::new("orders");
    const USERS: TableDefinition<i64, &[u8]> = TableDefinition::new("users");
    const META: TableDefinition<&str, u64> = TableDefinition::new("metadata");
    // Capacity is shared across routers and clones, including independent test
    // applications. Redb work never occupies an async executor thread.
    static BLOCKING: LazyLock<Arc<Semaphore>> = LazyLock::new(|| Arc::new(Semaphore::new(4)));

    fn clean_name(name: &str) -> Result<String, AttachmentError> {
        let name = name.rsplit(['/', '\\']).next().unwrap_or("").trim();
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.len() > 180
            || name.chars().any(char::is_control)
        {
            return Err(AttachmentError::Invalid);
        }
        Ok(name.to_owned())
    }

    fn descriptor(
        id: u64,
        file_name: String,
        length: usize,
    ) -> Result<Attachment, AttachmentError> {
        if length > MAX_ATTACHMENT_BYTES {
            return Err(AttachmentError::TooLarge);
        }
        if length == 0 {
            return Err(AttachmentError::Invalid);
        }
        Ok(Attachment {
            id,
            file_name: clean_name(&file_name)?,
            size: length as u64,
        })
    }

    fn fresh_id() -> Result<u64, StoreError> {
        loop {
            let id = crate::store::fresh_generation(0)? & i64::MAX as u64;
            if id != 0 {
                return Ok(id);
            }
        }
    }

    fn encode(meta: &Attachment, bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
        let header = serde_json::to_vec(meta)?;
        let mut result = Vec::with_capacity(4 + header.len() + bytes.len());
        result.extend_from_slice(&(header.len() as u32).to_le_bytes());
        result.extend(header);
        result.extend_from_slice(bytes);
        Ok(result)
    }

    fn decode(encoded: &[u8]) -> Result<(Attachment, &[u8]), StoreError> {
        let length = encoded.get(..4).ok_or(AttachmentError::Invalid)?;
        let length = u32::from_le_bytes(length.try_into()?) as usize;
        if length > 1024 {
            return Err(AttachmentError::Invalid.into());
        }
        let header = encoded.get(4..4 + length).ok_or(AttachmentError::Invalid)?;
        let meta: Attachment = serde_json::from_slice(header)?;
        let bytes = &encoded[4 + length..];
        if meta.id == 0
            || meta.id > i64::MAX as u64
            || descriptor(meta.id, meta.file_name.clone(), bytes.len())? != meta
        {
            return Err(AttachmentError::Invalid.into());
        }
        Ok((meta, bytes))
    }

    #[derive(Deserialize)]
    struct Scope {
        tenant_id: i64,
    }

    fn check_scope(
        order: Option<&[u8]>,
        user: Option<&[u8]>,
        generation: Option<u64>,
        principal: &Principal,
    ) -> Result<(), StoreError> {
        if generation != Some(principal.key.generation) {
            return Err(AttachmentError::IdentityChanged.into());
        }
        let user: UserRecord =
            serde_json::from_slice(user.ok_or(AttachmentError::IdentityChanged)?)?;
        if !user.enabled
            || user.id != principal.key.user_id
            || user.tenant_id != principal.key.tenant_id
        {
            return Err(AttachmentError::IdentityChanged.into());
        }
        let scope: Scope = serde_json::from_slice(order.ok_or(AttachmentError::NotFound)?)?;
        if scope.tenant_id != principal.key.tenant_id {
            return Err(AttachmentError::NotFound.into());
        }
        Ok(())
    }

    fn check_read(
        read: &redb::ReadTransaction,
        principal: &Principal,
        order: u64,
    ) -> Result<(), StoreError> {
        let orders = read.open_table(ORDERS)?;
        let users = read.open_table(USERS)?;
        let meta = read.open_table(META)?;
        check_scope(
            orders.get(order)?.as_ref().map(|value| value.value()),
            users
                .get(principal.key.user_id)?
                .as_ref()
                .map(|value| value.value()),
            meta.get("generation")?.map(|value| value.value()),
            principal,
        )
    }

    fn check_write(
        write: &redb::WriteTransaction,
        principal: &Principal,
        order: u64,
    ) -> Result<(), StoreError> {
        let orders = write.open_table(ORDERS)?;
        let users = write.open_table(USERS)?;
        let meta = write.open_table(META)?;
        check_scope(
            orders.get(order)?.as_ref().map(|value| value.value()),
            users
                .get(principal.key.user_id)?
                .as_ref()
                .map(|value| value.value()),
            meta.get("generation")?.map(|value| value.value()),
            principal,
        )
    }

    fn map_error(error: StoreError) -> AttachmentError {
        error
            .downcast::<AttachmentError>()
            .map(|error| *error)
            .unwrap_or(AttachmentError::Unavailable)
    }

    async fn run<T: Send + 'static>(
        database: Arc<Database>,
        work: impl FnOnce(&Database) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, AttachmentError> {
        let permit = tokio::time::timeout(Duration::from_secs(5), BLOCKING.clone().acquire_owned())
            .await
            .map_err(|_| AttachmentError::Unavailable)?
            .map_err(|_| AttachmentError::Unavailable)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            work(&database)
        })
        .await
        .map_err(|_| AttachmentError::Unavailable)?
        .map_err(map_error)
    }

    pub async fn list(
        store: &OrderStore,
        principal: &Principal,
        order: u64,
    ) -> Result<Vec<Attachment>, AttachmentError> {
        if let Some(database) = store.redb_database() {
            let principal = principal.clone();
            return run(database, move |database| {
                let read = database.begin_read()?;
                check_read(&read, &principal, order)?;
                let table = read.open_table(FILES)?;
                let mut result = Vec::new();
                for row in table.range((order, 0)..=(order, u64::MAX))? {
                    let (_, value) = row?;
                    result.push(decode(value.value())?.0);
                }
                Ok(result)
            })
            .await;
        }
        #[cfg(feature = "sqlite")]
        if let Some(pool) = store.sqlite_pool() {
            return sql_list(&pool, principal, order).await.map_err(map_error);
        }
        Err(AttachmentError::Unavailable)
    }

    pub async fn put(
        store: &OrderStore,
        principal: &Principal,
        order: u64,
        file_name: String,
        bytes: Vec<u8>,
    ) -> Result<Attachment, AttachmentError> {
        let mut meta = descriptor(0, file_name, bytes.len())?;
        if let Some(database) = store.redb_database() {
            let principal = principal.clone();
            return run(database, move |database| {
                let write = database.begin_write()?;
                check_write(&write, &principal, order)?;
                {
                    let mut table = write.open_table(FILES)?;
                    if table.range((order, 0)..=(order, u64::MAX))?.count() >= MAX_ATTACHMENTS {
                        return Err(AttachmentError::TooMany.into());
                    }
                    loop {
                        meta.id = fresh_id()?;
                        if table.get((order, meta.id))?.is_none() {
                            break;
                        }
                    }
                    let encoded = encode(&meta, &bytes)?;
                    table.insert((order, meta.id), encoded.as_slice())?;
                }
                write.commit()?;
                Ok(meta)
            })
            .await;
        }
        #[cfg(feature = "sqlite")]
        if let Some(pool) = store.sqlite_pool() {
            return sql_put(&pool, principal, order, meta, bytes)
                .await
                .map_err(map_error);
        }
        Err(AttachmentError::Unavailable)
    }

    pub async fn get(
        store: &OrderStore,
        principal: &Principal,
        order: u64,
        id: u64,
    ) -> Result<(Attachment, Vec<u8>), AttachmentError> {
        if let Some(database) = store.redb_database() {
            let principal = principal.clone();
            return run(database, move |database| {
                let read = database.begin_read()?;
                check_read(&read, &principal, order)?;
                let table = read.open_table(FILES)?;
                let value = table.get((order, id))?.ok_or(AttachmentError::NotFound)?;
                let (meta, bytes) = decode(value.value())?;
                Ok((meta, bytes.to_vec()))
            })
            .await;
        }
        #[cfg(feature = "sqlite")]
        if let Some(pool) = store.sqlite_pool() {
            return sql_get(&pool, principal, order, id)
                .await
                .map_err(map_error);
        }
        Err(AttachmentError::Unavailable)
    }

    pub async fn remove(
        store: &OrderStore,
        principal: &Principal,
        order: u64,
        id: u64,
    ) -> Result<(), AttachmentError> {
        if let Some(database) = store.redb_database() {
            let principal = principal.clone();
            return run(database, move |database| {
                let write = database.begin_write()?;
                check_write(&write, &principal, order)?;
                if write.open_table(FILES)?.remove((order, id))?.is_none() {
                    return Err(AttachmentError::NotFound.into());
                }
                write.commit()?;
                Ok(())
            })
            .await;
        }
        #[cfg(feature = "sqlite")]
        if let Some(pool) = store.sqlite_pool() {
            return sql_remove(&pool, principal, order, id)
                .await
                .map_err(map_error);
        }
        Err(AttachmentError::Unavailable)
    }

    pub(crate) fn redb_initialize(write: &redb::WriteTransaction) -> Result<(), StoreError> {
        write.open_table(FILES)?;
        Ok(())
    }
    pub(crate) fn redb_copy(
        read: &redb::ReadTransaction,
        write: &redb::WriteTransaction,
    ) -> Result<(), StoreError> {
        let source = read.open_table(FILES)?;
        let mut target = write.open_table(FILES)?;
        for row in source.iter()? {
            let (key, value) = row?;
            target.insert(key.value(), value.value())?;
        }
        Ok(())
    }
    pub(crate) fn redb_delete_order(
        write: &redb::WriteTransaction,
        order: u64,
    ) -> Result<(), StoreError> {
        let mut files = write.open_table(FILES)?;
        let keys = files
            .range((order, 0)..=(order, u64::MAX))?
            .map(|row| row.map(|(key, _)| key.value()))
            .collect::<Result<Vec<_>, _>>()?;
        for key in keys {
            files.remove(key)?;
        }
        Ok(())
    }
    pub(crate) fn redb_validate(
        read: &redb::ReadTransaction,
        required: bool,
    ) -> Result<(), StoreError> {
        let files = match read.open_table(FILES) {
            Ok(files) => files,
            Err(redb::TableError::TableDoesNotExist(_)) if !required => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        let orders = read.open_table(ORDERS)?;
        let mut previous = 0;
        let mut count = 0;
        for row in files.iter()? {
            let (key, value) = row?;
            let (order, id) = key.value();
            let (meta, _) = decode(value.value())?;
            if id != meta.id || orders.get(order)?.is_none() {
                return Err(AttachmentError::Invalid.into());
            }
            if previous != order {
                previous = order;
                count = 0;
            }
            count += 1;
            if count > MAX_ATTACHMENTS {
                return Err(AttachmentError::TooMany.into());
            }
        }
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn check_sql(
        connection: &mut sqlx::SqliteConnection,
        principal: &Principal,
        order: u64,
    ) -> Result<(), StoreError> {
        let generation: String =
            sqlx::query_scalar("SELECT value FROM database_metadata WHERE name = 'generation'")
                .fetch_one(&mut *connection)
                .await?;
        let user: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM users WHERE id = ? AND tenant_id = ? AND enabled = 1",
        )
        .bind(principal.key.user_id)
        .bind(principal.key.tenant_id)
        .fetch_optional(&mut *connection)
        .await?;
        if generation.parse::<u64>()? != principal.key.generation || user.is_none() {
            return Err(AttachmentError::IdentityChanged.into());
        }
        let order = i64::try_from(order).map_err(|_| AttachmentError::NotFound)?;
        let existing: Option<i64> =
            sqlx::query_scalar("SELECT id FROM orders WHERE id = ? AND tenant_id = ?")
                .bind(order)
                .bind(principal.key.tenant_id)
                .fetch_optional(&mut *connection)
                .await?;
        existing.ok_or(AttachmentError::NotFound)?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    async fn sql_list(
        pool: &sqlx::SqlitePool,
        principal: &Principal,
        order: u64,
    ) -> Result<Vec<Attachment>, StoreError> {
        let mut tx = pool.begin().await?;
        check_sql(&mut tx, principal, order).await?;
        let rows: Vec<(i64, String, i64)> = sqlx::query_as("SELECT id, file_name, length(bytes) FROM order_attachments WHERE order_id = ? ORDER BY id")
            .bind(order as i64).fetch_all(&mut *tx).await?;
        Ok(rows
            .into_iter()
            .map(|(id, file_name, size)| Attachment {
                id: id as u64,
                file_name,
                size: size as u64,
            })
            .collect())
    }

    #[cfg(feature = "sqlite")]
    async fn sql_put(
        pool: &sqlx::SqlitePool,
        principal: &Principal,
        order: u64,
        mut meta: Attachment,
        bytes: Vec<u8>,
    ) -> Result<Attachment, StoreError> {
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        check_sql(&mut tx, principal, order).await?;
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM order_attachments WHERE order_id = ?")
                .bind(order as i64)
                .fetch_one(&mut *tx)
                .await?;
        if count >= MAX_ATTACHMENTS as i64 {
            return Err(AttachmentError::TooMany.into());
        }
        loop {
            meta.id = fresh_id()?;
            let exists: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM order_attachments WHERE order_id = ? AND id = ?",
            )
            .bind(order as i64)
            .bind(meta.id as i64)
            .fetch_optional(&mut *tx)
            .await?;
            if exists.is_none() {
                break;
            }
        }
        sqlx::query(
            "INSERT INTO order_attachments(order_id, id, file_name, bytes) VALUES (?, ?, ?, ?)",
        )
        .bind(order as i64)
        .bind(meta.id as i64)
        .bind(&meta.file_name)
        .bind(bytes)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(meta)
    }

    #[cfg(feature = "sqlite")]
    async fn sql_get(
        pool: &sqlx::SqlitePool,
        principal: &Principal,
        order: u64,
        id: u64,
    ) -> Result<(Attachment, Vec<u8>), StoreError> {
        let mut tx = pool.begin().await?;
        check_sql(&mut tx, principal, order).await?;
        let id = i64::try_from(id).map_err(|_| AttachmentError::NotFound)?;
        let row: Option<(String, Vec<u8>)> = sqlx::query_as(
            "SELECT file_name, bytes FROM order_attachments WHERE order_id = ? AND id = ?",
        )
        .bind(order as i64)
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        let (file_name, bytes) = row.ok_or(AttachmentError::NotFound)?;
        Ok((descriptor(id as u64, file_name, bytes.len())?, bytes))
    }

    #[cfg(feature = "sqlite")]
    async fn sql_remove(
        pool: &sqlx::SqlitePool,
        principal: &Principal,
        order: u64,
        id: u64,
    ) -> Result<(), StoreError> {
        let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
        check_sql(&mut tx, principal, order).await?;
        let id = i64::try_from(id).map_err(|_| AttachmentError::NotFound)?;
        if sqlx::query("DELETE FROM order_attachments WHERE order_id = ? AND id = ?")
            .bind(order as i64)
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected()
            == 0
        {
            return Err(AttachmentError::NotFound.into());
        }
        tx.commit().await?;
        Ok(())
    }

    #[cfg(feature = "sqlite")]
    pub(crate) async fn sqlite_validate(
        pool: &sqlx::SqlitePool,
        required: bool,
    ) -> Result<(), StoreError> {
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'order_attachments'",
        )
        .fetch_optional(pool)
        .await?;
        if exists.is_none() {
            return if required {
                Err(AttachmentError::Invalid.into())
            } else {
                Ok(())
            };
        }
        let invalid: i64 = sqlx::query_scalar("SELECT count(*) FROM order_attachments a LEFT JOIN orders o ON a.order_id = o.id WHERE o.id IS NULL OR a.id <= 0 OR length(a.bytes) NOT BETWEEN 1 AND 1048576")
            .fetch_one(pool).await?;
        let over_limit: i64 = sqlx::query_scalar("SELECT count(*) FROM (SELECT order_id FROM order_attachments GROUP BY order_id HAVING count(*) > 3)")
            .fetch_one(pool).await?;
        if invalid != 0 || over_limit != 0 {
            return Err(AttachmentError::Invalid.into());
        }
        let names: Vec<String> = sqlx::query_scalar("SELECT file_name FROM order_attachments")
            .fetch_all(pool)
            .await?;
        for name in names {
            if clean_name(&name)? != name {
                return Err(AttachmentError::Invalid.into());
            }
        }
        Ok(())
    }

    /// Merge before the application's authentication and CSRF middleware.
    pub fn routes<S: Clone + Send + Sync + 'static>(store: OrderStore) -> Router<S> {
        Router::new()
            .route("/orders/{id}/attachments", post(upload))
            .route(
                "/orders/{id}/attachments/{attachment_id}",
                get_route(download),
            )
            .route(
                "/orders/{id}/attachments/{attachment_id}/delete",
                post(delete),
            )
            .layer(DefaultBodyLimit::max(MAX_MULTIPART_BYTES))
            .with_state(store)
    }

    fn json_response(headers: &HeaderMap) -> bool {
        headers
            .get(header::ACCEPT)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(',')
                    .any(|part| part.split(';').next().unwrap_or("").trim() == "application/json")
            })
    }
    #[derive(Serialize)]
    struct Failure {
        message: String,
        error: AttachmentError,
    }

    fn error_response(error: AttachmentError, order: u64, json: bool) -> Response {
        let status = match error {
            AttachmentError::Invalid => StatusCode::BAD_REQUEST,
            AttachmentError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            AttachmentError::TooMany => StatusCode::CONFLICT,
            AttachmentError::NotFound => StatusCode::NOT_FOUND,
            AttachmentError::IdentityChanged => StatusCode::FORBIDDEN,
            AttachmentError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        };
        let mut response = if json {
            (
                status,
                Json(Failure {
                    message: error.to_string(),
                    error,
                }),
            )
                .into_response()
        } else {
            (status, Html(format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Attachment not saved</title><link rel=\"stylesheet\" href=\"/theme.css\"><main class=\"container\"><h1>Attachment not saved</h1><p>{error}</p><p>File selections cannot be restored. Return to the order and select the file again if needed.</p><a href=\"/orders/{order}\">Return to the order</a></main></html>"))).into_response()
        };
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
        response
    }

    async fn upload(
        State(store): State<OrderStore>,
        auth: AuthSession,
        Path(order): Path<u64>,
        req: Request,
    ) -> Response {
        let json = json_response(req.headers());
        if auth.user.is_none() {
            return if json {
                StatusCode::UNAUTHORIZED.into_response()
            } else {
                Redirect::to(&format!("/login?return_to=%2Forders%2F{order}")).into_response()
            };
        }
        let multipart = match Multipart::from_request(req, &()).await {
            Ok(value) => value,
            Err(_) => return error_response(AttachmentError::Invalid, order, json),
        };
        let parsed =
            match tokio::time::timeout(Duration::from_secs(30), parse_upload(multipart)).await {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => return error_response(error, order, json),
                Err(_) => return error_response(AttachmentError::Invalid, order, json),
            };
        let principal = match authorize(&auth, parsed.expected) {
            Ok(principal) => principal,
            Err(_) => return error_response(AttachmentError::IdentityChanged, order, json),
        };
        match put(&store, &principal, order, parsed.file_name, parsed.bytes).await {
            Ok(meta) if json => (
                StatusCode::CREATED,
                [(header::CACHE_CONTROL, "private, no-store")],
                Json(meta),
            )
                .into_response(),
            Ok(_) => Redirect::to(&format!("/orders/{order}")).into_response(),
            Err(error) => error_response(error, order, json),
        }
    }

    struct Upload {
        expected: PrincipalKey,
        file_name: String,
        bytes: Vec<u8>,
    }

    async fn parse_upload(mut multipart: Multipart) -> Result<Upload, AttachmentError> {
        let mut fields = std::collections::BTreeMap::new();
        let mut file = None;
        while let Some(mut field) = multipart.next_field().await.map_err(multipart_error)? {
            let name = field.name().ok_or(AttachmentError::Invalid)?.to_owned();
            if name == "attachment" {
                if file.is_some() {
                    return Err(AttachmentError::Invalid);
                }
                let file_name = clean_name(field.file_name().ok_or(AttachmentError::Invalid)?)?;
                let mut bytes = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                    if bytes.len().saturating_add(chunk.len()) > MAX_ATTACHMENT_BYTES {
                        return Err(AttachmentError::TooLarge);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                descriptor(0, file_name.clone(), bytes.len())?;
                file = Some((file_name, bytes));
            } else {
                if !matches!(
                    name.as_str(),
                    "_csrf" | "expected_user_id" | "expected_tenant_id" | "expected_generation"
                ) || fields.contains_key(&name)
                    || field.file_name().is_some()
                {
                    return Err(AttachmentError::Invalid);
                }
                let mut bytes = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
                    if bytes.len().saturating_add(chunk.len()) > 256 {
                        return Err(AttachmentError::Invalid);
                    }
                    bytes.extend_from_slice(&chunk);
                }
                fields.insert(
                    name,
                    String::from_utf8(bytes).map_err(|_| AttachmentError::Invalid)?,
                );
            }
        }
        let read = |name| fields.get(name).ok_or(AttachmentError::Invalid);
        let expected = PrincipalKey {
            user_id: read("expected_user_id")?
                .parse()
                .map_err(|_| AttachmentError::Invalid)?,
            tenant_id: read("expected_tenant_id")?
                .parse()
                .map_err(|_| AttachmentError::Invalid)?,
            generation: read("expected_generation")?
                .parse()
                .map_err(|_| AttachmentError::Invalid)?,
        };
        let (file_name, bytes) = file.ok_or(AttachmentError::Invalid)?;
        Ok(Upload {
            expected,
            file_name,
            bytes,
        })
    }

    fn multipart_error(error: axum::extract::multipart::MultipartError) -> AttachmentError {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            AttachmentError::TooLarge
        } else {
            AttachmentError::Invalid
        }
    }

    async fn download(
        State(store): State<OrderStore>,
        auth: AuthSession,
        Path((order, id)): Path<(u64, u64)>,
    ) -> Response {
        let Some(user) = auth.user else {
            return StatusCode::UNAUTHORIZED.into_response();
        };
        match get(&store, &user.principal(), order, id).await {
            Ok((meta, bytes)) => {
                let encoded = meta
                    .file_name
                    .bytes()
                    .map(|byte| {
                        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                            (byte as char).to_string()
                        } else {
                            format!("%{byte:02X}")
                        }
                    })
                    .collect::<String>();
                (
                    [
                        (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
                        (
                            header::CONTENT_DISPOSITION,
                            format!(
                                "attachment; filename=\"attachment\"; filename*=UTF-8''{encoded}"
                            ),
                        ),
                        (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
                        (header::CACHE_CONTROL, "private, no-store".to_owned()),
                    ],
                    bytes,
                )
                    .into_response()
            }
            Err(error) => error_response(error, order, true),
        }
    }

    #[derive(Deserialize)]
    struct Expected {
        expected_user_id: i64,
        expected_tenant_id: i64,
        expected_generation: u64,
    }
    async fn delete(
        State(store): State<OrderStore>,
        auth: AuthSession,
        Path((order, id)): Path<(u64, u64)>,
        headers: HeaderMap,
        Form(expected): Form<Expected>,
    ) -> Response {
        let json = json_response(&headers);
        if auth.user.is_none() {
            return if json {
                StatusCode::UNAUTHORIZED.into_response()
            } else {
                Redirect::to(&format!("/login?return_to=%2Forders%2F{order}")).into_response()
            };
        }
        let expected = PrincipalKey {
            user_id: expected.expected_user_id,
            tenant_id: expected.expected_tenant_id,
            generation: expected.expected_generation,
        };
        let principal = match authorize(&auth, expected) {
            Ok(principal) => principal,
            Err(_) => return error_response(AttachmentError::IdentityChanged, order, json),
        };
        match remove(&store, &principal, order, id).await {
            Ok(()) if json => StatusCode::NO_CONTENT.into_response(),
            Ok(()) => Redirect::to(&format!("/orders/{order}")).into_response(),
            Err(error) => error_response(error, order, json),
        }
    }
}

#[cfg(feature = "sqlite")]
pub(crate) use server::sqlite_validate;
#[cfg(feature = "ssr")]
pub use server::{get, list, put, remove, routes};
#[cfg(feature = "ssr")]
pub(crate) use server::{redb_copy, redb_delete_order, redb_initialize, redb_validate};
