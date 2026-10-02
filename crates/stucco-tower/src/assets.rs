//! Serving `Bundle` files.

use std::convert::Infallible;
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::header::{
    ALLOW, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, ETAG, IF_NONE_MATCH, X_CONTENT_TYPE_OPTIONS,
};
use http::{Method, Request, Response, StatusCode};
use http_body_util::{Either, Full};
use pin_project::pin_project;
use stucco_core::Bundle;
use tower::Service;

/// Serves a [`Bundle`]'s hashed files (GET and HEAD), with conditional
/// requests and immutable caching.
#[derive(Clone, Debug)]
pub struct AssetService {
    bundle: Arc<Bundle>,
}

impl AssetService {
    /// A service for `bundle`'s files.
    pub fn new(bundle: Arc<Bundle>) -> Self {
        AssetService { bundle }
    }

    /// Sends requests outside the bundle's prefix to `inner`.
    pub fn fallback<S>(self, inner: S) -> WithFallback<S> {
        WithFallback {
            assets: self,
            inner,
        }
    }

    fn answer<B>(&self, req: &Request<B>) -> Response<Full<Bytes>> {
        let builder = Response::builder().header(X_CONTENT_TYPE_OPTIONS, "nosniff");
        if req.method() != Method::GET && req.method() != Method::HEAD {
            return builder
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .header(ALLOW, "GET, HEAD")
                .body(Full::default())
                .expect("valid response");
        }
        // The path is used as-is: never percent-decoded, so encoded names and
        // dot segments simply do not match a file.
        let Some(file) = self.bundle.get(req.uri().path()) else {
            return builder
                .status(StatusCode::NOT_FOUND)
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .body(Full::new(Bytes::from_static(b"not found")))
                .expect("valid response");
        };
        let builder = builder
            .header(ETAG, file.etag.as_str())
            .header(CACHE_CONTROL, IMMUTABLE);
        if if_none_match(req.headers().get(IF_NONE_MATCH), &file.etag) {
            return builder
                .status(StatusCode::NOT_MODIFIED)
                .body(Full::default())
                .expect("valid response");
        }
        let body = if req.method() == Method::HEAD {
            Full::default()
        } else {
            Full::new(Bytes::from_owner(file.bytes.clone()))
        };
        builder
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, file.mime)
            .header(CONTENT_LENGTH, file.bytes.len())
            .body(body)
            .expect("valid response")
    }
}

const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Whether an `If-None-Match` header matches `etag` (lists, weak tags and `*`).
fn if_none_match(header: Option<&http::HeaderValue>, etag: &str) -> bool {
    let Some(value) = header.and_then(|h| h.to_str().ok()) else {
        return false;
    };
    value.split(',').map(str::trim).any(|candidate| {
        candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == etag
    })
}

impl<B> Service<Request<B>> for AssetService {
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Infallible>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        ready(Ok(self.answer(&req)))
    }
}

/// An [`AssetService`] in front of another service; see
/// [`AssetService::fallback`].
#[derive(Clone, Debug)]
pub struct WithFallback<S> {
    assets: AssetService,
    inner: S,
}

impl<S, B, ResBody> Service<Request<B>> for WithFallback<S>
where
    S: Service<Request<B>, Response = Response<ResBody>>,
    ResBody: http_body::Body<Data = Bytes>,
{
    type Response = Response<Either<Full<Bytes>, ResBody>>;
    type Error = S::Error;
    type Future = FallbackFuture<S::Future>;

    /// Ready when the inner service is: assets are always ready, so this
    /// never reports readiness on the inner service's behalf.
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        if req
            .uri()
            .path()
            .starts_with(self.assets.bundle.url_prefix())
        {
            let res = self.assets.answer(&req);
            FallbackFuture::Asset { res: Some(res) }
        } else {
            FallbackFuture::Inner {
                future: self.inner.call(req),
            }
        }
    }
}

/// The future returned by [`WithFallback`].
#[pin_project(project = FallbackProj)]
pub enum FallbackFuture<F> {
    /// An asset response, ready immediately.
    Asset {
        /// The response, taken when polled.
        res: Option<Response<Full<Bytes>>>,
    },
    /// The inner service's response.
    Inner {
        /// The inner service's future.
        #[pin]
        future: F,
    },
}

impl<F, ResBody, E> Future for FallbackFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
{
    type Output = Result<Response<Either<Full<Bytes>, ResBody>>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            FallbackProj::Asset { res } => Poll::Ready(Ok(res
                .take()
                .expect("polled after completion")
                .map(Either::Left))),
            FallbackProj::Inner { future } => {
                future.poll(cx).map(|r| r.map(|res| res.map(Either::Right)))
            }
        }
    }
}

#[cfg(test)]
mod tests;
