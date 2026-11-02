# stucco-blob

Private blob storage for stucco applications: uploads, attachments and exports.
One small interface over a directory, memory, or any S3-compatible service.

```rust,no_run
use stucco_blob::{BlobKey, BlobStore, FilesystemStore};
# async fn demo() -> Result<(), stucco_blob::BlobError> {
let store = FilesystemStore::open("/var/lib/app/blobs")?;
let key = BlobKey::new("tenants/7/files/12/original")?;
store.put(&key, "hello".into()).await?;
let bytes = store.get(&key).await?;
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
| `FilesystemStore` | One server. Files are private to their owner, and writes are synced and renamed into place. The directory must already exist. `list` walks all of it. |
| `S3Store` | Any S3-compatible service, with the optional `s3` feature. |
| `MemoryStore` | Tests. |

Blobs are held in memory while they move, so limit upload sizes first. There are no
signed URLs: serve blobs from a handler that checks authorization each time, because
a signed URL keeps working after the file is deleted.

To test against a live service, set `STUCCO_BLOB_S3_ENDPOINT`, `_BUCKET`,
`_ACCESS_KEY_ID` and `_SECRET_ACCESS_KEY` for an empty bucket and run
`cargo test -p stucco-blob --features s3 -- --ignored`.

Minimum Rust: **1.85**. Licensed under MIT or Apache-2.0.
