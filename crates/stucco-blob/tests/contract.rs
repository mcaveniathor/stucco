//! What every backend promises.
use std::sync::Arc;
use stucco_blob::{BlobKey, BlobStore, FilesystemStore, MemoryStore};

fn tempdir() -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir().unwrap()
}

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
    // A whole-segment prefix includes an object named exactly that prefix.
    assert_eq!(list(a.clone(), None, 1).await, std::slice::from_ref(&a));
    assert_eq!(list(a.clone(), Some(a.clone()), 1).await, []);
    assert_eq!(
        list(a.clone(), Some(k("before")), 1).await,
        std::slice::from_ref(&a)
    );
    assert_eq!(list(k("missing-exact"), None, 1).await, []);

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
    let dir = tempdir();
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
    let dir = tempdir();
    assert!(FilesystemStore::open(dir.path().join("unmounted")).is_err());
}

#[tokio::test]
async fn the_filesystem_treats_a_directory_as_no_blob() {
    let dir = tempdir();
    let store = FilesystemStore::open(dir.path()).unwrap();
    store.put(&key("a/b"), "x".into()).await.unwrap();
    assert_eq!(store.get(&key("a")).await.unwrap(), None);
    store.delete(&key("a")).await.unwrap();
    assert!(store.get(&key("a/b")).await.unwrap().is_some());
}

