//! Stores a blob, finds it again and removes it. Run with
//! `cargo run -p stucco-blob --example uploads`.
use stucco_blob::{BlobError, BlobKey, BlobStore, FilesystemStore};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), BlobError> {
    let dir = std::env::temp_dir().join("stucco-blob-example");
    std::fs::create_dir_all(&dir)?;
    let store = FilesystemStore::open(&dir)?;

    // Keys come from your own ids, so a tenant's blobs share a prefix.
    let key = BlobKey::new("tenants/7/files/12/original")?;
    store.put(&key, "a receipt".into()).await?;

    let tenant = BlobKey::new("tenants/7")?;
    println!("stored: {:?}", store.list(&tenant, None, 100).await?);

    store.delete(&key).await?;
    println!("left: {:?}", store.list(&tenant, None, 100).await?);
    Ok(())
}
