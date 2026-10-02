//! Reading a submitted HTML form into a [`FormState`].

use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use http::StatusCode;
use http::header::CONTENT_TYPE;
use stucco_core::FormState;

/// An axum extractor for an `application/x-www-form-urlencoded` POST body.
///
/// The CSRF field is left out of the state so it is never redisplayed. The
/// body is read through axum's `Bytes` extractor, so `DefaultBodyLimit` and
/// the standard layers' body limit apply. A request with another content
/// type is rejected with `415 Unsupported Media Type`.
///
/// ```
/// use stucco_core::Validator;
/// use stucco_tower::{SeeOther, Submission};
///
/// async fn create(Submission(form): Submission) -> axum::response::Response {
///     let mut v = Validator::new(form);
///     let name = v.text("name").required("Enter a name").get();
///     match v.finish(|| name) {
///         Ok(_name) => axum::response::IntoResponse::into_response(SeeOther::new("/done")),
///         Err(_state) => todo!("re-render the form with status 422"),
///     }
/// }
/// let _app: axum::Router = axum::Router::new().route("/", axum::routing::post(create));
/// ```
#[derive(Clone, Debug)]
pub struct Submission(pub FormState);

/// Why a [`Submission`] could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub enum SubmissionRejection {
    /// The request is not `application/x-www-form-urlencoded` (415).
    UnsupportedMediaType,
    /// The body could not be read; too large gives 413.
    Body(BytesRejection),
}

impl std::fmt::Display for SubmissionRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmissionRejection::UnsupportedMediaType => {
                f.write_str("expected an application/x-www-form-urlencoded body")
            }
            SubmissionRejection::Body(e) => write!(f, "could not read the form body: {e}"),
        }
    }
}

impl std::error::Error for SubmissionRejection {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SubmissionRejection::UnsupportedMediaType => None,
            SubmissionRejection::Body(e) => Some(e),
        }
    }
}

impl IntoResponse for SubmissionRejection {
    fn into_response(self) -> Response {
        match self {
            SubmissionRejection::UnsupportedMediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "Expected a form submission",
            )
                .into_response(),
            SubmissionRejection::Body(e) => e.into_response(),
        }
    }
}

impl<S: Send + Sync> FromRequest<S> for Submission {
    type Rejection = SubmissionRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let urlencoded = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .is_some_and(|mime| {
                mime.trim()
                    .eq_ignore_ascii_case("application/x-www-form-urlencoded")
            });
        if !urlencoded {
            return Err(SubmissionRejection::UnsupportedMediaType);
        }
        let body = Bytes::from_request(req, state)
            .await
            .map_err(SubmissionRejection::Body)?;
        Ok(Submission(FormState::from_urlencoded(&body)))
    }
}