#[tokio::test]
async fn the_filesystem_leaves_nothing_in_staging_and_hides_it_from_lists() {
    let dir = tempdir();
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
    let dir = tempdir();
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

    /// Exercise the real AmazonS3 HTTP adapter, including its appended slash,
    /// against a tiny deterministic loopback ListObjectsV2/HEAD fixture.
    #[tokio::test]
    async fn s3_adapter_lists_exact_prefix_and_descendants_with_exclusive_cursors() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut requests = Vec::new();
            while requests.len() < 7 {
                let (mut socket, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "missing adapter requests: {requests:?}"
                        );
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(error) => panic!("{error}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    assert_eq!(socket.read(&mut byte).unwrap(), 1);
                    request.push(byte[0]);
                    assert!(request.len() < 16_384);
                }
                let request = String::from_utf8(request).unwrap();
                let first = request.lines().next().unwrap();
                let words: Vec<_> = first.split_whitespace().collect();
                let target = words[1].replace("%2F", "/").replace("%2f", "/");
                let (status, body) = if words[0] == "HEAD" {
                    if target == "/fixture/a/b" {
                        ("200 OK", String::new())
                    } else {
                        assert_eq!(target, "/fixture/missing");
                        ("404 Not Found", String::new())
                    }
                } else {
                    assert_eq!(words[0], "GET");
                    // object_store's actual adapter requests the descendants.
                    let query = target.split_once('?').unwrap().1;
                    let parameter = |name: &str| {
                        query.split('&').find_map(|pair| {
                            let (key, value) = pair.split_once('=')?;
                            (key == name).then_some(value)
                        })
                    };
                    let prefix = parameter("prefix").unwrap();
                    assert!(matches!(prefix, "a/b/" | "missing/"));
                    let after = parameter("start-after").unwrap_or("");
                    let contents: String = ["a/b/c", "a/b/d"]
                        .iter()
                        .filter(|key| key.starts_with(prefix) && **key > after)
                        .map(|key| format!("<Contents><Key>{key}</Key><LastModified>2026-01-01T00:00:00Z</LastModified><ETag>test</ETag><Size>0</Size></Contents>"))
                        .collect();
                    (
                        "200 OK",
                        format!(
                            "<ListBucketResult><Name>fixture</Name><IsTruncated>false</IsTruncated>{contents}</ListBucketResult>"
                        ),
                    )
                };
                requests.push(format!("{} {}", words[0], target));
                write!(socket, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nLast-Modified: Thu, 01 Jan 2026 00:00:00 GMT\r\nETag: test\r\nContent-Type: application/xml\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        let store = S3Store::new(S3Config {
            bucket: "fixture".into(),
            region: "us-east-1".into(),
            endpoint: Some(format!("http://{address}")),
            access_key_id: "local-test-only".into(),
            secret_access_key: "local-test-only".into(),
            allow_http: true,
        })
        .unwrap();
        let prefix = key("a/b");
        assert_eq!(store.list(&prefix, None, 0).await.unwrap(), []);
        assert_eq!(
            store.list(&prefix, None, 1).await.unwrap(),
            std::slice::from_ref(&prefix)
        );
        assert_eq!(
            store.list(&prefix, None, 10).await.unwrap(),
            [prefix.clone(), key("a/b/c"), key("a/b/d")]
        );
        assert_eq!(
            store.list(&prefix, Some(&prefix), 1).await.unwrap(),
            [key("a/b/c")]
        );
        assert_eq!(
            store.list(&prefix, Some(&key("a/b/c")), 10).await.unwrap(),
            [key("a/b/d")]
        );
        assert_eq!(store.list(&key("missing"), None, 1).await.unwrap(), []);
        let requests = server.join().unwrap();
        assert_eq!(
            requests.iter().filter(|r| r.starts_with("HEAD ")).count(),
            3
        );
        assert!(requests[0].starts_with("HEAD "));
        assert!(
            requests[1].starts_with("HEAD "),
            "limit 1 must not list descendants"
        );
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
        bounded_contract(store.clone()).await;
        contract(store, "contract").await;
    }
}

async fn bounded_contract(store: impl BlobStore) {
    let blob = key("bounded/original");
    store.put_bounded(&blob, "abc".into(), 3).await.unwrap();
    assert_eq!(store.get_bounded(&blob, 3).await.unwrap().unwrap(), "abc");
    assert!(store.get_bounded(&blob, 2).await.is_err());
    assert!(
        store
            .put_bounded(&blob, "oversize".into(), 3)
            .await
            .is_err()
    );
    assert_eq!(
        store.get(&blob).await.unwrap().unwrap(),
        "abc",
        "failed write must not replace original"
    );
    let empty = key("bounded/empty");
    store.put_bounded(&empty, "".into(), 0).await.unwrap();
    assert!(
        store
            .get_bounded(&empty, 0)
            .await
            .unwrap()
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .get_bounded(&key("bounded/missing"), 0)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .list(&key("bounded"), None, 0)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        store
            .list(&key("bounded"), Some(&key("z")), 10)
            .await
            .unwrap()
            .is_empty()
    );
    store.delete(&blob).await.unwrap();
    store.delete(&empty).await.unwrap();
}

#[tokio::test]
async fn memory_enforces_byte_limits_without_partial_replacements() {
    bounded_contract(MemoryStore::new()).await;
}
#[tokio::test]
async fn filesystem_enforces_byte_limits_without_partial_replacements() {
    let dir = tempdir();
    bounded_contract(FilesystemStore::open(dir.path()).unwrap()).await;
}

#[tokio::test]
async fn staging_recovery_is_batched_and_releases_lock_after_last_clone() {
    let dir = tempdir();
    let store = FilesystemStore::open(dir.path()).unwrap();
    let clone = store.clone();
    assert!(FilesystemStore::open(dir.path()).is_err());
    for name in ["abandoned-1", "abandoned-2"] {
        std::fs::write(dir.path().join(".staging").join(name), "leftover").unwrap();
    }
    store.put(&key("a/keep"), "live".into()).await.unwrap();
    assert_eq!(store.recover_staging(0).await.unwrap(), 0);
    assert_eq!(store.recover_staging(1).await.unwrap(), 1);
    assert_eq!(store.recover_staging(10).await.unwrap(), 1);
    assert_eq!(store.recover_staging(10).await.unwrap(), 0);
    assert_eq!(store.get(&key("a/keep")).await.unwrap().unwrap(), "live");
    drop(store);
    assert!(
        FilesystemStore::open(dir.path()).is_err(),
        "clone owns the lock"
    );
    drop(clone);
    assert!(FilesystemStore::open(dir.path()).is_ok());
}

#[tokio::test]
async fn filesystem_enumeration_budget_fails_instead_of_truncating_silently() {
    let dir = tempdir();
    let store = FilesystemStore::open(dir.path())
        .unwrap()
        .with_scan_limit(2);
    for name in ["scope/c", "scope/a", "scope/b"] {
        store.put(&key(name), "x".into()).await.unwrap();
    }
    assert!(store.list(&key("scope"), None, 1).await.is_err());
    let store = store.with_scan_limit(3);
    assert_eq!(
        store.list(&key("scope"), None, 2).await.unwrap(),
        [key("scope/a"), key("scope/b")]
    );
    assert_eq!(
        store
            .list(&key("scope"), Some(&key("scope/b")), 2)
            .await
            .unwrap(),
        [key("scope/c")]
    );
}

#[cfg(unix)]
#[tokio::test]
async fn filesystem_refuses_symlink_components_and_unsafe_staging_entries() {
    use std::os::unix::fs::symlink;
    let root = tempdir();
    let outside = tempdir();
    std::fs::write(outside.path().join("secret"), "untouched").unwrap();
    let store = FilesystemStore::open(root.path()).unwrap();
    symlink(outside.path(), root.path().join("escape")).unwrap();
    let escaped = key("escape/secret");
    assert!(store.get(&escaped).await.is_err());
    assert!(store.put(&escaped, "overwrite".into()).await.is_err());
    assert!(store.delete(&escaped).await.is_err());
    assert!(store.list(&key("escape"), None, 10).await.is_err());
    symlink(
        outside.path().join("secret"),
        root.path().join(".staging/link"),
    )
    .unwrap();
    assert!(store.recover_staging(10).await.is_err());
    assert_eq!(
        std::fs::read_to_string(outside.path().join("secret")).unwrap(),
        "untouched"
    );
    assert!(root.path().join(".staging/link").symlink_metadata().is_ok());
}

#[cfg(unix)]
#[test]
fn filesystem_rejects_shared_or_symlink_roots() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempdir();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(FilesystemStore::open(root.path()).is_err());
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let links = tempdir();
    symlink(root.path(), links.path().join("root")).unwrap();
    assert!(FilesystemStore::open(links.path().join("root")).is_err());
}

// The child exits with a live backend to simulate process death without Drop.
#[test]
fn staging_crash_child() {
    if let Ok(root) = std::env::var("STUCCO_TEST_CRASH_ROOT") {
        let _store = FilesystemStore::open(&root).unwrap();
        std::fs::write(
            std::path::Path::new(&root).join(".staging/crashed"),
            "partial",
        )
        .unwrap();
        std::process::exit(0);
    }
}

#[tokio::test]
async fn crash_releases_os_lock_and_next_owner_recovers_staging() {
    let root = tempdir();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("staging_crash_child")
        .env("STUCCO_TEST_CRASH_ROOT", root.path())
        .status()
        .unwrap();
    assert!(status.success());
    let store = FilesystemStore::open(root.path()).unwrap();
    assert_eq!(store.recover_staging(10).await.unwrap(), 1);
}
