//! Short-lived signed redirect feedback.
use crate::{SecretKey, cookie, csrf::reject};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use http::{Request, Response, StatusCode, header::SET_COOKIE};
use http_body_util::{Either, Full};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{SystemTime, UNIX_EPOCH},
};
pub use stucco_core::{Flash, FlashLevel};
use tower::{Layer, Service};

const DOMAIN: &[u8] = b"stucco:flash:v1\0";
const MAX_MESSAGE: usize = 1024;
const LIFETIME: u64 = 300;

/// Transports a [`Flash`] response extension through a signed, five-minute cookie.
/// The next request receives it as a request extension (`RequestContext::get`)
/// and clears the cookie. Invalid/expired cookies are cleared without display.
///
/// Use [`SeeOther::flash`](crate::SeeOther::flash) or insert a `Flash` directly
/// into response extensions. Messages over 1024 UTF-8 bytes produce 500 rather
/// than silently failing to set a browser cookie. Cookies are signed, not
/// encrypted: do not store secrets. Clearing is browser-side, so deliberate
/// cookie replay remains possible until expiry; never use flash as authorization.
///
/// Apply only to routes that consume feedback, before mounting static assets.
/// The layer is opt-in and shares [`SecretKey`] with CSRF using a separate
/// signing domain. It does not require axum.
#[derive(Clone, Debug)]
pub struct FlashLayer {
    key: SecretKey,
    secure: bool,
}
impl FlashLayer {
    /// Uses a stable application key; cookies are Secure by default.
    pub fn new(key: SecretKey) -> Self {
        Self { key, secure: true }
    }
    /// Disables Secure and the `__Host-` prefix for local HTTP development only.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    /// The cookie name used by this layer.
    pub fn cookie_name(&self) -> &'static str {
        if self.secure {
            "__Host-stucco-flash"
        } else {
            "stucco-flash"
        }
    }
    fn encode(&self, flash: Flash, now: u64) -> Option<String> {
        if flash.message.len() > MAX_MESSAGE {
            return None;
        }
        let expires = now.checked_add(LIFETIME)?;
        let mut payload = expires.to_be_bytes().to_vec();
        payload.push(match flash.level {
            FlashLevel::Info => 0,
            FlashLevel::Success => 1,
            FlashLevel::Warning => 2,
            FlashLevel::Error => 3,
        });
        payload.extend_from_slice(flash.message.as_bytes());
        let tag = self.key.sign(DOMAIN, &payload);
        Some(format!("{}.{}", URL_SAFE_NO_PAD.encode(payload), tag))
    }
    fn decode(&self, value: &str, now: u64) -> Option<Flash> {
        // Keep even adversarial allocations comfortably under a browser cookie's limit.
        if value.len() > 1500 {
            return None;
        }
        let (payload, tag) = value.split_once('.')?;
        let payload = URL_SAFE_NO_PAD.decode(payload).ok()?;
        if payload.len() < 9
            || payload.len() > MAX_MESSAGE + 9
            || !self.key.verify(DOMAIN, &payload, tag)
        {
            return None;
        }
        let expires = u64::from_be_bytes(payload[..8].try_into().ok()?);
        if expires <= now || expires.saturating_sub(now) > LIFETIME {
            return None;
        }
        let level = match payload[8] {
            0 => FlashLevel::Info,
            1 => FlashLevel::Success,
            2 => FlashLevel::Warning,
            3 => FlashLevel::Error,
            _ => return None,
        };
        Some(Flash::new(
            level,
            String::from_utf8(payload[9..].to_vec()).ok()?,
        ))
    }
}
impl<S> Layer<S> for FlashLayer {
    type Service = FlashService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        FlashService {
            inner,
            config: self.clone(),
        }
    }
}

/// Service produced by [`FlashLayer`].
#[derive(Clone, Debug)]
pub struct FlashService<S> {
    inner: S,
    config: FlashLayer,
}
impl<S, B, R> Service<Request<B>> for FlashService<S>
where
    B: Send + 'static,
    S: Service<Request<B>, Response = Response<R>> + Clone + Send + 'static,
    S::Future: Send,
    R: Send + 'static,
{
    type Response = Response<Either<Full<Bytes>, R>>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let replacement = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, replacement);
        let config = self.config.clone();
        Box::pin(async move {
            let Ok(now) = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
            else {
                return Ok(reject(StatusCode::INTERNAL_SERVER_ERROR));
            };
            let raw = cookie::read(req.headers(), config.cookie_name());
            let clear = raw != Ok(None);
            let flash = raw.ok().flatten().and_then(|v| config.decode(v, now));
            req.extensions_mut().remove::<Flash>();
            if let Some(flash) = flash {
                req.extensions_mut().insert(flash);
            }
            let mut response = inner.call(req).await?.map(Either::Right);
            if let Some(flash) = response.extensions_mut().remove::<Flash>() {
                let Some(value) = config.encode(flash, now) else {
                    return Ok(reject(StatusCode::INTERNAL_SERVER_ERROR));
                };
                response.headers_mut().append(
                    SET_COOKIE,
                    cookie::header(config.cookie_name(), &value, config.secure, Some(LIFETIME)),
                );
            } else if clear {
                response.headers_mut().append(
                    SET_COOKIE,
                    cookie::header(config.cookie_name(), "", config.secure, Some(0)),
                );
            }
            // Even an empty feedback response varies by cookie; never let a cached
            // empty page hide a later message or replay someone else's feedback.
            cookie::private(response.headers_mut());
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn messages_expire_and_cannot_be_tampered_with() {
        let layer = FlashLayer::new(SecretKey::new([7; 32]));
        for level in [
            FlashLevel::Info,
            FlashLevel::Success,
            FlashLevel::Warning,
            FlashLevel::Error,
        ] {
            let message = Flash::new(level, "Saved café <b>&\n");
            let encoded = layer.encode(message.clone(), 1000).unwrap();
            assert_eq!(layer.decode(&encoded, 1000), Some(message.clone()));
            assert_eq!(layer.decode(&encoded, 1299), Some(message));
            assert!(layer.decode(&encoded, 1300).is_none());
            assert!(layer.decode(&encoded, 999).is_none());
            assert!(
                FlashLayer::new(SecretKey::new([8; 32]))
                    .decode(&encoded, 1000)
                    .is_none()
            );
            let mut tampered = encoded.into_bytes();
            tampered[0] = b'B';
            assert!(
                layer
                    .decode(std::str::from_utf8(&tampered).unwrap(), 1000)
                    .is_none()
            );
        }
        assert!(
            layer
                .encode(Flash::new(FlashLevel::Info, "x".repeat(1025)), 0)
                .is_none()
        );
        assert!(layer.decode(&"a".repeat(2000), 0).is_none());
    }
}
