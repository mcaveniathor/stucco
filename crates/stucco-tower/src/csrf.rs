//! Signed double-submit CSRF protection.
use crate::{SecretKey, cookie};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use bytes::Bytes;
use http::{
    HeaderMap, Method, Request, Response, StatusCode, Uri,
    header::{CONTENT_TYPE, ORIGIN, SET_COOKIE},
};
use http_body::Body;
use http_body_util::{BodyExt, Either, Full, LengthLimitError, Limited};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use stucco_core::behavior::{CSRF_FIELD, HEADER_CSRF};
use tower::{Layer, Service};

const DOMAIN: &[u8] = b"stucco:csrf:v1\0";
type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// A form token issued by [`CsrfLayer`], available through `RequestContext::get`.
/// Pass `as_str()` to `Form::csrf`; it is never logged by Debug.
#[derive(Clone)]
pub struct CsrfToken(String);
impl CsrfToken {
    /// The value for a hidden `_csrf` field or `Stucco-Csrf` header.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl std::fmt::Debug for CsrfToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CsrfToken([redacted])")
    }
}

/// The configured public origin is not an HTTP(S) origin.
#[derive(Clone, Copy, Debug)]
pub struct InvalidOrigin;
impl std::fmt::Display for InvalidOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("expected an http(s) origin with no credentials, query or path")
    }
}
impl std::error::Error for InvalidOrigin {}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Origin {
    scheme: String,
    host: String,
    port: u16,
}
impl Origin {
    fn parse(value: &str) -> Result<Self, InvalidOrigin> {
        let uri: Uri = value.parse().map_err(|_| InvalidOrigin)?;
        let scheme = uri.scheme_str().ok_or(InvalidOrigin)?;
        if !matches!(scheme, "http" | "https")
            || uri.authority().is_none_or(|a| a.as_str().contains('@'))
            || !matches!(uri.path(), "" | "/")
            || uri.query().is_some()
            || value.contains('#')
        {
            return Err(InvalidOrigin);
        }
        // Reject malformed/non-numeric ports rather than treating them as defaults.
        let authority = uri.authority().ok_or(InvalidOrigin)?;
        let suffix = authority
            .as_str()
            .strip_prefix(authority.host())
            .ok_or(InvalidOrigin)?;
        if !suffix.is_empty() && (suffix == ":" || authority.port_u16().is_none()) {
            return Err(InvalidOrigin);
        }
        Ok(Self {
            scheme: scheme.to_owned(),
            host: uri.host().ok_or(InvalidOrigin)?.to_ascii_lowercase(),
            port: uri
                .port_u16()
                .unwrap_or(if scheme == "https" { 443 } else { 80 }),
        })
    }
}

