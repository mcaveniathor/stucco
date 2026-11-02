use crate::{BlobError, BlobFuture, BlobKey, BlobStore, Bytes};
use fs2::FileExt;
use std::{
    collections::BTreeSet,
    fs, io,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

const STAGING: &str = ".staging";
const LOCK: &str = ".store-lock";
const DEFAULT_SCAN_LIMIT: usize = 10_000;

#[derive(Debug)]
struct Inner {
    root: PathBuf,
    // Keep the exclusive OS lock alive through every clone and blocking task.
    _lock: fs::File,
    operations: Arc<tokio::sync::Mutex<()>>,
}

/// Private blobs in a local directory owned by one process. Clones share its
/// exclusive OS lock and operation gate. Independent opens of the same root fail.
///
/// The existing root must be private (no group/other permissions on Unix).
/// Symlink components and non-regular objects are refused. The root and its
/// ancestors must remain controlled by the operator: this is not a sandbox
/// against a malicious process running as the same OS user.
///
/// Writes sync a private stage file then rename. [`recover_staging`](Self::recover_staging)
/// safely reclaims abandoned files while quiescing this backend's active I/O.
#[derive(Clone, Debug)]
pub struct FilesystemStore {
    inner: Arc<Inner>,
    scan_limit: usize,
}

impl FilesystemStore {
    /// Opens an existing private local directory with exclusive ownership.
    /// A missing root is an error. Enumeration fails after 10,000 inspected
    /// directory entries; configure a different technical budget with
    /// [`with_scan_limit`](Self::with_scan_limit), or use a narrower prefix.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, BlobError> {
        let supplied = root.as_ref();
        let meta = fs::symlink_metadata(supplied)?;
        private_directory(&meta)?;
        let root = supplied.canonicalize()?;
        let lock_path = root.join(LOCK);
        match fs::symlink_metadata(&lock_path) {
            Ok(meta) => regular_file(&meta)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let lock = options.open(lock_path)?;
        lock.try_lock_exclusive()
            .map_err(|_| BlobError::new("blob directory already owned or locking unavailable"))?;
        let staging = root.join(STAGING);
        private_dir(&staging)?;
        private_directory(&fs::symlink_metadata(&staging)?)?;
        Ok(Self {
            inner: Arc::new(Inner {
                root,
                _lock: lock,
                operations: Arc::new(tokio::sync::Mutex::new(())),
            }),
            scan_limit: DEFAULT_SCAN_LIMIT,
        })
    }

    /// Maximum directory entries inspected by each list call. Exceeding it is
    /// an error, never a silently incomplete page. Zero disallows directory scans.
    /// The budget is per handle; clones inherit it. `limit` separately bounds
    /// the number of retained result keys. Listing still rescans its prefix.
    pub fn with_scan_limit(mut self, entries: usize) -> Self {
        self.scan_limit = entries;
        self
    }

    fn path(&self, key: &BlobKey) -> Result<PathBuf, BlobError> {
        let mut path = self.inner.root.clone();
        for segment in key.segments() {
            path.push(segment);
            match fs::symlink_metadata(&path) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    return Err(BlobError::new("symlinks are not blob paths"));
                }
                Ok(meta) if !meta.is_dir() && !meta.is_file() => {
                    return Err(BlobError::new("non-regular blob path"));
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(path)
    }

    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(FilesystemStore) -> Result<T, BlobError> + Send + 'static,
    ) -> Result<T, BlobError> {
        let store = self.clone();
        let guard = store.inner.operations.clone().lock_owned().await;
        tokio::task::spawn_blocking(move || {
            let _guard = guard;
            work(store)
        })
        .await
        .map_err(BlobError::new)?
    }

    /// Deletes up to `limit` abandoned stage files, including after process
    /// restart. Holds the same gate as writes, so it cannot remove an active
    /// writer's staging file, even if that writer's async caller was cancelled.
    /// No age heuristic is used. Repeat until zero at startup before uploads.
    /// Unexpected directories/symlinks fail closed, never followed or removed.
    pub async fn recover_staging(&self, limit: usize) -> Result<usize, BlobError> {
        self.blocking(move |store| {
            let staging = store.inner.root.join(STAGING);
            private_directory(&fs::symlink_metadata(&staging)?)?;
            let mut removed = 0;
            for entry in fs::read_dir(&staging)?.take(limit) {
                let entry = entry?;
                regular_file(&fs::symlink_metadata(entry.path())?)?;
                fs::remove_file(entry.path())?;
                removed += 1;
            }
            // Successful cleanup is durable before it is acknowledged.
            sync_dir(&staging)?;
            Ok(removed)
        })
        .await
    }

    fn write(&self, key: &BlobKey, bytes: &[u8]) -> Result<(), BlobError> {
        let path = self.path(key)?;
        let parent = path.parent().expect("key has parent");
        let mut created = Vec::new();
        let mut missing = Some(parent);
        while let Some(dir) = missing.filter(|dir| !dir.exists()) {
            created.push(dir);
            missing = dir.parent();
        }
        for dir in created.iter().rev() {
            private_dir(dir)?;
            sync_dir(dir.parent().expect("under root"))?;
        }
        let staging = self.inner.root.join(STAGING);
        private_directory(&fs::symlink_metadata(&staging)?)?;
        let mut random = [0; 16];
        getrandom::fill(&mut random).map_err(|_| BlobError::new("no random source"))?;
        let name: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let staged = staging.join(name);
        let result = write_synced(&staged, bytes)
            .and_then(|()| fs::rename(&staged, &path))
            .and_then(|()| sync_dir(parent))
            .and_then(|()| sync_dir(&staging));
        if result.is_err() {
            let _ = fs::remove_file(&staged);
        }
        Ok(result?)
    }

    fn walk(
        &self,
        dir: &Path,
        prefix: &str,
        after: Option<&BlobKey>,
        limit: usize,
        visited: &mut usize,
        keys: &mut BTreeSet<BlobKey>,
    ) -> Result<(), BlobError> {
        for entry in fs::read_dir(dir)? {
            if *visited >= self.scan_limit {
                return Err(BlobError::new(
                    "blob enumeration scan budget exceeded; narrow the prefix",
                ));
            }
            *visited += 1;
            let entry = entry?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| BlobError::new("invalid blob path"))?;
            if name.starts_with('.') {
                continue;
            }
            let key = BlobKey::new(format!("{prefix}/{name}"))?;
            let meta = fs::symlink_metadata(entry.path())?;
            if meta.file_type().is_symlink() {
                return Err(BlobError::new("symlinks are not blob paths"));
            }
            if meta.is_dir() {
                self.walk(&entry.path(), key.as_str(), after, limit, visited, keys)?;
            } else {
                regular_file(&meta)?;
                if after.is_none_or(|after| &key > after) {
                    keys.insert(key);
                    if keys.len() > limit {
                        keys.pop_last();
                    }
                }
            }
        }
        Ok(())
    }
}

