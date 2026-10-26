//! tower-sessions persistence using the selected application database.
//! Session IDs are opaque; a delayed save cannot resurrect a deleted session.
use crate::store::{OrderStore, StoreError};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::Arc;
use tower_sessions::{
    ExpiredDeletion, SessionStore,
    session::{Id, Record},
    session_store,
};

const SESSIONS: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");
#[derive(Clone, Debug)]
pub enum Sessions {
    Redb {
        database: Arc<redb::Database>,
        blocking: Arc<tokio::sync::Semaphore>,
    },
    #[cfg(feature = "sqlite")]
    Sqlite {
        pool: sqlx::SqlitePool,
        store: tower_sessions_sqlx_store::SqliteStore,
    },
}
fn unavailable(_: impl std::fmt::Display) -> session_store::Error {
    session_store::Error::Backend("Session storage unavailable".into())
}
async fn run<T: Send + 'static>(
    database: Arc<redb::Database>,
    blocking: Arc<tokio::sync::Semaphore>,
    operation: impl FnOnce(&redb::Database) -> Result<T, StoreError> + Send + 'static,
) -> session_store::Result<T> {
    let permit = tokio::time::timeout(std::time::Duration::from_secs(5), blocking.acquire_owned())
        .await
        .map_err(unavailable)?
        .map_err(unavailable)?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        operation(&database)
    })
    .await
    .map_err(unavailable)?
    .map_err(unavailable)
}
impl Sessions {
    pub async fn new(store: &OrderStore) -> Result<Self, StoreError> {
        if let Some(database) = store.redb_database() {
            let blocking = Arc::new(tokio::sync::Semaphore::new(16));
            run(database.clone(), blocking.clone(), |db| {
                let tx = db.begin_write()?;
                {
                    tx.open_table(SESSIONS)?;
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
            return Ok(Self::Redb { database, blocking });
        }
        #[cfg(feature = "sqlite")]
        if let Some(pool) = store.sqlite_pool() {
            let store = tower_sessions_sqlx_store::SqliteStore::new(pool.clone());
            store.migrate().await?;
            return Ok(Self::Sqlite { pool, store });
        }
        Err("Database is closed".into())
    }
    /// Used only by offline restore; never exposed as an HTTP route.
    pub async fn clear(&self) -> Result<(), StoreError> {
        match self {
            Self::Redb { database, blocking } => run(database.clone(), blocking.clone(), |db| {
                let tx = db.begin_write()?;
                {
                    let mut table = tx.open_table(SESSIONS)?;
                    table.retain(|_, _| false)?;
                }
                tx.commit()?;
                Ok(())
            })
            .await
            .map_err(Into::into),
            #[cfg(feature = "sqlite")]
            Self::Sqlite { pool, .. } => {
                sqlx::query("DELETE FROM tower_sessions")
                    .execute(pool)
                    .await?;
                Ok(())
            }
        }
    }
}
#[async_trait::async_trait]
impl SessionStore for Sessions {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        match self {
            Self::Redb { database, blocking } => {
                let candidate = record.clone();
                let saved = run(database.clone(), blocking.clone(), move |db| {
                    let mut candidate = candidate;
                    let tx = db.begin_write()?;
                    {
                        let mut table = tx.open_table(SESSIONS)?;
                        loop {
                            if table.get(candidate.id.to_string().as_str())?.is_none() {
                                break;
                            }
                            candidate.id = Id::default();
                        }
                        let bytes = serde_json::to_vec(&candidate)?;
                        table.insert(candidate.id.to_string().as_str(), bytes.as_slice())?;
                    }
                    tx.commit()?;
                    Ok(candidate)
                })
                .await?;
                *record = saved;
                Ok(())
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite { store, .. } => store.create(record).await,
        }
    }
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        match self {
            Self::Redb { database, blocking } => {
                let record = record.clone();
                run(database.clone(), blocking.clone(), move |db| {
                    let tx = db.begin_write()?;
                    {
                        let mut table = tx.open_table(SESSIONS)?;
                        let id = record.id.to_string();
                        let existing: Record = {
                            let value = table.get(id.as_str())?.ok_or("Session was revoked")?;
                            serde_json::from_slice(value.value())?
                        };
                        if existing.expiry_date <= time::OffsetDateTime::now_utc() {
                            return Err("Session expired".into());
                        }
                        let bytes = serde_json::to_vec(&record)?;
                        table.insert(id.as_str(), bytes.as_slice())?;
                    }
                    tx.commit()?;
                    Ok(())
                })
                .await
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite { pool, .. } => {
                let bytes = rmp_serde::to_vec(record).map_err(unavailable)?;
                let result = sqlx::query("UPDATE tower_sessions SET data = ?, expiry_date = ? WHERE id = ? AND expiry_date > ?")
                    .bind(bytes).bind(record.expiry_date).bind(record.id.to_string()).bind(time::OffsetDateTime::now_utc())
                    .execute(pool).await.map_err(unavailable)?;
                if result.rows_affected() == 1 {
                    Ok(())
                } else {
                    Err(unavailable("Session revoked or expired"))
                }
            }
        }
    }
    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        match self {
            Self::Redb { database, blocking } => {
                let id = id.to_string();
                run(database.clone(), blocking.clone(), move |db| {
                    let tx = db.begin_read()?;
                    let table = tx.open_table(SESSIONS)?;
                    let Some(value) = table.get(id.as_str())? else {
                        return Ok(None);
                    };
                    let record: Record = serde_json::from_slice(value.value())?;
                    Ok((record.expiry_date > time::OffsetDateTime::now_utc()).then_some(record))
                })
                .await
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite { store, .. } => store.load(id).await,
        }
    }
    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        match self {
            Self::Redb { database, blocking } => {
                let id = id.to_string();
                run(database.clone(), blocking.clone(), move |db| {
                    let tx = db.begin_write()?;
                    {
                        tx.open_table(SESSIONS)?.remove(id.as_str())?;
                    }
                    tx.commit()?;
                    Ok(())
                })
                .await
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite { store, .. } => store.delete(id).await,
        }
    }
}
#[async_trait::async_trait]
impl ExpiredDeletion for Sessions {
    async fn delete_expired(&self) -> session_store::Result<()> {
        match self {
            Self::Redb { database, blocking } => {
                run(database.clone(), blocking.clone(), |db| {
                    // Scan without holding redb's single writer lock. Delete in
                    // short batches and recheck expiry because a concurrent save
                    // may have renewed a row since the read snapshot.
                    let expired = {
                        let tx = db.begin_read()?;
                        let table = tx.open_table(SESSIONS)?;
                        let mut expired = Vec::new();
                        let now = time::OffsetDateTime::now_utc();
                        for item in table.iter()? {
                            let (id, bytes) = item?;
                            let record: Record = serde_json::from_slice(bytes.value())?;
                            if record.expiry_date <= now {
                                expired.push(id.value().to_owned());
                            }
                        }
                        expired
                    };
                    for batch in expired.chunks(128) {
                        let tx = db.begin_write()?;
                        {
                            let mut table = tx.open_table(SESSIONS)?;
                            for id in batch {
                                let expired = {
                                    let value = table.get(id.as_str())?;
                                    value
                                        .map(|bytes| {
                                            serde_json::from_slice::<Record>(bytes.value())
                                        })
                                        .transpose()?
                                        .is_some_and(|record| {
                                            record.expiry_date <= time::OffsetDateTime::now_utc()
                                        })
                                };
                                if expired {
                                    table.remove(id.as_str())?;
                                }
                            }
                        }
                        tx.commit()?;
                    }
                    Ok(())
                })
                .await
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite { store, .. } => store.delete_expired().await,
        }
    }
}