/// Opt-in CSRF middleware for browser routes. Safe methods issue a random,
/// host-only, HttpOnly cookie and a separate HMAC form token. All other methods
/// require the cookie and a matching `_csrf` form field or `Stucco-Csrf` header.
///
/// `Origin`, when present, must match the configured public origin; forwarded
/// headers and Host are never trusted. Cross-site and same-site sibling
/// requests are rejected using `Sec-Fetch-Site`. Clients without these headers
/// still need a valid token. GET, HEAD and OPTIONS must not mutate application data.
///
/// Responses are private and non-cacheable, including when a cookie already
/// exists. Apply to page routes before mounting shared/static assets. This
/// layer is not installed automatically by `StuccoRouter`.
///
/// The cookie uses `__Host-` with Secure by default. Applications own session
/// management; clear the CSRF cookie when rotating an authentication session.
/// No JavaScript, session store or axum dependency is required.
#[derive(Clone, Debug)]
pub struct CsrfLayer {
    key: SecretKey,
    origin: Origin,
    secure: bool,
    body_limit: usize,
    #[cfg(feature = "multipart")]
    multipart: bool,
}
impl CsrfLayer {
    /// Uses a stable key and the browser-visible origin, e.g. `https://example.com`.
    pub fn new(key: SecretKey, origin: &str) -> Result<Self, InvalidOrigin> {
        Ok(Self {
            key,
            origin: Origin::parse(origin)?,
            secure: true,
            body_limit: 1024 * 1024,
            #[cfg(feature = "multipart")]
            multipart: false,
        })
    }
    /// Disables Secure and the `__Host-` prefix for local HTTP development only.
    /// Production must retain the default `true`.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    /// Maximum buffered form body (default 1 MiB). Configure this alongside the
    /// standard stack's body limit. Header-authenticated bodies pass through
    /// without buffering and remain subject to the server's body limit.
    pub fn body_limit(mut self, bytes: usize) -> Self {
        self.body_limit = bytes;
        self
    }
    /// Accept a single `_csrf` text field in bounded multipart form bodies.
    /// Requires the `multipart` feature. Disabled by default. The complete body
    /// is checked before calling the application, then replayed unchanged.
    /// Configure [`Self::body_limit`] and the application's upload limit together.
    #[cfg(feature = "multipart")]
    pub fn multipart(mut self, enabled: bool) -> Self {
        self.multipart = enabled;
        self
    }
    /// Cookie name, also useful when clearing it on login/logout.
    pub fn cookie_name(&self) -> &'static str {
        if self.secure {
            "__Host-stucco-csrf"
        } else {
            "stucco-csrf"
        }
    }
    /// Issues a fresh form token and its matching `Set-Cookie` header.
    ///
    /// Use after a CSRF-validated login/logout POST to rotate protection with
    /// the authentication session. Append the header to that response and use
    /// the returned token for subsequent forms. Do not rotate during an initial
    /// GET: the middleware may already be issuing its own cookie for that request.
    pub fn rotate_token(&self) -> Result<(CsrfToken, http::HeaderValue), crate::SecretKeyError> {
        let mut nonce = [0; 32];
        getrandom::fill(&mut nonce).map_err(|_| crate::SecretKeyError::Random)?;
        Ok((
            CsrfToken(self.key.sign(DOMAIN, &nonce)),
            cookie::header(
                self.cookie_name(),
                &URL_SAFE_NO_PAD.encode(nonce),
                self.secure,
                None,
            ),
        ))
    }
    fn trusted_source(&self, headers: &HeaderMap) -> bool {
        let origins = headers.get_all(ORIGIN);
        if origins.iter().count() > 1 {
            return false;
        }
        if let Some(value) = origins.iter().next() {
            if value
                .to_str()
                .ok()
                .and_then(|v| Origin::parse(v).ok())
                .as_ref()
                != Some(&self.origin)
            {
                return false;
            }
        }
        let sites = headers.get_all("sec-fetch-site");
        sites.iter().count() <= 1
            && sites
                .iter()
                .all(|v| matches!(v.to_str(), Ok("same-origin" | "none")))
    }
}
impl<S> Layer<S> for CsrfLayer {
    type Service = CsrfService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        CsrfService {
            inner,
            config: self.clone(),
        }
    }
}

