//! Durable at-least-once jobs; external effects must be idempotent.
use crate::{IndexTable, PostcardCodec, Store, StoreError, Table, Utf8Key, WriteTx};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::num::{NonZeroU32, NonZeroU64};
use stucco_core::Direction;

/// Persistent job lifecycle. Times are caller-supplied Unix seconds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobState {
    /// Eligible at or after this instant.
    Ready {
        /// Next eligible instant.
        at: u64,
    },
    /// A worker owns this attempt until the exclusive deadline.
    Leased {
        /// Lease expiration, exclusive.
        until: u64,
    },
    /// Acknowledged; the deduplication tombstone is retained.
    Done,
    /// Attempt budget exhausted; retained for inspection/manual intervention.
    Failed,
}

/// A job and its immutable payload. Queue keys must be globally unique within
/// the queue; use a tenant-prefixed operation id, never just a public tag id.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job<V> {
    /// Application's permanent idempotency key.
    pub key: String,
    /// Application-defined payload; authorization is not inferred from it.
    pub payload: V,
    /// Number of leases issued (including expired/crashed attempts).
    pub attempts: u32,
    /// Maximum attempts supplied at enqueue time.
    pub max_attempts: u32,
    /// Current durable state.
    pub state: JobState,
}

#[derive(Serialize, Deserialize)]
struct Wake {
    key: String,
    at: u64,
}

type Jobs<V> = Table<String, Job<V>, Utf8Key, PostcardCodec<Job<V>>>;
type Wakes = IndexTable<String, Wake, Utf8Key, PostcardCodec<Wake>>;

/// A claimed attempt. Keep it until acknowledgement/retry. Its token fences
/// queue metadata only, not network or filesystem effects by stale workers.
#[derive(Clone, Debug)]
pub struct Lease<V> {
    owner: Store,
    queue: String,
    key: String,
    attempt: u32,
    until: u64,
    payload: V,
}
impl<V> Lease<V> {
    /// Permanent job identity.
    pub fn key(&self) -> &str {
        &self.key
    }
    /// Monotonic attempt token, never reused for this key.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }
    /// Exclusive lease deadline.
    pub fn until(&self) -> u64 {
        self.until
    }
    /// Application work. Do not perform I/O inside a database transaction.
    pub fn payload(&self) -> &V {
        &self.payload
    }
}

/// An opt-in durable queue (`jobs` feature), with transactional enqueue,
/// claim, acknowledgement and scheduled retries. No worker runtime is started.
///
/// All commands accept the caller's WriteTx, allowing application state and job
/// effects to commit together. A separate bounded due index excludes terminal
/// history. Completed/failed jobs remain forever to prevent key/token reuse;
/// retention and any explicit archival policy are an application decision.
pub struct JobQueue<V> {
    jobs: Jobs<V>,
    wakes: Wakes,
}
impl<V> Clone for JobQueue<V> {
    fn clone(&self) -> Self {
        Self {
            jobs: self.jobs.clone(),
            wakes: self.wakes.clone(),
        }
    }
}
impl<V> std::fmt::Debug for JobQueue<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobQueue")
            .field("jobs", &self.jobs)
            .finish_non_exhaustive()
    }
}

