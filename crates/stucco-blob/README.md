# stucco-blob

Private blob storage for stucco applications: uploads, attachments and exports.
One small interface over a directory, memory, or any S3-compatible service.

```rust,no_run
use stucco_blob::{BlobKey, BlobStore, FilesystemStore};
# async fn demo() -> Result<(), stucco_blob::BlobError> {
// Provision this existing directory private to the service user (0700 on Unix).
let store = FilesystemStore::open("/var/lib/app/blobs")?.with_scan_limit(10_000);
while store.recover_staging(100).await? != 0 {}
let key = BlobKey::new("tenants/7/files/12/original")?;
// This is an example technical byte budget, not a product/retention default.
store.put_bounded(&key, "hello".into(), 1024).await?;
let bytes = store.get_bounded(&key, 1024).await?;
store.delete(&key).await?;
# Ok(()) }
```

Keep metadata and access rules in your own records and bytes here. Keys are
validated, so build them from your own ids, not from file names.

Every backend promises the same behaviour:

- A reader sees a whole blob or none of it.
- `put` replaces, and `delete` succeeds on a missing blob, so cleanup jobs can repeat.
- `list` pages through keys in byte order, so a job can find orphans.

Pick a backend at startup with `Arc<dyn BlobStore>`.

| Backend | Use |
| --- | --- |
| `FilesystemStore` | One owner process, cloneable within it. Private existing root, exclusive OS lock, synced rename writes, backend-owned staging recovery. Listing has an explicit scan budget. |
| `S3Store` | Any S3-compatible service, with the optional `s3` feature. |
| `MemoryStore` | Tests. |

## Explicit byte budgets

Prefer `get_bounded(key, max_bytes)` for untrusted uploads. Filesystem metadata or
S3 response metadata exceeding the budget is refused before allocating that object
size; streaming/chunked collection also checks the limit as bytes arrive. S3 rejects
truncated/inconsistent declared sizes. The retained payload is bounded; transport,
stack and allocator overhead still exist. MemoryStore checks before cloning Bytes.

`put_bounded` refuses oversized buffers before any backend write, preserving an
existing object. The caller has **already allocated** its Bytes: enforce request
body limits and concurrent-upload limits before collection. The original unbounded
`get`/`put` remain available for explicitly trusted data; no hidden product quotas
or retention defaults are introduced. These are whole-buffer APIs, not a streaming
upload protocol or a total-process-memory cap.

## Filesystem ownership, staging and path safety

The root must already exist, be private (no group/other permission bits on Unix),
and remain controlled by the service operator. `open` obtains an exclusive OS
lock in `.store-lock`; a second independent open fails until **all** clones and
blocking tasks release ownership. Do not delete/replace that lock file. Share
clones instead of opening one store per request. Local filesystems only.
Use a case-sensitive filesystem and keys whose leaf never doubles as a directory
prefix. Empty parent directories are retained after deletion and count toward the
scan budget; reconcile them during operator maintenance. Memory/S3 do not have
these filesystem layout restrictions.

Operations acquire an async FIFO gate before scheduling blocking work. Cancellation
while waiting schedules no work; once scheduled, the operation retains the gate
until settlement. Cancellation is not rollback; retry/reconcile ambiguous writes.

`recover_staging(limit)` removes at most that many regular files from the backend's
private `.staging`, which ordinary BlobStore listing intentionally hides. Run it
in batches before uploads after startup; it is also safe during operation because
it takes the same operation gate as reads/writes. Recovery waits for active blocking
writers, including a writer whose async caller was cancelled. There is no guessed
age cutoff and no removal of active stage files. Unexpected symlinks/directories
fail closed and require operator repair; retry after failures.

Blob paths, root and staging reject symlinks/non-regular files; generated child
files/directories are private. Canonical root ancestors must be trusted and stable.
This is **not** a filesystem sandbox against malicious code with the same OS user,
root privileges or permission to replace ancestors. Use a dedicated OS identity
and private directory; do not put attacker-writable trees beneath it. Other-platform
ACL/ownership setup remains the operator's responsibility.

## Enumeration budgets

Listings return sorted keys strictly after a cursor and match whole path segments.
MemoryStore ranges directly over the relevant key prefix. Filesystem listing
rescans its prefix, retaining only the smallest requested keys, and inspects at
most the configured `with_scan_limit` directory entries (default technical budget
10,000). Exceeding the budget is an **error**, not a misleading truncated page.
Narrow the prefix or explicitly raise the budget for maintenance; this is not a
constant-time large-bucket crawler. Zero result limit performs no enumeration.

S3 listing includes an object named exactly the prefix (a conditional HEAD when
its key is after the cursor), followed by descendants in order. A zero result
limit makes no requests; an exact match filling the limit avoids descendant
listing. It checks strict ordering/cursor progress and segment-prefix membership,
and fails after 10,000 inspected service entries in one call. A compatible endpoint
must provide ordered listings. Application prefixes should isolate its objects;
unknown keys and late writes still require durable app reconciliation. Limits
bound retained keys/inspected entries, not the provider client's internal buffers.

There are no signed URLs: serve bytes from a handler that checks authorization,
current expiry and metadata state each time, with private/no-store cache headers.
The backend does not implement auth, retention, variant manifests, quota accounting
or database/object atomicity. A lease cannot fence an already-running remote put;
keep unique attempt keys and repeat orphan reconciliation. This crate does not
start S3 multipart uploads; if that capability is added, abandoned remote multipart
attempts need their own tracked abort/recovery, not just completed-object listing.

To test against a live service, set `STUCCO_BLOB_S3_ENDPOINT`, `_BUCKET`,
`_ACCESS_KEY_ID` and `_SECRET_ACCESS_KEY` for an empty bucket and run
`cargo test -p stucco-blob --features s3 -- --ignored`.

Minimum Rust: **1.85**. Licensed under MIT or Apache-2.0.
