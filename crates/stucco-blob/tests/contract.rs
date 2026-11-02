//! What every backend promises.
use std::sync::Arc;
use stucco_blob::{BlobKey, BlobStore, FilesystemStore, MemoryStore};

fn key(text: &str) -> BlobKey {
    BlobKey::new(text).unwrap()
}

async fn contract(store: impl BlobStore, root: &str) {
    let k = |tail: &str| key(&format!("{root}/{tail}"));
    let (a, b, c, other) = (k("t/1/a"), k("t/1/b"), k("t/2/c"), k("tx/9/d"));
    assert_eq!(store.get(&a).await.unwrap(), None);

    store.put(&a, "first".into()).await.unwrap();
    assert_eq!(store.get(&a).await.unwrap().unwrap(), "first");
    store.put(&a, "second".into()).await.unwrap();
    assert_eq!(store.get(&a).await.unwrap().unwrap(), "second");

    store.put(&b, vec![0_u8, 255, 10].into()).await.unwrap();
    assert_eq!(&store.get(&b).await.unwrap().unwrap()[..], [0, 255, 10]);
    store.put(&c, "".into()).await.unwrap();
    assert_eq!(store.get(&c).await.unwrap().unwrap(), "");
    store.put(&other, "x".into()).await.unwrap();

    let list = |prefix: BlobKey, after: Option<BlobKey>, limit| {
        let store = &store;
        async move { store.list(&prefix, after.as_ref(), limit).await.unwrap() }
    };
    assert_eq!(
        list(k("t"), None, 10).await,
        vec![a.clone(), b.clone(), c.clone()]
    );
    assert_eq!(list(k("t"), None, 2).await, vec![a.clone(), b.clone()]);
    assert_eq!(
        list(k("t"), Some(a.clone()), 1).await,
        std::slice::from_ref(&b)
    );
    assert_eq!(
        list(k("t"), Some(b.clone()), 10).await,
        std::slice::from_ref(&c)
    );
    assert_eq!(list(k("t"), Some(c.clone()), 10).await, []);
    assert_eq!(list(k("t/1"), None, 10).await, vec![a.clone(), b.clone()]);
    assert_eq!(list(k("nothing"), None, 10).await, []);

    store.delete(&a).await.unwrap();
    store.delete(&a).await.unwrap();
    store.delete(&k("never/stored")).await.unwrap();
    assert_eq!(store.get(&a).await.unwrap(), None);
    assert_eq!(store.get(&b).await.unwrap().unwrap().len(), 3);

    for leftover in [b, c, other] {
        store.delete(&leftover).await.unwrap();
    }
    assert_eq!(list(key(root), None, 10).await, []);
}

#[tokio::test]
async fn memory_keeps_the_contract() {
    contract(MemoryStore::new(), "root").await;
}

#[tokio::test]
async fn the_filesystem_keeps_the_contract() {
    let dir = tempfile::tempdir().unwrap();
    contract(FilesystemStore::open(dir.path()).unwrap(), "root").await;
}

#[tokio::test]
async fn a_trait_object_works_for_applications_that_choose_at_startup() {
    let store: Arc<dyn BlobStore> = Arc::new(MemoryStore::new());
    store.put(&key("a"), "x".into()).await.unwrap();
    assert!(store.get(&key("a")).await.unwrap().is_some());
}

#[tokio::test]
async fn the_filesystem_refuses_a_missing_root() {
    let dir = tempfile::tempdir().unwrap();
    assert!(FilesystemStore::open(dir.path().join("unmounted")).is_err());
}

#[tokio::test]
async fn the_filesystem_treats_a_directory_as_no_blob() {
    let dir = tempfile::tempdir().unwrap();
    let store = FilesystemStore::open(dir.path()).unwrap();
    store.put(&key("a/b"), "x".into()).await.unwrap();
    assert_eq!(store.get(&key("a")).await.unwrap(), None);
    store.delete(&key("a")).await.unwrap();
    assert!(store.get(&key("a/b")).await.unwrap().is_some());
}

#[tokio::test]
async fn the_filesystem_leaves_nothing_in_staging_and_hides_it_from_lists() {
    let dir = tempfile::tempdir().unwrap();
    let store = FilesystemStore::open(dir.path()).unwrap();
    store.put(&key("a/b"), "x".into()).await.unwrap();
    let staging = dir.path().join(".staging");
    assert_eq!(std::fs::read_dir(&staging).unwrap().count(), 0);
    std::fs::write(staging.join("abandoned"), "partial").unwrap();
    assert_eq!(store.list(&key("a"), None, 10).await.unwrap(), [key("a/b")]);
}

#[cfg(unix)]
#[tokio::test]
async fn filesystem_blobs_are_private_to_their_owner() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let store = FilesystemStore::open(dir.path()).unwrap();
    store.put(&key("a/b"), "x".into()).await.unwrap();
    let mode = |path: &str| {
        let meta = std::fs::metadata(dir.path().join(path)).unwrap();
        meta.permissions().mode() & 0o777
    };
    assert_eq!(mode("a"), 0o700);
    assert_eq!(mode("a/b"), 0o600);
}

#[cfg(feature = "s3")]
mod s3 {
    use super::*;
    use stucco_blob::{S3Config, S3Store};

    #[test]
    fn debug_output_hides_credentials() {
        let config = S3Config {
            bucket: "b".into(),
            region: "auto".into(),
            endpoint: None,
            access_key_id: "ACCESS-ID".into(),
            secret_access_key: "SECRET".into(),
            allow_http: false,
        };
        let shown = format!("{config:?}");
        assert!(!shown.contains("SECRET") && !shown.contains("ACCESS-ID"));
    }

    /// Runs against a real service. Point `STUCCO_BLOB_S3_ENDPOINT`, `_BUCKET`,
    /// `_ACCESS_KEY_ID` and `_SECRET_ACCESS_KEY` at an empty test bucket.
    #[tokio::test]
    #[ignore = "needs an S3-compatible service"]
    async fn a_live_service_keeps_the_contract() {
        let var = |name: &str| std::env::var(format!("STUCCO_BLOB_S3_{name}")).unwrap();
        let endpoint = var("ENDPOINT");
        let store = S3Store::new(S3Config {
            bucket: var("BUCKET"),
            region: "us-east-1".into(),
            allow_http: endpoint.starts_with("http://"),
            endpoint: Some(endpoint),
            access_key_id: var("ACCESS_KEY_ID"),
            secret_access_key: var("SECRET_ACCESS_KEY"),
        })
        .unwrap();
        contract(store, "contract").await;
    }
}
