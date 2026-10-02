//! One-shot messages carried across a redirect: "Order 42 was saved."

use http::header::COOKIE;
use http::{HeaderMap, HeaderValue};

/// The cookie that carries a flash message.
pub const FLASH_COOKIE: &str = "stucco-flash";

/// The longest message kept, in bytes; longer ones are cut at a character
/// boundary.
const MAX_MESSAGE: usize = 400;

/// How a [`Flash`] message reads: map it to a notice tone when rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlashLevel {
    /// Something finished as intended.
    Success,
    /// Something the user should know.
    Info,
    /// Something may need attention.
    Warning,
    /// Something failed.
    Error,
}

impl FlashLevel {
    fn as_str(self) -> &'static str {
        match self {
            FlashLevel::Success => "success",
            FlashLevel::Info => "info",
            FlashLevel::Warning => "warning",
            FlashLevel::Error => "error",
        }
    }

    fn parse(s: &str) -> Option<FlashLevel> {
        Some(match s {
            "success" => FlashLevel::Success,
            "info" => FlashLevel::Info,
            "warning" => FlashLevel::Warning,
            "error" => FlashLevel::Error,
            _ => return None,
        })
    }
}

/// A message for the next page the browser loads, for feedback after a
/// post/redirect/get: attach it to the [`SeeOther`](crate::SeeOther) that
/// ends a successful submission, and read it with [`IncomingFlash`] on the
/// page the redirect leads to.
///
/// It travels in a short-lived cookie (`HttpOnly`, `SameSite=Lax`, one
/// minute), which the page that shows it clears. The cookie is not signed:
/// treat the message as display text. Stucco escapes it like any other
/// text, but anyone who can set cookies for your site can choose what it
/// says, so never put secrets in it, never act on it, and keep messages to
/// what happened ("Order 42 was saved."), not instructions.
///
/// ```
/// use stucco_tower::{Flash, SeeOther};
///
/// let res = SeeOther::new("/orders/42").flash(Flash::success("Order 42 was saved.")).into_response();
/// let cookie = res.headers()["set-cookie"].to_str().unwrap();
/// assert!(cookie.starts_with("stucco-flash=success."));
/// assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Lax"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flash {
    level: FlashLevel,
    message: String,
}

impl Flash {
    /// A message at `level`.
    pub fn new(level: FlashLevel, message: impl Into<String>) -> Flash {
        let mut message: String = message.into();
        if message.len() > MAX_MESSAGE {
            let mut end = MAX_MESSAGE;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
        }
        Flash { level, message }
    }

    /// Something finished as intended.
    pub fn success(message: impl Into<String>) -> Flash {
        Flash::new(FlashLevel::Success, message)
    }

    /// Something the user should know.
    pub fn info(message: impl Into<String>) -> Flash {
        Flash::new(FlashLevel::Info, message)
    }

    /// Something may need attention.
    pub fn warning(message: impl Into<String>) -> Flash {
        Flash::new(FlashLevel::Warning, message)
    }

    /// Something failed.
    pub fn error(message: impl Into<String>) -> Flash {
        Flash::new(FlashLevel::Error, message)
    }

    /// How the message reads.
    pub fn level(&self) -> FlashLevel {
        self.level
    }

    /// The message text.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The `Set-Cookie` value that carries this message.
    pub fn set_cookie(&self) -> HeaderValue {
        let message: String = form_urlencoded::byte_serialize(self.message.as_bytes()).collect();
        let value = format!(
            "{FLASH_COOKIE}={}.{message}; Path=/; Max-Age=60; HttpOnly; SameSite=Lax",
            self.level.as_str()
        );
        HeaderValue::from_str(&value).expect("an encoded cookie is a valid header value")
    }