impl<V: Serialize + DeserializeOwned + 'static> JobQueue<V> {
    /// Opens dedicated `name` and `name_due` tables. Do not reuse those names
    /// elsewhere; names ending in `_due` are reserved. Payload schema migration
    /// is owned by the application.
    pub fn open(store: &Store, name: &str) -> Result<Self, StoreError> {
        if name.ends_with("_due") {
            return Err(StoreError::invalid(
                "queue names cannot end in reserved _due suffix",
            ));
        }
        let due = format!("{name}_due");
        crate::store::validate_name(&due)?;
        Ok(Self {
            jobs: store.table(name, Utf8Key, PostcardCodec::default())?,
            wakes: store
                .table(&due, Utf8Key, PostcardCodec::default())?
                .index("due", |wake: &Wake| wake.at.to_be_bytes().to_vec())?,
        })
    }

    /// Enqueues once. False means the key already exists, even if Done/Failed;
    /// the original payload, budget and schedule are never overwritten.
    pub fn enqueue(
        &self,
        tx: &mut WriteTx,
        key: &str,
        payload: V,
        at: u64,
        max_attempts: NonZeroU32,
    ) -> Result<bool, StoreError> {
        if key.is_empty() || key.len() > 1024 {
            return Err(StoreError::invalid("job key must contain 1–1024 bytes"));
        }
        let key = key.to_owned();
        if tx.get(&self.jobs, &key)?.is_some() {
            return Ok(false);
        }
        let job = Job {
            key: key.clone(),
            payload,
            attempts: 0,
            max_attempts: max_attempts.get(),
            state: JobState::Ready { at },
        };
        tx.put(&self.jobs, &key, &job)?;
        tx.put(
            &self.wakes,
            &key,
            &Wake {
                key: key.clone(),
                at,
            },
        )?;
        Ok(true)
    }

    /// Inspects durable state inside the caller's transaction, including history.
    pub fn get(&self, tx: &WriteTx, key: &str) -> Result<Option<Job<V>>, StoreError> {
        tx.get(&self.jobs, &key.to_owned())
    }

    /// Claims one due job or expired lease. Processes at most 64 exhausted
    /// attempts per call; None may mean that maintenance made progress, so poll
    /// again later. At the deadline a lease is expired, never acknowledgeable.
    pub fn claim(
        &self,
        tx: &mut WriteTx,
        now: u64,
        lease_seconds: NonZeroU64,
    ) -> Result<Option<Lease<V>>, StoreError> {
        let until = now
            .checked_add(lease_seconds.get())
            .ok_or_else(|| StoreError::invalid("lease deadline overflow"))?;
        let upper = now.to_be_bytes();
        let due = tx.index_range(&self.wakes, ..=upper.as_slice(), Direction::Asc, 64)?;
        for wake in due {
            let mut job = tx
                .get(&self.jobs, &wake.key)?
                .ok_or_else(|| StoreError::invalid("job due index points to missing record"))?;
            match job.state {
                JobState::Ready { at } if at <= now => {}
                JobState::Leased { until } if until <= now => {}
                _ => return Err(StoreError::invalid("job due index disagrees with state")),
            }
            if job.attempts >= job.max_attempts {
                job.state = JobState::Failed;
                tx.put(&self.jobs, &wake.key, &job)?;
                tx.remove(&self.wakes, &wake.key)?;
                continue;
            }
            job.attempts += 1;
            job.state = JobState::Leased { until };
            tx.put(&self.jobs, &wake.key, &job)?;
            tx.put(
                &self.wakes,
                &wake.key,
                &Wake {
                    key: wake.key.clone(),
                    at: until,
                },
            )?;
            return Ok(Some(Lease {
                owner: self.jobs.store.clone(),
                queue: self.jobs.name.clone(),
                key: wake.key,
                attempt: job.attempts,
                until,
                payload: job.payload,
            }));
        }
        Ok(None)
    }

    fn current(
        &self,
        tx: &WriteTx,
        lease: &Lease<V>,
        now: u64,
    ) -> Result<Option<Job<V>>, StoreError> {
        if lease.queue != self.jobs.name
            || !std::sync::Arc::ptr_eq(&lease.owner.db, &self.jobs.store.db)
        {
            return Ok(None);
        }
        Ok(tx.get(&self.jobs, &lease.key)?.filter(|job| {
            job.attempts == lease.attempt && matches!(job.state, JobState::Leased { until } if until > now && until == lease.until)
        }))
    }

    /// Acknowledges the current live attempt. False means stale/expired/already
    /// finished: the caller must not commit its app effect on a false result.
    /// Keep app effects and this acknowledgement in one transaction.
    pub fn acknowledge(
        &self,
        tx: &mut WriteTx,
        lease: &Lease<V>,
        now: u64,
    ) -> Result<bool, StoreError> {
        let Some(mut job) = self.current(tx, lease, now)? else {
            return Ok(false);
        };
        job.state = JobState::Done;
        tx.put(&self.jobs, &lease.key, &job)?;
        tx.remove(&self.wakes, &lease.key)?;
        Ok(true)
    }

    /// Releases the live attempt for retry at `retry_at`, or marks Failed if its
    /// budget is exhausted. Caller chooses backoff; past schedules are rejected.
    /// False means the lease is stale and no state changed.
    pub fn retry(
        &self,
        tx: &mut WriteTx,
        lease: &Lease<V>,
        now: u64,
        retry_at: u64,
    ) -> Result<bool, StoreError> {
        if retry_at < now {
            return Err(StoreError::invalid("retry cannot be scheduled in the past"));
        }
        let Some(mut job) = self.current(tx, lease, now)? else {
            return Ok(false);
        };
        job.state = if job.attempts >= job.max_attempts {
            JobState::Failed
        } else {
            JobState::Ready { at: retry_at }
        };
        tx.put(&self.jobs, &lease.key, &job)?;
        match job.state {
            JobState::Failed => {
                tx.remove(&self.wakes, &lease.key)?;
            }
            _ => {
                tx.put(
                    &self.wakes,
                    &lease.key,
                    &Wake {
                        key: lease.key.clone(),
                        at: retry_at,
                    },
                )?;
            }
        }
        Ok(true)
    }
}
