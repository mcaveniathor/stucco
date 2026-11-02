# Startup migrations and durable work

`stucco-redb` keeps these mechanisms opt-in; they add no facade defaults and work
with synchronous storage without Tokio (`--no-default-features --features
migrations,jobs`). The crate's minimum Rust version remains 1.90.

## Startup order

1. Open Store, typed record/index handles, Migrations history and JobQueue handles.
2. Run the complete Migration plan before exposing application handles to requests.
   A callback uses only the provided WriteTx. No nested writes or external I/O.
3. Start the server and bounded worker loop only after successful migration.

Every step has a contiguous version and immutable identity/revision. The runner
compares all stored identities, refuses future history, then executes all pending
steps and writes history in one transaction. A failure or panic rolls the batch
back. Transform logic belongs to the application and must preserve its indexes.

Existing application schema markers need a deliberate bridge migration. Read
and validate the old marker inside the transaction; decode old records with
frozen codecs; transform/backfill and update that marker atomically. Reject unknown
future layouts. A schema marker alone cannot add a field to a Postcard record.
Keep committed-old-data fixture tests plus rollback and repeat-open tests.

## Atomic queue use

The [runnable example](../../crates/stucco-redb/examples/durable_work.rs) shows
startup, transactional enqueue and acknowledgement with an application record.

Use a stable tenant-scoped operation key, not a public identifier or request
filename. Enqueue only after authorization checks in that same WriteTx. Duplicate
enqueue returns false without overwriting the original record. Done and Failed
remain tombstones, preventing identifier/lease-token reuse.

A worker claims one due record inside a short transaction, then releases the
writer lock before I/O. Leases are exclusive until their deadline. Expiry makes
work available to a new attempt; retry explicitly schedules a new eligible time.
The nonzero maximum-attempt budget counts crashed attempts too. No automatic
backoff, worker fleet or external broker is required.

After idempotent work, acknowledge with the current lease and commit app metadata
in one WriteTx. A false acknowledgement means stale/expired/already-finished:
abort or skip the app effect. Propagate errors to roll the transaction back. A
lease from another queue/database cannot acknowledge a job with a coincidentally
matching key/attempt/deadline.

## Limits that applications must handle

- Lease tokens fence queue metadata, **not external I/O**. A stale remote write
  can complete after a lease expires. Use immutable attempt identities, durable
  manifests, idempotent deletion and reconciliation; do not promise exactly once.
- No renewal API or automatic runtime is supplied. Bound work, choose an adequate
  lease and handle retries. Server time is trusted input; monitor clock changes.
- Claims process at most 64 due entries. None can mean exhausted jobs were retired;
  a worker should poll again later, not assume the queue is permanently empty.
- Finished history is not automatically deleted. Retention/archival must preserve
  idempotency and monotonic identities; Failed work needs observable intervention.
- Queue payloads are Postcard persisted schemas. Keep versioned application
  payloads and avoid changing their binary layout without migration.
- For attachment cleanup, durable job delivery alone is insufficient: metadata
  transitions, byte manifests, access-time expiry, confirmed deletion and quota
  release must be coordinated by the application. Filesystem `.staging` recovery
  and S3 orphan/multipart maintenance belong to the blob backend, not this queue.

Tests cover migration history drift/future-version refusal, atomic failure/panic
rollback and concurrent startup, plus enqueue/effect rollback, retry/restart,
expired/stale/wrong-queue leases and concurrent claimers. No external storage or
production worker is started by these tests.