    /// The `Set-Cookie` value that deletes the message once shown.
    pub fn clear_cookie() -> HeaderValue {
        HeaderValue::from_static("stucco-flash=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
    }

    /// Reads a message from a request's `Cookie` headers. A missing or
    /// malformed cookie gives `None`.
    pub fn from_headers(headers: &HeaderMap) -> Option<Flash> {
        let raw = headers
            .get_all(COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .flat_map(|v| v.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == FLASH_COOKIE)
            .map(|(_, value)| value)?;
        let (level, message) = raw.split_once('.')?;
        let level = FlashLevel::parse(level)?;
        let message: String = form_urlencoded::parse(format!("m={message}").as_bytes())
            .next()
            .map(|(_, m)| m.into_owned())?;
        (!message.trim().is_empty()).then(|| Flash::new(level, message))
    }
}

/// The flash message sent with this request, if any.
///
/// As an axum extractor it reads the message; returned as part of the
/// response (`(flash, document)`), it deletes the cookie, so the message
/// shows once. The page reads [`IncomingFlash::get`] while it is built,
/// then hands the flash back with the page.
///
/// ```
/// use stucco_core::el;
/// use stucco_tower::{Document, IncomingFlash, PageCx};
///
/// async fn show(flash: IncomingFlash, page: PageCx) -> (IncomingFlash, Document) {
///     let notice = flash.get().map(|f| el::p().attr("role", "status").text(f.message()));
///     let document = page.title("Order 42").body(el::main().id("main").child(notice));
///     (flash, document)
/// }
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IncomingFlash(Option<Flash>);

impl IncomingFlash {
    /// The message from a request's headers.
    pub fn from_headers(headers: &HeaderMap) -> IncomingFlash {
        IncomingFlash(Flash::from_headers(headers))
    }

    /// The message, if one was sent.
    pub fn get(&self) -> Option<&Flash> {
        self.0.as_ref()
    }
}

#[cfg(feature = "axum")]
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for IncomingFlash {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(IncomingFlash::from_headers(&parts.headers))
    }
}

#[cfg(feature = "axum")]
impl axum::response::IntoResponseParts for IncomingFlash {
    type Error = std::convert::Infallible;

    fn into_response_parts(
        self,
        mut res: axum::response::ResponseParts,
    ) -> Result<axum::response::ResponseParts, Self::Error> {
        // Only a request that carried a message has a cookie to delete.
        if self.0.is_some() {
            res.headers_mut()
                .append(http::header::SET_COOKIE, Flash::clear_cookie());
        }
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cookie_header(set_cookie: &HeaderValue) -> HeaderMap {
        let pair = set_cookie.to_str().unwrap().split(';').next().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("theme=dark; {pair}; other=1")).unwrap(),
        );
        headers
    }

    #[test]
    fn messages_round_trip_through_the_cookie() {
        for flash in [
            Flash::success("Order 42 was saved."),
            Flash::error("Couldn't save: “caf\u{e9}” & <b>; = ok"),
            Flash::warning("Ünïcödé ✓"),
        ] {
            let headers = cookie_header(&flash.set_cookie());
            assert_eq!(Flash::from_headers(&headers), Some(flash));
        }
    }

    #[test]
    fn malformed_cookies_and_long_messages_are_handled() {
        let mut headers = HeaderMap::new();
        for bad in [
            "stucco-flash=",
            "stucco-flash=nonsense",
            "stucco-flash=evil.hi",
            "stucco-flash=info.",
            "stucco-flash=info.%20",
            "other=info.hi",
        ] {
            headers.insert(COOKIE, HeaderValue::from_static(bad));
            assert_eq!(Flash::from_headers(&headers), None, "{bad}");
        }
        let long = Flash::info("é".repeat(MAX_MESSAGE));
        assert!(long.message().len() <= MAX_MESSAGE);
        assert!(long.message().chars().all(|c| c == 'é'));
        assert!(
            Flash::clear_cookie()
                .to_str()
                .unwrap()
                .contains("Max-Age=0")
        );
    }
}
