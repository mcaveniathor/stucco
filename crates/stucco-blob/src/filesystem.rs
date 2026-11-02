use crate::{BlobError, BlobFuture, BlobKey, BlobStore, Bytes};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

// Keys cannot start a segment with a dot, so this never collides with a blob.
const STAGING: &str = ".staging";

/// Blobs as files under one directory, private to the owning user on Unix.
///
/// A write goes to a staging file, is synced, and is renamed into place, so a
/// crash leaves the old blob or the new one and at worst an unused staging file.
/// [`list`](BlobStore::list) walks the whole directory, which suits a periodic
/// job, not a request.
#[derive(Clone, Debug)]
pub struct FilesystemStore {
    root: Arc<PathBuf>,
}

impl FilesystemStore {
    /// Uses the existing directory `root`, which must be on a local filesystem.
    /// A missing directory is an error, so a mistyped path or an unmounted disk
    /// cannot quietly start a new, empty store.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, BlobError> {
        let root = root.as_ref().canonicalize()?;
        if !root.is_dir() {
            return Err(BlobError::new("blob root is not a directory"));
        }
        private_dir(&root.join(STAGING))?;
        Ok(Self {
            root: Arc::new(root),
        })
    }

    fn path(&self, key: &BlobKey) -> PathBuf {
        key.segments()
            .fold(self.root.as_ref().clone(), |path, segment| {
                path.join(segment)
            })
    }

    async fn blocking<T: Send + 'static>(
        &self,
        work: impl FnOnce(FilesystemStore) -> Result<T, BlobError> + Send + 'static,
    ) -> Result<T, BlobError> {
        let store = self.clone();
        tokio::task::spawn_blocking(move || work(store))
            .await
            .map_err(BlobError::new)?
    }

    fn write(&self, key: &BlobKey, bytes: &[u8]) -> Result<(), BlobError> {
        let path = self.path(key);
        let parent = path.parent().expect("a key has a parent under the root");
        let mut created = Vec::new();
        let mut missing = Some(parent);
        while let Some(dir) = missing.filter(|dir| !dir.exists()) {
            created.push(dir);
            missing = dir.parent();
        }
        for dir in created.iter().rev() {
            private_dir(dir)?;
            sync_dir(dir.parent().expect("under the root"))?;
        }
        let staged = self.staging_path()?;
        let result = write_synced(&staged, bytes)
            .and_then(|()| fs::rename(&staged, &path))
            .and_then(|()| sync_dir(parent));
        if result.is_err() {
            let _ = fs::remove_file(&staged);
        }
        Ok(result?)
    }

    fn staging_path(&self) -> Result<PathBuf, BlobError> {
        let mut name = [0; 16];
        getrandom::fill(&mut name).map_err(|_| BlobError::new("no random source"))?;
        let name: String = name.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(self.root.join(STAGING).join(name))
    }

    fn walk(&self, dir: &Path, prefix: &str, keys: &mut Vec<BlobKey>) -> Result<(), BlobError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with('.') {
                continue;
            }
            let key = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            if entry.file_type()?.is_dir() {
                self.walk(&entry.path(), &key, keys)?;
            } else if let Ok(key) = BlobKey::new(key) {
                keys.push(key);
            }
        }
        Ok(())
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

fn private_dir(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    match builder.create(path) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => Err(error),
        _ => Ok(()),
    }
}

fn sync_dir(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(path)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn missing_is_fine(result: io::Result<()>) -> Result<(), BlobError> {
    match result {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

impl BlobStore for FilesystemStore {
    fn put<'a>(&'a self, key: &'a BlobKey, bytes: Bytes) -> BlobFuture<'a, ()> {
        let key = key.clone();
        Box::pin(self.blocking(move |store| store.write(&key, &bytes)))
    }

    fn get<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, Option<Bytes>> {
        let key = key.clone();
        Box::pin(
            self.blocking(move |store| match fs::read(store.path(&key)) {
                Ok(bytes) => Ok(Some(bytes.into())),
                // A key that is a directory here holds other blobs but is not one.
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::IsADirectory
                    ) =>
                {
                    Ok(None)
                }
                Err(error) => Err(error.into()),
            }),
        )
    }

    fn delete<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, ()> {
        let key = key.clone();
        Box::pin(self.blocking(move |store| {
            let path = store.path(&key);
            if path.is_dir() {
                return Ok(());
            }
            missing_is_fine(fs::remove_file(&path))?;
            if let Some(parent) = path.parent() {
                sync_dir(parent).or_else(|error| match error.kind() {
                    io::ErrorKind::NotFound => Ok(()),
                    _ => Err(error),
                })?;
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
            let start = store.path(&prefix);
            let mut keys = Vec::new();
            if start.is_dir() {
                store.walk(&start, prefix.as_str(), &mut keys)?;
            } else if start.is_file() {
                keys.push(prefix);
            }
            keys.sort();
            keys.retain(|key| after.as_ref().is_none_or(|after| key > after));
            keys.truncate(limit);
            Ok(keys)
        }))
    }
}
