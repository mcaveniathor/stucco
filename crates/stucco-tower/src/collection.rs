//! Read-side collection integration.
use http::Extensions;
use std::error::Error;
use std::fmt;
use std::future::Future;
use stucco_core::{Capabilities, CollectionPage, CollectionQuery};

/// Request metadata shared by data sources.
#[derive(Clone, Debug, Default)]
pub struct RequestContext {
    /// Sanitized request identifier supplied by the middleware.
    pub request_id: String,
    /// Application-selected locale.
    pub locale: Option<String>,
    extensions: Extensions,
}
impl RequestContext {
    /// Copies metadata without consuming the request's extensions.
    ///
    /// ```
    /// let (parts, _) = http::Request::new(()).into_parts();
    /// let cx = stucco_tower::RequestContext::from_parts(&parts);
    /// assert!(cx.request_id.is_empty());
    /// ```
    pub fn from_parts(parts: &http::request::Parts) -> Self {
        Self {
            request_id: parts
                .headers
                .get("x-request-id")
                .and_then(|s| s.to_str().ok())
                .unwrap_or("")
                .to_owned(),
            locale: None,
            extensions: parts.extensions.clone(),
        }
    }
    /// Application-defined typed metadata, such as a principal.
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions.get::<T>()
    }
}
/// A diagnostic data-source error. Do not display its text to users.
#[derive(Debug)]
pub struct SourceError(Box<dyn Error + Send + Sync>);
impl SourceError {
    /// Retains an underlying error for tracing.
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(error))
    }
}
impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Error for SourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}
/// A storage-independent collection source. Query futures must be Send.
pub trait CollectionSource: Send + Sync + 'static {
    /// Owned row returned by the source.
    type Row: Send;
    /// Supported query operations.
    fn capabilities(&self) -> Capabilities;
    /// Reads a page; implementations must not expose transaction guards.
    fn query(
        &self,
        query: &CollectionQuery,
        cx: &RequestContext,
    ) -> impl Future<Output = Result<CollectionPage<Self::Row>, SourceError>> + Send;
}
#[cfg(feature = "axum")]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for RequestContext {
    type Rejection = std::convert::Infallible;
    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_parts(parts))
    }
}
