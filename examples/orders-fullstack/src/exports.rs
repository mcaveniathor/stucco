//! Bounded, principal-owned CSV exports. Apalis runs jobs; application records
//! preserve status and idempotency across refreshes and server restarts.
use serde::{Deserialize, Serialize};

pub const MAX_EXPORT_ORDERS: usize = 1_000;
pub const MAX_EXPORT_BYTES: usize = 1_048_576;
pub const EXPORT_LIFETIME_SECONDS: i64 = 86_400;
pub const MAX_EXPORT_ATTEMPTS: u32 = 3;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExportView {
    pub token: String,
    pub attempt: u32,
    pub created_at: i64,
    pub expires_at: i64,
    pub state: ExportState,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExportState {
    Queued,
    Running { processed: usize, total: usize },
    Complete { rows: usize },
    Failed(ExportFailure),
    Cancelled,
}
impl ExportState {
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Queued | Self::Running { .. })
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExportFailure {
    TooManyOrders,
    Unavailable,
}
impl std::fmt::Display for ExportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooManyOrders => "This export exceeds the 1,000-order limit.",
            Self::Unavailable => "The export could not finish. You can retry explicitly.",
        })
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum ExportError {
    NotFound,
    InvalidToken,
    Capacity,
    NotRetryable,
    AttemptsExhausted,
    Unavailable,
}
impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotFound => "Export not found or no longer available.",
            Self::InvalidToken => "Reload this page before requesting an export.",
            Self::Capacity => "The export limit is reached. Exports expire after 24 hours.",
            Self::NotRetryable => "This export cannot be retried in its current state.",
            Self::AttemptsExhausted => "This export has reached its three-attempt limit.",
            Self::Unavailable => "Exports are temporarily unavailable. Please try again.",
        })
    }
}
impl std::error::Error for ExportError {}

#[cfg(feature = "ssr")]
mod server {
    use super::*;
    use crate::{
        auth::{Principal, PrincipalKey},
        domain::Order,
        store::{OrderStore, StoreError},
    };
    use apalis::prelude::{
        Data, MemoryStorage, MessageQueue, Monitor, WorkerBuilder, WorkerBuilderExt,
        WorkerFactoryFn,
    };
    use redb::{ReadableDatabase, ReadableTable, TableDefinition};
    use std::{
        collections::{BTreeMap, BTreeSet},
        sync::{Arc, Mutex, Weak},
        time::Duration,
    };
    use tokio::sync::Semaphore;

    const JOBS: TableDefinition<&str, &[u8]> = TableDefinition::new("export_jobs");
    const RESULTS: TableDefinition<&str, &[u8]> = TableDefinition::new("export_results");
    const MAX_JOBS: usize = 64;
    const MAX_USER_JOBS: usize = 8;

    #[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
    struct Record {
        principal: PrincipalKey,
        session_epoch: i64,
        #[serde(default)]
        run_started: Option<i64>,
        view: ExportView,
    }
    impl Record {
        fn key(&self) -> String {
            storage_key(self.principal, &self.view.token)
        }
    }
    #[derive(Clone, Debug)]
    struct Work {
        key: String,
        attempt: u32,
    }
    #[derive(Clone)]
    struct Records {
        store: OrderStore,
        blocking: Arc<Semaphore>,
    }
    enum ResultChange {
        Put(String, Vec<u8>),
        Delete(String),
    }

