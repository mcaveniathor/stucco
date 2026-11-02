use crate::{BlobError, BlobFuture, BlobKey, BlobStore, Bytes};
use futures_util::TryStreamExt;
use object_store::{
    Error, ObjectStore, ObjectStoreExt, PutPayload,
    aws::{AmazonS3, AmazonS3Builder},
    path::Path,
};
use std::{fmt, sync::Arc};

/// Where an [`S3Store`] keeps its blobs. Debug output hides the credentials.
#[derive(Clone)]
pub struct S3Config {
    /// The bucket. Keep it private, with public access blocked.
    pub bucket: String,
    /// The region, such as `us-east-1`. Many compatible services accept `auto`.
    pub region: String,
    /// The service's URL, for anything other than AWS.
    pub endpoint: Option<String>,
    /// The access key id.
    pub access_key_id: String,
    /// The secret access key.
    pub secret_access_key: String,
    /// Allows an `http://` endpoint, for local development only.
    pub allow_http: bool,
}

impl fmt::Debug for S3Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Config")
            .field("bucket", &self.bucket)
            .field("region", &self.region)
            .field("endpoint", &self.endpoint)
            .finish_non_exhaustive()
    }
}

/// Blobs in any S3-compatible bucket. Requires the `s3` feature.
///
/// S3 requests sign with the configured key, and an endpoint uses path-style
/// addressing, which compatible services accept. Set the bucket's lifecycle
/// rules as a safety net if you like, but keep deleting from your application:
/// a lifecycle rule runs on the service's schedule, not yours.
#[derive(Clone)]
pub struct S3Store(Arc<AmazonS3>);

impl fmt::Debug for S3Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Store").finish_non_exhaustive()
    }
}

impl S3Store {
    /// Builds a client. This makes no request, so a wrong bucket or credential
    /// shows up on the first operation.
    pub fn new(config: S3Config) -> Result<Self, BlobError> {
        let mut builder = AmazonS3Builder::new()
            .with_bucket_name(config.bucket)
            .with_region(config.region)
            .with_access_key_id(config.access_key_id)
            .with_secret_access_key(config.secret_access_key)
            .with_allow_http(config.allow_http);
        if let Some(endpoint) = config.endpoint {
            builder = builder
                .with_endpoint(endpoint)
                .with_virtual_hosted_style_request(false);
        }
        Ok(Self(Arc::new(builder.build().map_err(BlobError::new)?)))
    }
}

fn path(key: &BlobKey) -> Path {
    Path::parse(key.as_str()).expect("a BlobKey is a valid object path")
}

impl BlobStore for S3Store {
    fn put<'a>(&'a self, key: &'a BlobKey, bytes: Bytes) -> BlobFuture<'a, ()> {
        Box::pin(async move {
            self.0
                .put(&path(key), PutPayload::from_bytes(bytes))
                .await
                .map_err(BlobError::new)?;
            Ok(())
        })
    }

    fn get_bounded<'a>(
        &'a self,
        key: &'a BlobKey,
        max_bytes: usize,
    ) -> BlobFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            match self.0.get(&path(key)).await {
                Ok(found) => {
                    let size = found.meta.size;
                    Ok(Some(
                        collect_bounded(found.into_stream(), size, max_bytes).await?,
                    ))
                }
                Err(Error::NotFound { .. }) => Ok(None),
                Err(error) => Err(BlobError::new(error)),
            }
        })
    }

    fn delete<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, ()> {
        Box::pin(async move {
            match self.0.delete(&path(key)).await {
                Ok(()) | Err(Error::NotFound { .. }) => Ok(()),
                Err(error) => Err(BlobError::new(error)),
            }
        })
    }

    fn list<'a>(
        &'a self,
        prefix: &'a BlobKey,
        after: Option<&'a BlobKey>,
        limit: usize,
    ) -> BlobFuture<'a, Vec<BlobKey>> {
        Box::pin(async move {
            if limit == 0 {
                return Ok(Vec::new());
            }
            let scope = path(prefix);
            // object_store lists descendants under `prefix/`, not the object
            // named exactly `prefix`. Include that object separately when the
            // exclusive cursor permits it, without reading its payload.
            let mut keys = Vec::new();
            if after.is_none_or(|after| after < prefix) {
                match self.0.head(&scope).await {
                    Ok(_) => keys.push(prefix.clone()),
                    Err(Error::NotFound { .. }) => {}
                    Err(error) => return Err(BlobError::new(error)),
                }
            }
            if keys.len() == limit {
                return Ok(keys);
            }
            let listed = match after {
                Some(after) => self.0.list_with_offset(Some(&scope), &path(after)),
                None => self.0.list(Some(&scope)),
            };
            keys.extend(collect_keys(listed, prefix, after, limit - keys.len(), 10_000).await?);
            Ok(keys)
        })
    }
}

