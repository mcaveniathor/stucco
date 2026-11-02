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
#[derive(Clone, Debug)]
pub struct S3Store(Arc<AmazonS3>);

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

    fn get<'a>(&'a self, key: &'a BlobKey) -> BlobFuture<'a, Option<Bytes>> {
        Box::pin(async move {
            match self.0.get(&path(key)).await {
                Ok(found) => Ok(Some(found.bytes().await.map_err(BlobError::new)?)),
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
            let scope = path(prefix);
            let mut listed = match after {
                Some(after) => self.0.list_with_offset(Some(&scope), &path(after)),
                None => self.0.list(Some(&scope)),
            };
            let mut keys = Vec::new();
            while keys.len() < limit {
                let Some(found) = listed.try_next().await.map_err(BlobError::new)? else {
                    break;
                };
                keys.push(BlobKey::new(found.location.to_string())?);
            }
            Ok(keys)
        })
    }
}