/// Service produced by [`CsrfLayer`]. Buffered forms are replayed byte-for-byte.
#[derive(Clone, Debug)]
pub struct CsrfService<S> {
    inner: S,
    config: CsrfLayer,
}
impl<S, B, R> Service<Request<B>> for CsrfService<S>
where
    B: Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
    S: Service<Request<Either<Full<Bytes>, B>>, Response = Response<R>> + Clone + Send + 'static,
    S::Future: Send,
    R: Send + 'static,
{
    type Response = Response<Either<Full<Bytes>, R>>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }
    fn call(&mut self, req: Request<B>) -> Self::Future {
        let replacement = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, replacement);
        let config = self.config.clone();
        Box::pin(async move {
            let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
            let raw = cookie::read(req.headers(), config.cookie_name())
                .ok()
                .flatten();
            let existing = raw
                .filter(|v| v.len() == 43)
                .and_then(|v| URL_SAFE_NO_PAD.decode(v).ok())
                .filter(|v| v.len() == 32);
            let issue = existing.is_none();
            if !safe && (issue || !config.trusted_source(req.headers())) {
                return Ok(reject_csrf(StatusCode::FORBIDDEN));
            }
            let nonce = if let Some(nonce) = existing {
                nonce
            } else {
                let mut nonce = vec![0; 32];
                if getrandom::fill(&mut nonce).is_err() {
                    return Ok(reject_csrf(StatusCode::INTERNAL_SERVER_ERROR));
                }
                nonce
            };
            let (mut parts, body) = req.into_parts();
            let body = if safe {
                Either::Right(body)
            } else {
                let headers = parts.headers.get_all(HEADER_CSRF);
                if headers.iter().count() > 1 {
                    return Ok(reject_csrf(StatusCode::FORBIDDEN));
                }
                if let Some(token) = headers.iter().next() {
                    if !token
                        .to_str()
                        .is_ok_and(|t| config.key.verify(DOMAIN, &nonce, t))
                    {
                        return Ok(reject_csrf(StatusCode::FORBIDDEN));
                    }
                    Either::Right(body)
                } else {
                    let content_type = parts
                        .headers
                        .get(CONTENT_TYPE)
                        .and_then(|header| header.to_str().ok())
                        .unwrap_or("");
                    let mime = content_type.split(';').next().unwrap_or("").trim();
                    let is_form = mime.eq_ignore_ascii_case("application/x-www-form-urlencoded");
                    #[cfg(feature = "multipart")]
                    let boundary =
                        if config.multipart && mime.eq_ignore_ascii_case("multipart/form-data") {
                            Some(match multer::parse_boundary(content_type) {
                                Ok(boundary) => boundary,
                                Err(_) => return Ok(reject_csrf(StatusCode::BAD_REQUEST)),
                            })
                        } else {
                            None
                        };
                    #[cfg(not(feature = "multipart"))]
                    let is_multipart = false;
                    #[cfg(feature = "multipart")]
                    let is_multipart = boundary.is_some();
                    if (!is_form && !is_multipart)
                        || parts.headers.get_all(CONTENT_TYPE).iter().count() != 1
                    {
                        return Ok(reject_csrf(StatusCode::FORBIDDEN));
                    }
                    let bytes = match Limited::new(body, config.body_limit).collect().await {
                        Ok(collected) => collected.to_bytes(),
                        Err(error) => {
                            // A server's outer limit may also produce LengthLimitError.
                            let mut source: &(dyn std::error::Error + 'static) = error.as_ref();
                            let status = loop {
                                if source.is::<LengthLimitError>() {
                                    break StatusCode::PAYLOAD_TOO_LARGE;
                                }
                                match source.source() {
                                    Some(next) => source = next,
                                    None => break StatusCode::BAD_REQUEST,
                                }
                            };
                            return Ok(reject_csrf(status));
                        }
                    };
                    let valid = if is_form {
                        let mut tokens =
                            form_urlencoded::parse(&bytes).filter(|(name, _)| name == CSRF_FIELD);
                        tokens
                            .next()
                            .is_some_and(|(_, token)| config.key.verify(DOMAIN, &nonce, &token))
                            && tokens.next().is_none()
                    } else {
                        #[cfg(feature = "multipart")]
                        {
                            match multipart_token(
                                bytes.clone(),
                                boundary.expect("multipart boundary"),
                                &config,
                                &nonce,
                            )
                            .await
                            {
                                Ok(valid) => valid,
                                Err(status) => return Ok(reject_csrf(status)),
                            }
                        }
                        #[cfg(not(feature = "multipart"))]
                        {
                            false
                        }
                    };
                    if !valid {
                        return Ok(reject_csrf(StatusCode::FORBIDDEN));
                    }
                    Either::Left(Full::new(bytes))
                }
            };
            parts
                .extensions
                .insert(CsrfToken(config.key.sign(DOMAIN, &nonce)));
            let mut response = inner
                .call(Request::from_parts(parts, body))
                .await?
                .map(Either::Right);
            cookie::private(response.headers_mut());
            if issue {
                response.headers_mut().append(
                    SET_COOKIE,
                    cookie::header(
                        config.cookie_name(),
                        &URL_SAFE_NO_PAD.encode(nonce),
                        config.secure,
                        None,
                    ),
                );
            }
            Ok(response)
        })
    }
}

pub(crate) fn reject<R>(status: StatusCode) -> Response<Either<Full<Bytes>, R>> {
    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Either::Left(Full::new(Bytes::from_static(
            b"Request rejected",
        ))))
        .expect("valid response");
    cookie::private(response.headers_mut());
    response
}

/// Marks a CSRF rejection made before the inner application is called.
/// Transport adapters can distinguish this definite non-mutation from an
/// unrelated proxy or application response. It is an extension, not a header.
#[derive(Clone, Copy, Debug)]
pub struct CsrfRejection;
fn reject_csrf<R>(status: StatusCode) -> Response<Either<Full<Bytes>, R>> {
    let mut response = reject(status);
    if status == StatusCode::FORBIDDEN {
        response.extensions_mut().insert(CsrfRejection);
    }
    response
}

#[cfg(feature = "multipart")]
async fn multipart_token(
    bytes: Bytes,
    boundary: String,
    config: &CsrfLayer,
    nonce: &[u8],
) -> Result<bool, StatusCode> {
    let stream =
        futures_util::stream::once(async move { Ok::<_, std::convert::Infallible>(bytes) });
    let constraints = multer::Constraints::new().size_limit(
        multer::SizeLimit::new()
            .whole_stream(config.body_limit as u64)
            .for_field(CSRF_FIELD, 1024),
    );
    let mut multipart = multer::Multipart::with_constraints(stream, boundary, constraints);
    let mut seen = false;
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        if field.name() == Some(CSRF_FIELD) {
            if seen || field.file_name().is_some() {
                return Ok(false);
            }
            seen = true;
            let token = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if !config.key.verify(DOMAIN, nonce, &token) {
                return Ok(false);
            }
        } else {
            // Parse every field, including trailing data, before authorizing.
            while field
                .chunk()
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?
                .is_some()
            {}
        }
    }
    Ok(seen)
}
