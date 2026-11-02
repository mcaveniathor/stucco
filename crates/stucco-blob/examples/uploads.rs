//! Stores a blob, finds it again and removes it. Run with
//! `cargo run -p stucco-blob --example uploads`.
use stucco_blob::{BlobError, BlobKey, BlobStore, FilesystemStore};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), BlobError> {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    let dir = builder.tempdir()?;
    let store = FilesystemStore::open(dir.path())?;
    while store.recover_staging(100).await? != 0 {}

    // Keys come from your own ids, so a tenant's blobs share a prefix.
    let key = BlobKey::new("tenants/7/files/12/original")?;
    store.put_bounded(&key, "a receipt".into(), 1024).await?;

    let tenant = BlobKey::new("tenants/7")?;
    println!("stored: {:?}", store.list(&tenant, None, 100).await?);

    store.delete(&key).await?;
    println!("left: {:?}", store.list(&tenant, None, 100).await?);
    Ok(())
}
