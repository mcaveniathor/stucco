//! Native form and download adapters for the export application service.
use crate::{
    auth::{AccessError, AuthSession, PrincipalKey, authorize},
    exports::{ExportError, Exports},
};
use axum::{
    Form, Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;

pub fn routes<S: Clone + Send + Sync + 'static>(exports: Exports) -> Router<S> {
    Router::new()
        .route("/exports", post(start))
        .route("/exports/{token}/retry", post(retry))
        .route("/exports/{token}/cancel", post(cancel))
        .route("/exports/{token}/download", get(download))
        .with_state(exports)
}

#[derive(Deserialize)]
struct Identity {
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
}
impl Identity {
    fn key(&self) -> PrincipalKey {
        PrincipalKey {
            user_id: self.expected_user_id,
            tenant_id: self.expected_tenant_id,
            generation: self.expected_generation,
        }
    }
}
#[derive(Deserialize)]
struct Start {
    token: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
}
#[derive(Deserialize)]
struct Retry {
    expected_attempt: u32,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
}
async fn start(
    State(exports): State<Exports>,
    auth: AuthSession,
    Form(form): Form<Start>,
) -> Response {
    let principal = match authorize(
        &auth,
        PrincipalKey {
            user_id: form.expected_user_id,
            tenant_id: form.expected_tenant_id,
            generation: form.expected_generation,
        },
    ) {
        Ok(principal) => principal,
        Err(error) => return access_error(error),
    };
    match exports.start(&principal, form.token).await {
        Ok(view) => Redirect::to(&format!("/exports/{}", view.token)).into_response(),
        Err(error) => export_error(error),
    }
}
async fn retry(
    State(exports): State<Exports>,
    auth: AuthSession,
    Path(token): Path<String>,
    Form(form): Form<Retry>,
) -> Response {
    let principal = match authorize(
        &auth,
        PrincipalKey {
            user_id: form.expected_user_id,
            tenant_id: form.expected_tenant_id,
            generation: form.expected_generation,
        },
    ) {
        Ok(principal) => principal,
        Err(error) => return access_error(error),
    };
    match exports
        .retry(&principal, &token, form.expected_attempt)
        .await
    {
        Ok(view) => Redirect::to(&format!("/exports/{}", view.token)).into_response(),
        Err(error) => export_error(error),
    }
}
async fn cancel(
    State(exports): State<Exports>,
    auth: AuthSession,
    Path(token): Path<String>,
    Form(form): Form<Identity>,
) -> Response {
    let principal = match authorize(&auth, form.key()) {
        Ok(principal) => principal,
        Err(error) => return access_error(error),
    };
    match exports.cancel(&principal, &token).await {
        Ok(view) => Redirect::to(&format!("/exports/{}", view.token)).into_response(),
        Err(error) => export_error(error),
    }
}
async fn download(
    State(exports): State<Exports>,
    auth: AuthSession,
    Path(token): Path<String>,
) -> Response {
    let Some(user) = auth.user else {
        return access_error(AccessError::Unauthenticated);
    };
    match exports.download(&user.principal(), &token).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_owned()),
                (
                    header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"orders-{token}.csv\""),
                ),
                (header::CACHE_CONTROL, "private, no-store".to_owned()),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => export_error(error),
    }
}
fn access_error(error: AccessError) -> Response {
    let status = match error {
        AccessError::Unauthenticated => StatusCode::UNAUTHORIZED,
        AccessError::IdentityChanged => StatusCode::FORBIDDEN,
    };
    error_page(status, &error.to_string(), true)
}
fn export_error(error: ExportError) -> Response {
    let status = match error {
        ExportError::NotFound => StatusCode::NOT_FOUND,
        ExportError::InvalidToken => StatusCode::BAD_REQUEST,
        ExportError::Capacity => StatusCode::TOO_MANY_REQUESTS,
        ExportError::NotRetryable | ExportError::AttemptsExhausted => StatusCode::CONFLICT,
        ExportError::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
    };
    error_page(status, &error.to_string(), false)
}
fn error_page(status: StatusCode, message: &str, sign_in: bool) -> Response {
    // Both messages are application-owned enum Display strings, never input.
    let link = if sign_in {
        "<a href=\"/login?return_to=%2Fexports\">Sign in to continue</a>"
    } else {
        "<a href=\"/exports\">Return to exports</a>"
    };
    (status, [(header::CACHE_CONTROL, "private, no-store")], Html(format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Export unavailable</title><link rel=\"stylesheet\" href=\"/theme.css\"></head><body><main class=\"app-shell\"><h1>Export unavailable</h1><p>{message}</p><p>{link}</p></main></body></html>"
    ))).into_response()
}