    /// One executor per application/database. The queue contains only bounded
    /// identifiers, while status and completed bytes are stored atomically.
    #[derive(Clone)]
    pub struct Exports(Arc<Inner>);
    struct Inner {
        store: OrderStore,
        records: Records,
        queue: tokio::sync::Mutex<MemoryStorage<Work>>,
        queued: Mutex<BTreeSet<(String, u32)>>,
        runtime: Mutex<Option<tokio::task::JoinHandle<()>>>,
        stop: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    }
    impl Drop for Inner {
        fn drop(&mut self) {
            if let Ok(stop) = self.stop.get_mut()
                && let Some(stop) = stop.take()
            {
                let _ = stop.send(());
            }
            if let Ok(runtime) = self.runtime.get_mut()
                && let Some(runtime) = runtime.take()
            {
                runtime.abort();
            }
        }
    }
    impl std::fmt::Debug for Exports {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Exports").finish_non_exhaustive()
        }
    }

    /// Remove all derived exports during offline restore. Does not start work.
    pub async fn clear_stored(store: &OrderStore) -> Result<(), StoreError> {
        let records = Records {
            store: store.clone(),
            blocking: Arc::new(Semaphore::new(1)),
        };
        records.initialize().await?;
        if let Some(db) = store.redb_database() {
            tokio::task::spawn_blocking(move || -> Result<(), StoreError> {
                let tx = db.begin_write()?;
                tx.open_table(JOBS)?.retain(|_, _| false)?;
                tx.open_table(RESULTS)?.retain(|_, _| false)?;
                tx.commit()?;
                Ok(())
            })
            .await??;
            return Ok(());
        }
        #[cfg(feature = "sqlite")]
        if let Some(pool) = store.sqlite_pool() {
            let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
            sqlx::query("DELETE FROM export_results")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM export_jobs")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            return Ok(());
        }
        Err(ExportError::Unavailable.into())
    }

    pub fn new_token() -> Result<String, StoreError> {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
    }
    fn valid_token(token: &str) -> bool {
        token.len() == 32
            && token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }
    fn storage_key(principal: PrincipalKey, token: &str) -> String {
        format!(
            "{}:{}:{}:{token}",
            principal.generation, principal.tenant_id, principal.user_id
        )
    }
    fn now() -> i64 {
        time::OffsetDateTime::now_utc().unix_timestamp()
    }
    fn unavailable(_: impl std::fmt::Display) -> ExportError {
        ExportError::Unavailable
    }

    impl Exports {
        pub async fn new(store: OrderStore) -> Result<Self, StoreError> {
            let records = Records {
                store: store.clone(),
                blocking: Arc::new(Semaphore::new(2)),
            };
            records.initialize().await?;
            let generation = store.generation().await?;
            let work = records
                .change(move |rows, results| {
                    prune(rows, results);
                    rows.retain(|key, row| {
                        let keep = row.principal.generation == generation;
                        if !keep {
                            results.push(ResultChange::Delete(key.clone()));
                        }
                        keep
                    });
                    let mut work = Vec::new();
                    for row in rows.values_mut() {
                        if row.view.state.is_pending() {
                            row.view.state = ExportState::Queued;
                            row.run_started = None;
                            work.push(Work {
                                key: row.key(),
                                attempt: row.view.attempt,
                            });
                        }
                    }
                    Ok(work)
                })
                .await?;
            let queue = MemoryStorage::new();
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let inner = Arc::new(Inner {
                store,
                records,
                queue: tokio::sync::Mutex::new(queue.clone()),
                queued: Mutex::new(BTreeSet::new()),
                runtime: Mutex::new(None),
                stop: Mutex::new(Some(stop)),
            });
            let worker = WorkerBuilder::new("orders-csv")
                .concurrency(1)
                .data(Arc::downgrade(&inner))
                .backend(queue)
                .build_fn(run_export);
            let weak = Arc::downgrade(&inner);
            let runtime = tokio::spawn(async move {
                let monitor = Monitor::new().register(worker).run_with_signal(async {
                    let _ = stopped.await;
                    Ok(())
                });
                let cleanup = async move {
                    let mut interval = tokio::time::interval_at(
                        tokio::time::Instant::now() + Duration::from_secs(60),
                        Duration::from_secs(60),
                    );
                    loop {
                        interval.tick().await;
                        let Some(inner) = weak.upgrade() else {
                            return;
                        };
                        let _ = inner
                            .records
                            .change(|rows, results| {
                                prune(rows, results);
                                fail_stuck(rows);
                                Ok(())
                            })
                            .await;
                    }
                };
                tokio::select! { _ = monitor => {}, _ = cleanup => {} }
            });
            *inner.runtime.lock().map_err(|_| ExportError::Unavailable)? = Some(runtime);
            let service = Self(inner);
            for work in work {
                service.enqueue(work).await?;
            }
            Ok(service)
        }

        /// Stop accepting work and allow the current bounded export to finish.
        /// Dropping the last service handle aborts the executor as a fallback.
        pub async fn shutdown(&self) {
            let stop = self.0.stop.lock().ok().and_then(|mut stop| stop.take());
            if let Some(stop) = stop {
                let _ = stop.send(());
            }
            let runtime = self
                .0
                .runtime
                .lock()
                .ok()
                .and_then(|mut runtime| runtime.take());
            if let Some(mut runtime) = runtime
                && tokio::time::timeout(Duration::from_secs(5), &mut runtime)
                    .await
                    .is_err()
            {
                runtime.abort();
                let _ = runtime.await;
            }
        }

        pub async fn start(
            &self,
            principal: &Principal,
            token: String,
        ) -> Result<ExportView, ExportError> {
            if !self.is_active()? {
                return Err(ExportError::Unavailable);
            }
            if !valid_token(&token) {
                return Err(ExportError::InvalidToken);
            }
            let epoch = self.current_epoch(principal.key).await?;
            let principal = principal.key;
            let key = storage_key(principal, &token);
            let (view, enqueue) = self
                .0
                .records
                .change(move |rows, results| {
                    prune(rows, results);
                    // Revoked sessions cannot access or cancel their old rows.
                    // Remove those rows and bytes before checking capacity.
                    rows.retain(|key, row| {
                        let keep = row.principal != principal || row.session_epoch == epoch;
                        if !keep {
                            results.push(ResultChange::Delete(key.clone()));
                        }
                        keep
                    });
                    if let Some(row) = rows.get(&key) {
                        return if row.session_epoch == epoch {
                            Ok((row.view.clone(), row.view.state == ExportState::Queued))
                        } else {
                            Err(ExportError::NotFound)
                        };
                    }
                    if rows.len() >= MAX_JOBS
                        || rows
                            .values()
                            .filter(|row| row.principal == principal)
                            .count()
                            >= MAX_USER_JOBS
                    {
                        return Err(ExportError::Capacity);
                    }
                    let created_at = now();
                    let view = ExportView {
                        token,
                        attempt: 1,
                        created_at,
                        expires_at: created_at + EXPORT_LIFETIME_SECONDS,
                        state: ExportState::Queued,
                    };
                    rows.insert(
                        key,
                        Record {
                            principal,
                            session_epoch: epoch,
                            run_started: None,
                            view: view.clone(),
                        },
                    );
                    Ok((view, true))
                })
                .await?;
            if enqueue {
                self.enqueue(Work {
                    key: storage_key(principal, &view.token),
                    attempt: view.attempt,
                })
                .await?;
            }
            Ok(view)
        }

        pub async fn list(&self, principal: &Principal) -> Result<Vec<ExportView>, ExportError> {
            let epoch = self.current_epoch(principal.key).await?;
            let principal = principal.key;
            let mut rows = self
                .0
                .records
                .read()
                .await?
                .into_values()
                .filter(|row| {
                    row.principal == principal
                        && row.session_epoch == epoch
                        && row.view.expires_at > now()
                })
                .map(|row| row.view)
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| {
                b.created_at
                    .cmp(&a.created_at)
                    .then_with(|| b.token.cmp(&a.token))
            });
            Ok(rows)
        }
        pub async fn status(
            &self,
            principal: &Principal,
            token: &str,
        ) -> Result<ExportView, ExportError> {
            let row = self.record(principal, token).await?;
            if row.view.state == ExportState::Queued && self.is_active()? {
                self.enqueue(Work {
                    key: row.key(),
                    attempt: row.view.attempt,
                })
                .await?;
            }
            Ok(row.view)
        }
        pub async fn retry(
            &self,
            principal: &Principal,
            token: &str,
            expected_attempt: u32,
        ) -> Result<ExportView, ExportError> {
            if !self.is_active()? {
                return Err(ExportError::Unavailable);
            }
            let original = self.record(principal, token).await?;
            let key = original.key();
            let (view, enqueue) = self
                .0
                .records
                .change(move |rows, _| {
                    let row = rows.get_mut(&key).ok_or(ExportError::NotFound)?;
                    if row.view.expires_at <= now() || row.session_epoch != original.session_epoch {
                        return Err(ExportError::NotFound);
                    }
                    // A retry whose response was lost observes the existing attempt.
                    if expected_attempt < row.view.attempt {
                        return Ok((row.view.clone(), row.view.state == ExportState::Queued));
                    }
                    if expected_attempt != row.view.attempt
                        || !matches!(row.view.state, ExportState::Failed(_))
                    {
                        return Err(ExportError::NotRetryable);
                    }
                    if row.view.attempt >= MAX_EXPORT_ATTEMPTS {
                        return Err(ExportError::AttemptsExhausted);
                    }
                    row.view.attempt += 1;
                    row.view.state = ExportState::Queued;
                    Ok((row.view.clone(), true))
                })
                .await?;
            if enqueue {
                self.enqueue(Work {
                    key: storage_key(principal.key, &view.token),
                    attempt: view.attempt,
                })
                .await?;
            }
            Ok(view)
        }
        pub async fn cancel(
            &self,
            principal: &Principal,
            token: &str,
        ) -> Result<ExportView, ExportError> {
            let original = self.record(principal, token).await?;
            let key = original.key();
            self.0
                .records
                .change(move |rows, results| {
                    let row = rows.get_mut(&key).ok_or(ExportError::NotFound)?;
                    if row.view.expires_at <= now() || row.session_epoch != original.session_epoch {
                        return Err(ExportError::NotFound);
                    }
                    row.view.state = ExportState::Cancelled;
                    results.push(ResultChange::Delete(key));
                    Ok(row.view.clone())
                })
                .await
        }
        pub async fn download(
            &self,
            principal: &Principal,
            token: &str,
        ) -> Result<Vec<u8>, ExportError> {
            if !valid_token(token) {
                return Err(ExportError::NotFound);
            }
            self.0
                .records
                .authorized_result(principal.key, token.to_owned())
                .await?
                .ok_or(ExportError::NotFound)
        }
        async fn record(&self, principal: &Principal, token: &str) -> Result<Record, ExportError> {
            if !valid_token(token) {
                return Err(ExportError::NotFound);
            }
            let epoch = self.current_epoch(principal.key).await?;
            self.0
                .records
                .read()
                .await?
                .remove(&storage_key(principal.key, token))
                .filter(|row| row.session_epoch == epoch && row.view.expires_at > now())
                .ok_or(ExportError::NotFound)
        }
        async fn current_epoch(&self, principal: PrincipalKey) -> Result<i64, ExportError> {
            current_epoch(&self.0.store, principal).await
        }
        fn is_active(&self) -> Result<bool, ExportError> {
            Ok(self
                .0
                .runtime
                .lock()
                .map_err(unavailable)?
                .as_ref()
                .is_some_and(|runtime| !runtime.is_finished()))
        }
        async fn enqueue(&self, work: Work) -> Result<(), ExportError> {
            if !self.is_active()? {
                return Err(ExportError::Unavailable);
            }
            // Acquire before marking queued: a cancelled HTTP request must not
            // leave an identifier recorded without a matching queue entry.
            let mut queue = self.0.queue.lock().await;
            let id = (work.key.clone(), work.attempt);
            if !self
                .0
                .queued
                .lock()
                .map_err(unavailable)?
                .insert(id.clone())
            {
                return Ok(());
            }
            if queue.enqueue(work).await.is_err() {
                self.0.queued.lock().map_err(unavailable)?.remove(&id);
                return Err(ExportError::Unavailable);
            }
            Ok(())
        }
    }
    async fn current_epoch(
        store: &OrderStore,
        principal: PrincipalKey,
    ) -> Result<i64, ExportError> {
        if store.generation().await.map_err(unavailable)? != principal.generation {
            return Err(ExportError::NotFound);
        }
        store
            .find_user_by_id(principal.user_id)
            .await
            .map_err(unavailable)?
            .filter(|user| user.enabled && user.tenant_id == principal.tenant_id)
            .map(|user| user.session_epoch)
            .ok_or(ExportError::NotFound)
    }
    fn fail_stuck(rows: &mut BTreeMap<String, Record>) {
        let now = now();
        for row in rows.values_mut() {
            if matches!(row.view.state, ExportState::Running { .. })
                && row.run_started.is_some_and(|started| started + 60 <= now)
            {
                row.view.state = ExportState::Failed(ExportFailure::Unavailable);
            }
        }
    }
    fn prune(rows: &mut BTreeMap<String, Record>, results: &mut Vec<ResultChange>) {
        let now = now();
        rows.retain(|key, row| {
            if row.view.expires_at <= now {
                results.push(ResultChange::Delete(key.clone()));
                false
            } else {
                true
            }
        });
    }

    async fn run_export(work: Work, weak: Data<Weak<Inner>>) -> Result<(), std::io::Error> {
        let Some(inner) = weak.upgrade() else {
            return Ok(());
        };
        let service = Exports(inner);
        if let Ok(mut queued) = service.0.queued.lock() {
            queued.remove(&(work.key.clone(), work.attempt));
        }
        // Claim once. Duplicate queue delivery and stale attempts are harmless.
        let key = work.key.clone();
        let attempt = work.attempt;
        let claimed = service
            .0
            .records
            .change(move |rows, _| {
                let Some(row) = rows.get_mut(&key) else {
                    return Ok(None);
                };
                if row.view.attempt != attempt
                    || row.view.state != ExportState::Queued
                    || row.view.expires_at <= now()
                {
                    return Ok(None);
                }
                row.run_started = Some(now());
                row.view.state = ExportState::Running {
                    processed: 0,
                    total: 0,
                };
                Ok(Some(row.clone()))
            })
            .await;
        let Ok(Some(row)) = claimed else {
            return Ok(());
        };
        let result = tokio::time::timeout(Duration::from_secs(30), generate(&service, &work, &row))
            .await
            .unwrap_or(Err(WorkError::Failed(ExportFailure::Unavailable)));
        let key = work.key;
        let _ = service
            .0
            .records
            .change(move |rows, results| {
                let Some(current) = rows.get_mut(&key) else {
                    return Ok(());
                };
                if current.view.attempt != attempt
                    || !matches!(current.view.state, ExportState::Running { .. })
                {
                    return Ok(());
                }
                match result {
                    Ok((csv, rows)) if current.view.expires_at > now() => {
                        current.view.state = ExportState::Complete { rows };
                        results.push(ResultChange::Put(key, csv));
                    }
                    Err(WorkError::Cancelled) | Ok(_) => {
                        current.view.state = ExportState::Cancelled
                    }
                    Err(WorkError::Failed(error)) => {
                        current.view.state = ExportState::Failed(error)
                    }
                }
                Ok(())
            })
            .await;
        Ok(())
    }
    enum WorkError {
        Cancelled,
        Failed(ExportFailure),
    }
    async fn generate(
        service: &Exports,
        work: &Work,
        row: &Record,
    ) -> Result<(Vec<u8>, usize), WorkError> {
        let epoch = service
            .current_epoch(row.principal)
            .await
            .map_err(|_| WorkError::Cancelled)?;
        if epoch != row.session_epoch {
            return Err(WorkError::Cancelled);
        }
        let principal = Principal {
            key: row.principal,
            display_name: String::new(),
        };
        let orders = service
            .0
            .store
            .export_snapshot(&principal, MAX_EXPORT_ORDERS)
            .await
            .map_err(|error| {
                let too_large = error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|error| error.kind() == std::io::ErrorKind::InvalidInput);
                WorkError::Failed(if too_large {
                    ExportFailure::TooManyOrders
                } else {
                    ExportFailure::Unavailable
                })
            })?;
        if orders.len() > MAX_EXPORT_ORDERS {
            return Err(WorkError::Failed(ExportFailure::TooManyOrders));
        }
        let mut csv =
            String::from("order_id,revision,customer,quantity_one,quantity_two,total_cents\r\n");
        for (index, order) in orders.iter().enumerate() {
            append_csv(&mut csv, order).map_err(WorkError::Failed)?;
            if csv.len() > MAX_EXPORT_BYTES {
                return Err(WorkError::Failed(ExportFailure::TooManyOrders));
            }
            if index % 50 == 0 {
                let key = work.key.clone();
                let attempt = work.attempt;
                let total = orders.len();
                let active = service
                    .0
                    .records
                    .change(move |rows, _| {
                        let Some(row) = rows.get_mut(&key) else {
                            return Ok(false);
                        };
                        if row.view.attempt != attempt
                            || !matches!(row.view.state, ExportState::Running { .. })
                            || row.view.expires_at <= now()
                        {
                            return Ok(false);
                        }
                        row.view.state = ExportState::Running {
                            processed: index + 1,
                            total,
                        };
                        Ok(true)
                    })
                    .await
                    .map_err(|_| WorkError::Failed(ExportFailure::Unavailable))?;
                if !active {
                    return Err(WorkError::Cancelled);
                }
                tokio::task::yield_now().await;
            }
        }
        if service.current_epoch(row.principal).await.ok() != Some(row.session_epoch) {
            return Err(WorkError::Cancelled);
        }
        Ok((csv.into_bytes(), orders.len()))
    }
    fn append_csv(output: &mut String, order: &Order) -> Result<(), ExportFailure> {
        use std::fmt::Write;
        let mut customer = order.customer.clone();
        // Spreadsheet programs may interpret quoted cells as formulas. Prefix
        // dangerous starts after whitespace; quote every customer cell as CSV.
        if customer.trim_start().starts_with(['=', '+', '-', '@'])
            || customer.starts_with(['\t', '\r', '\n'])
        {
            customer.insert(0, '\'');
        }
        let customer = customer.replace('"', "\"\"");
        let total = order
            .total_cents()
            .map_err(|_| ExportFailure::Unavailable)?;
        write!(
            output,
            "{},{},\"{}\",{},{},{}\r\n",
            order.id,
            order.revision,
            customer,
            order.items[0].quantity,
            order.items[1].quantity,
            total
        )
        .map_err(|_| ExportFailure::Unavailable)
    }

    fn download_allowed(row: &Record, principal: PrincipalKey, epoch: i64) -> bool {
        row.principal == principal
            && row.session_epoch == epoch
            && row.view.expires_at > now()
            && matches!(row.view.state, ExportState::Complete { .. })
    }

    impl Records {
        async fn initialize(&self) -> Result<(), StoreError> {
            if let Some(db) = self.store.redb_database() {
                tokio::task::spawn_blocking(move || -> Result<(), StoreError> {
                    let tx = db.begin_write()?;
                    tx.open_table(JOBS)?;
                    tx.open_table(RESULTS)?;
                    tx.commit()?;
                    Ok(())
                })
                .await??;
                return Ok(());
            }
            #[cfg(feature = "sqlite")]
            if let Some(pool) = self.store.sqlite_pool() {
                sqlx::query("CREATE TABLE IF NOT EXISTS export_jobs (key TEXT PRIMARY KEY, record TEXT NOT NULL)").execute(&pool).await?;
                sqlx::query("CREATE TABLE IF NOT EXISTS export_results (key TEXT PRIMARY KEY REFERENCES export_jobs(key) ON DELETE CASCADE, data BLOB NOT NULL)").execute(&pool).await?;
                return Ok(());
            }
            Err(ExportError::Unavailable.into())
        }
        async fn blocking<T: Send + 'static>(
            &self,
            operation: impl FnOnce(&redb::Database) -> Result<T, StoreError> + Send + 'static,
        ) -> Result<T, ExportError> {
            let permit = tokio::time::timeout(
                Duration::from_secs(5),
                self.blocking.clone().acquire_owned(),
            )
            .await
            .map_err(unavailable)?
            .map_err(unavailable)?;
            let database = self.store.redb_database().ok_or(ExportError::Unavailable)?;
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                operation(&database)
            })
            .await
            .map_err(unavailable)?
            .map_err(unavailable)
        }
        async fn read(&self) -> Result<BTreeMap<String, Record>, ExportError> {
            if self.store.redb_database().is_some() {
                return self
                    .blocking(|db| {
                        let tx = db.begin_read()?;
                        let table = tx.open_table(JOBS)?;
                        table
                            .iter()?
                            .map(|entry| {
                                let (key, value) = entry?;
                                Ok((
                                    key.value().to_owned(),
                                    serde_json::from_slice(value.value())?,
                                ))
                            })
                            .collect()
                    })
                    .await;
            }
            #[cfg(feature = "sqlite")]
            if let Some(pool) = self.store.sqlite_pool() {
                let rows =
                    sqlx::query_as::<_, (String, String)>("SELECT key, record FROM export_jobs")
                        .fetch_all(&pool)
                        .await
                        .map_err(unavailable)?;
                return rows
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((key, serde_json::from_str(&value).map_err(unavailable)?))
                    })
                    .collect();
            }
            Err(ExportError::Unavailable)
        }
        async fn authorized_result(
            &self,
            principal: PrincipalKey,
            token: String,
        ) -> Result<Option<Vec<u8>>, ExportError> {
            let key = storage_key(principal, &token);
            if self.store.redb_database().is_some() {
                return self
                    .blocking(move |db| {
                        const META: TableDefinition<&str, u64> = TableDefinition::new("metadata");
                        const USERS: TableDefinition<i64, &[u8]> = TableDefinition::new("users");
                        let tx = db.begin_read()?;
                        if tx
                            .open_table(META)?
                            .get("generation")?
                            .map(|value| value.value())
                            != Some(principal.generation)
                        {
                            return Ok(None);
                        }
                        let users = tx.open_table(USERS)?;
                        let Some(user) = users.get(principal.user_id)? else {
                            return Ok(None);
                        };
                        let user: crate::store::UserRecord = serde_json::from_slice(user.value())?;
                        if !user.enabled || user.tenant_id != principal.tenant_id {
                            return Ok(None);
                        }
                        let jobs = tx.open_table(JOBS)?;
                        let Some(row) = jobs.get(key.as_str())? else {
                            return Ok(None);
                        };
                        let row: Record = serde_json::from_slice(row.value())?;
                        if !download_allowed(&row, principal, user.session_epoch) {
                            return Ok(None);
                        }
                        let results = tx.open_table(RESULTS)?;
                        Ok(results
                            .get(key.as_str())?
                            .filter(|bytes| bytes.value().len() <= MAX_EXPORT_BYTES)
                            .map(|bytes| bytes.value().to_vec()))
                    })
                    .await;
            }
            #[cfg(feature = "sqlite")]
            if let Some(pool) = self.store.sqlite_pool() {
                let row = sqlx::query_as::<_, (String, Vec<u8>, i64)>(
                    "SELECT j.record, r.data, u.session_epoch FROM export_jobs j JOIN export_results r ON r.key = j.key JOIN users u ON u.id = ? JOIN database_metadata m ON m.name = 'generation' WHERE j.key = ? AND u.tenant_id = ? AND u.enabled = 1 AND m.value = ? AND length(r.data) <= ?"
                ).bind(principal.user_id).bind(key).bind(principal.tenant_id).bind(principal.generation.to_string()).bind(MAX_EXPORT_BYTES as i64).fetch_optional(&pool).await.map_err(unavailable)?;
                let Some((record, bytes, epoch)) = row else {
                    return Ok(None);
                };
                let row: Record = serde_json::from_str(&record).map_err(unavailable)?;
                return Ok(download_allowed(&row, principal, epoch).then_some(bytes));
            }
            Err(ExportError::Unavailable)
        }
        #[cfg(test)]
        async fn result(&self, key: String) -> Result<Option<Vec<u8>>, ExportError> {
            if self.store.redb_database().is_some() {
                return self
                    .blocking(move |db| {
                        let tx = db.begin_read()?;
                        let table = tx.open_table(RESULTS)?;
                        Ok(table.get(key.as_str())?.map(|value| value.value().to_vec()))
                    })
                    .await;
            }
            #[cfg(feature = "sqlite")]
            if let Some(pool) = self.store.sqlite_pool() {
                return sqlx::query_scalar::<_, Vec<u8>>(
                    "SELECT data FROM export_results WHERE key = ?",
                )
                .bind(key)
                .fetch_optional(&pool)
                .await
                .map_err(unavailable);
            }
            Err(ExportError::Unavailable)
        }
        async fn change<T: Send + 'static>(
            &self,
            operation: impl FnOnce(
                &mut BTreeMap<String, Record>,
                &mut Vec<ResultChange>,
            ) -> Result<T, ExportError>
            + Send
            + 'static,
        ) -> Result<T, ExportError> {
            if self.store.redb_database().is_some() {
                return self
                    .blocking(move |db| {
                        let tx = db.begin_write()?;
                        let value;
                        {
                            let mut table = tx.open_table(JOBS)?;
                            let original = table
                                .iter()?
                                .map(|entry| {
                                    let (key, value) = entry?;
                                    Ok((
                                        key.value().to_owned(),
                                        serde_json::from_slice(value.value())?,
                                    ))
                                })
                                .collect::<Result<BTreeMap<String, Record>, StoreError>>()?;
                            let mut rows = original.clone();
                            let mut results = Vec::new();
                            value = operation(&mut rows, &mut results);
                            if value.is_err() {
                                return Ok(value);
                            }
                            for key in original.keys().filter(|key| !rows.contains_key(*key)) {
                                table.remove(key.as_str())?;
                            }
                            for (key, row) in &rows {
                                if original.get(key) != Some(row) {
                                    table.insert(
                                        key.as_str(),
                                        serde_json::to_vec(row)?.as_slice(),
                                    )?;
                                }
                            }
                            let mut result_table = tx.open_table(RESULTS)?;
                            for result in results {
                                match result {
                                    ResultChange::Put(key, bytes) => {
                                        result_table.insert(key.as_str(), bytes.as_slice())?;
                                    }
                                    ResultChange::Delete(key) => {
                                        result_table.remove(key.as_str())?;
                                    }
                                }
                            }
                        }
                        tx.commit()?;
                        Ok(value)
                    })
                    .await?;
            }
            #[cfg(feature = "sqlite")]
            if let Some(pool) = self.store.sqlite_pool() {
                let mut tx = pool
                    .begin_with("BEGIN IMMEDIATE")
                    .await
                    .map_err(unavailable)?;
                let raw =
                    sqlx::query_as::<_, (String, String)>("SELECT key, record FROM export_jobs")
                        .fetch_all(&mut *tx)
                        .await
                        .map_err(unavailable)?;
                let original = raw
                    .into_iter()
                    .map(|(key, value)| {
                        Ok((key, serde_json::from_str(&value).map_err(unavailable)?))
                    })
                    .collect::<Result<BTreeMap<String, Record>, ExportError>>()?;
                let mut rows = original.clone();
                let mut results = Vec::new();
                let value = operation(&mut rows, &mut results)?;
                for key in original.keys().filter(|key| !rows.contains_key(*key)) {
                    sqlx::query("DELETE FROM export_jobs WHERE key = ?")
                        .bind(key)
                        .execute(&mut *tx)
                        .await
                        .map_err(unavailable)?;
                }
                for (key, row) in &rows {
                    if original.get(key) != Some(row) {
                        sqlx::query("INSERT INTO export_jobs (key, record) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET record = excluded.record").bind(key).bind(serde_json::to_string(row).map_err(unavailable)?).execute(&mut *tx).await.map_err(unavailable)?;
                    }
                }
                for result in results {
                    match result {
                        ResultChange::Put(key, bytes) => {
                            sqlx::query("INSERT INTO export_results (key, data) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET data = excluded.data").bind(key).bind(bytes).execute(&mut *tx).await.map_err(unavailable)?;
                        }
                        ResultChange::Delete(key) => {
                            sqlx::query("DELETE FROM export_results WHERE key = ?")
                                .bind(key)
                                .execute(&mut *tx)
                                .await
                                .map_err(unavailable)?;
                        }
                    }
                }
                tx.commit().await.map_err(unavailable)?;
                return Ok(value);
            }
            Err(ExportError::Unavailable)
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        async fn fixture() -> (tempfile::TempDir, OrderStore, Principal) {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("exports.db");
            #[cfg(feature = "sqlite")]
            let store = OrderStore::open_sqlite(&path).await.unwrap();
            #[cfg(not(feature = "sqlite"))]
            let store = OrderStore::open(&path).await.unwrap();
            let principal = store
                .insert_user("tenant", "user", "User", "fixture")
                .await
                .unwrap();
            (directory, store, principal)
        }

        #[tokio::test]
        async fn interrupted_running_record_is_reconstructed_without_another_attempt() {
            let (_directory, store, principal) = fixture().await;
            let records = Records {
                store: store.clone(),
                blocking: Arc::new(Semaphore::new(1)),
            };
            records.initialize().await.unwrap();
            // This persisted fixture represents a process that stopped after its
            // claim transaction committed and before publishing the CSV.
            let token = new_token().unwrap();
            let record = Record {
                principal: principal.key,
                session_epoch: 0,
                run_started: Some(now() - 120),
                view: ExportView {
                    token: token.clone(),
                    attempt: 2,
                    created_at: now(),
                    expires_at: now() + 100,
                    state: ExportState::Running {
                        processed: 1,
                        total: 2,
                    },
                },
            };
            records
                .change(move |rows, _| {
                    rows.insert(record.key(), record);
                    Ok(())
                })
                .await
                .unwrap();
            let service = Exports::new(store).await.unwrap();
            let view = tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let view = service.status(&principal, &token).await.unwrap();
                    if !view.state.is_pending() {
                        break view;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            assert_eq!(view.attempt, 2);
            assert_eq!(view.state, ExportState::Complete { rows: 0 });
        }

        #[tokio::test]
        async fn revocation_discards_hidden_records_and_bytes_before_user_capacity_check() {
            let (_directory, store, principal) = fixture().await;
            let records = Records {
                store: store.clone(),
                blocking: Arc::new(Semaphore::new(1)),
            };
            records.initialize().await.unwrap();
            let mut keys = Vec::new();
            for _ in 0..MAX_USER_JOBS {
                let row = Record {
                    principal: principal.key,
                    session_epoch: 0,
                    run_started: None,
                    view: ExportView {
                        token: new_token().unwrap(),
                        attempt: 1,
                        created_at: now(),
                        expires_at: now() + 100,
                        state: ExportState::Complete { rows: 1 },
                    },
                };
                keys.push(row.key());
                records
                    .change(move |rows, results| {
                        results.push(ResultChange::Put(
                            row.key(),
                            b"revoked private output".to_vec(),
                        ));
                        rows.insert(row.key(), row);
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            let service = Exports::new(store.clone()).await.unwrap();
            store.revoke_sessions("user").await.unwrap();
            assert!(service.list(&principal).await.unwrap().is_empty());
            service
                .start(&principal, new_token().unwrap())
                .await
                .unwrap();
            assert_eq!(service.list(&principal).await.unwrap().len(), 1);
            for key in keys {
                assert!(records.result(key).await.unwrap().is_none());
            }
        }

        #[tokio::test]
        async fn expired_jobs_and_result_bytes_are_removed_before_capacity_checks() {
            let (_directory, store, principal) = fixture().await;
            let records = Records {
                store: store.clone(),
                blocking: Arc::new(Semaphore::new(1)),
            };
            records.initialize().await.unwrap();
            let token = new_token().unwrap();
            let key = storage_key(principal.key, &token);
            let record = Record {
                principal: principal.key,
                session_epoch: 0,
                run_started: None,
                view: ExportView {
                    token,
                    attempt: 1,
                    created_at: now() - 200,
                    expires_at: now() - 100,
                    state: ExportState::Complete { rows: 1 },
                },
            };
            records
                .change(move |rows, results| {
                    results.push(ResultChange::Put(
                        record.key(),
                        b"private expired output".to_vec(),
                    ));
                    rows.insert(record.key(), record);
                    Ok(())
                })
                .await
                .unwrap();
            let service = Exports::new(store).await.unwrap();
            assert!(service.list(&principal).await.unwrap().is_empty());
            assert!(records.result(key).await.unwrap().is_none());
            service
                .start(&principal, new_token().unwrap())
                .await
                .unwrap();
        }
    }
}
#[cfg(feature = "ssr")]
pub use server::{Exports, clear_stored, new_token};