fn regular_file(meta: &fs::Metadata) -> Result<(), BlobError> {
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(BlobError::new("expected regular blob file"));
    }
    Ok(())
}
fn private_directory(meta: &fs::Metadata) -> Result<(), BlobError> {
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(BlobError::new("expected private blob directory"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err(BlobError::new(
                "blob directory must have no group/other permissions",
            ));
        }
    }
    Ok(())
}
fn private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    match builder.create(path) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => Err(error),
        _ => Ok(()),
    }
}
fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

impl BlobStore for FilesystemStore {
    fn put<'a>(&'a self, key: &'a BlobKey, bytes: Bytes) -> BlobFuture<'a, ()> {
        let key = key.clone();
        Box::pin(self.blocking(move |store| store.write(&key, &bytes)))
    }
    fn get_bounded<'a>(
        &'a self,
        key: &'a BlobKey,
        max_bytes: usize,
    ) -> BlobFuture<'a, Option<Bytes>> {
        let key = key.clone();
        Box::pin(self.blocking(move |store| {
            let path = store.path(&key)?;
            let mut file = match fs::File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            let meta = file.metadata()?;
            if meta.is_dir() {
                return Ok(None);
            }
            regular_file(&meta)?;
            if meta.len() > max_bytes as u64 {
                return Err(BlobError::new("blob exceeds byte limit"));
            }
            let mut bytes = Vec::new();
            let mut chunk = [0; 8192];
            loop {
                let count = file.read(&mut chunk)?;
                if count == 0 {
                    break;
                }
                crate::append_bounded(&mut bytes, &chunk[..count], max_bytes)?;
            }
            Ok(Some(bytes.into()))
        }))
    }
    fn delete<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, ()> {
        let key = key.clone();
        Box::pin(self.blocking(move |store| {
            let path = store.path(&key)?;
            if path.is_dir() {
                return Ok(());
            }
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
                _ => {}
            }
            if let Some(parent) = path.parent() {
                match sync_dir(parent) {
                    Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e.into()),
                    _ => {}
                }
            }
            Ok(())
        }))
    }
    fn list<'a>(
        &'a self,
        prefix: &'a BlobKey,
        after: Option<&'a BlobKey>,
        limit: usize,
    ) -> BlobFuture<'a, Vec<BlobKey>> {
        let (prefix, after) = (prefix.clone(), after.cloned());
        Box::pin(self.blocking(move |store| {
            if limit == 0 {
                return Ok(Vec::new());
            }
            let start = store.path(&prefix)?;
            let mut keys = BTreeSet::new();
            if start.is_dir() {
                store.walk(
                    &start,
                    prefix.as_str(),
                    after.as_ref(),
                    limit,
                    &mut 0,
                    &mut keys,
                )?;
            } else if start.is_file() && after.as_ref().is_none_or(|after| &prefix > after) {
                keys.insert(prefix);
            }
            Ok(keys.into_iter().collect())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> tempfile::TempDir {
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(std::fs::Permissions::from_mode(0o700));
        }
        builder.tempdir().unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelled_async_writer_keeps_gate_until_blocking_work_finishes() {
        let root = tempdir();
        let store = FilesystemStore::open(root.path()).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let writer = store.clone();
        let task = tokio::spawn(async move {
            writer
                .blocking(move |store| {
                    let stage = store.inner.root.join(STAGING).join("active");
                    fs::write(&stage, "in progress")?;
                    started_tx.send(()).unwrap();
                    finish_rx.recv().unwrap();
                    assert!(stage.exists(), "cleanup must not touch an active writer");
                    Ok(())
                })
                .await
        });
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        task.abort(); // spawn_blocking work is still running and holds ownership.
        let recovery_store = store.clone();
        let recovery = tokio::spawn(async move { recovery_store.recover_staging(10).await });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(!recovery.is_finished());
        assert!(root.path().join(STAGING).join("active").exists());
        finish_tx.send(()).unwrap();
        assert_eq!(recovery.await.unwrap().unwrap(), 1);
    }

    #[tokio::test]
    async fn cancellation_while_waiting_schedules_no_write() {
        let root = tempdir();
        let store = FilesystemStore::open(root.path()).unwrap();
        let guard = store.inner.operations.clone().lock_owned().await;
        let key = BlobKey::new("cancelled").unwrap();
        let mut pending = store.put(&key, Bytes::from_static(b"cancel me"));
        std::future::poll_fn(|cx| {
            assert!(pending.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        drop(pending);
        drop(guard);
        assert!(store.get_bounded(&key, 20).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn sparse_oversized_file_is_refused_without_allocating_its_size() {
        let root = tempdir();
        let store = FilesystemStore::open(root.path()).unwrap();
        fs::File::create(root.path().join("huge"))
            .unwrap()
            .set_len(1 << 40)
            .unwrap();
        assert!(
            store
                .get_bounded(&BlobKey::new("huge").unwrap(), 16)
                .await
                .is_err()
        );
    }
}