async fn collect_keys(
    mut listed: impl futures_util::Stream<Item = Result<object_store::ObjectMeta, Error>> + Unpin,
    prefix: &BlobKey,
    after: Option<&BlobKey>,
    limit: usize,
    scan_limit: usize,
) -> Result<Vec<BlobKey>, BlobError> {
    let mut keys = Vec::new();
    let mut inspected = 0;
    let mut previous = after.cloned();
    while keys.len() < limit {
        let Some(found) = listed.try_next().await.map_err(BlobError::new)? else {
            break;
        };
        inspected += 1;
        if inspected > scan_limit {
            return Err(BlobError::new("S3 enumeration scan budget exceeded"));
        }
        let key = BlobKey::new(found.location.to_string())?;
        if previous.as_ref().is_some_and(|previous| &key <= previous) {
            return Err(BlobError::new(
                "S3 listing is not strictly ordered after cursor",
            ));
        }
        previous = Some(key.clone());
        if key.has_prefix(prefix) {
            keys.push(key);
        }
    }
    Ok(keys)
}

async fn collect_bounded(
    mut stream: impl futures_util::Stream<Item = Result<Bytes, Error>> + Unpin,
    declared: u64,
    max_bytes: usize,
) -> Result<Bytes, BlobError> {
    if declared > max_bytes as u64 {
        return Err(BlobError::new("blob exceeds byte limit"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.try_next().await.map_err(BlobError::new)? {
        crate::append_bounded(&mut bytes, &chunk, max_bytes)?;
    }
    if bytes.len() as u64 != declared {
        return Err(BlobError::new("S3 object length disagrees with metadata"));
    }
    Ok(bytes.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;

    #[test]
    fn store_debug_never_formats_credentials() {
        let store = S3Store::new(S3Config {
            bucket: "private".into(),
            region: "us-east-1".into(),
            access_key_id: "test-access-key".into(),
            secret_access_key: "test-secret-key".into(),
            endpoint: None,
            allow_http: false,
        })
        .unwrap();
        assert_eq!(format!("{store:?}"), "S3Store { .. }");
    }

    #[tokio::test]
    async fn declared_oversize_is_refused_before_polling_body() {
        let body = stream::poll_fn(|_| -> std::task::Poll<Option<Result<Bytes, Error>>> {
            panic!("oversize body must not be polled");
        });
        assert!(collect_bounded(body, 100, 10).await.is_err());
    }

    #[tokio::test]
    async fn streamed_size_limit_and_length_are_enforced() {
        let chunks = || {
            stream::iter([
                Ok(Bytes::from_static(b"abc")),
                Ok(Bytes::from_static(b"de")),
            ])
        };
        assert_eq!(collect_bounded(chunks(), 5, 5).await.unwrap(), "abcde");
        assert!(collect_bounded(chunks(), 3, 4).await.is_err());
        assert!(collect_bounded(chunks(), 6, 10).await.is_err());
    }
    fn listing(
        names: &[&str],
    ) -> impl futures_util::Stream<Item = Result<object_store::ObjectMeta, Error>> + Unpin {
        stream::iter(
            names
                .iter()
                .map(|name| {
                    Ok(object_store::ObjectMeta {
                        location: Path::from(*name),
                        last_modified: Default::default(),
                        size: 0,
                        e_tag: None,
                        version: None,
                    })
                })
                .collect::<Vec<_>>(),
        )
    }

    #[tokio::test]
    async fn listing_enforces_prefix_cursor_order_and_scan_budget() {
        let prefix = BlobKey::new("a/b").unwrap();
        let names = ["a/b", "a/b-other", "a/b/x", "a/bc"];
        let keys = collect_keys(listing(&names), &prefix, None, 10, 10)
            .await
            .unwrap();
        assert_eq!(keys, [prefix.clone(), BlobKey::new("a/b/x").unwrap()]);
        assert!(
            collect_keys(listing(&names), &prefix, None, 10, 2)
                .await
                .is_err()
        );
        assert!(
            collect_keys(listing(&["a/b/y", "a/b/x"]), &prefix, None, 10, 10)
                .await
                .is_err()
        );
        assert!(
            collect_keys(listing(&["a/b"]), &prefix, Some(&prefix), 10, 10)
                .await
                .is_err()
        );
        assert!(
            collect_keys(listing(&names), &prefix, None, 0, 0)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
