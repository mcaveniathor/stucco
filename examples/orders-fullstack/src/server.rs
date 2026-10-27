//! Axum adapters. Every transport authorizes before entering tenant services.
use crate::sessions::Sessions;
use crate::{
    api::{DeleteState, EditorState},
    auth::{
        AccessError, AuthSession, Backend, Credentials, LoginError, LoginState, PrincipalKey,
        SessionView, authorize,
    },
    domain::{DeleteError, OrderDraft, SaveError},
    shell,
    store::{OrderStore, StoreError},
};
use axum::{
    Form, Router,
    body::Body,
    extract::{FromRequest, Path, Request, State},
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use leptos::{config::LeptosOptions, prelude::*};
use serde::{Deserialize, Serialize};
use stucco_tower::{CsrfLayer, CsrfToken, SecretKey};
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, Session, SessionManagerLayer, cookie::SameSite};

#[derive(Clone)]
pub struct AuthConfig {
    pub csrf: CsrfLayer,
    pub session_seconds: i64,
}
#[derive(Clone)]
struct AppState {
    store: OrderStore,
    options: LeptosOptions,
    auth: AuthConfig,
    exports: crate::exports::Exports,
}

/// Migrate before listening. A caller owns the pool and closes it on shutdown.
pub async fn app(
    options: LeptosOptions,
    key: SecretKey,
    origin: &str,
    store: OrderStore,
    session_seconds: i64,
) -> Result<Router, StoreError> {
    Ok(
        app_with_exports(options, key, origin, store, session_seconds)
            .await?
            .0,
    )
}

/// Return the executor handle so the server can drain background work on shutdown.
pub async fn app_with_exports(
    options: LeptosOptions,
    key: SecretKey,
    origin: &str,
    store: OrderStore,
    session_seconds: i64,
) -> Result<(Router, crate::exports::Exports), StoreError> {
    validate_runtime_options(&options)?;
    if !(1..=604_800).contains(&session_seconds) {
        return Err("Session lifetime must be 1–604800 seconds".into());
    }
    let _ = any_spawner::Executor::init_tokio();
    let secure = origin.starts_with("https://");
    let csrf = CsrfLayer::new(key, origin)?
        .secure(secure)
        .multipart(true)
        .body_limit(crate::attachments::MAX_MULTIPART_BYTES);
    let auth = AuthConfig {
        csrf: csrf.clone(),
        session_seconds,
    };
    let session_store = Sessions::new(&store).await?;
    let sessions = SessionManagerLayer::new(session_store)
        .with_name(if secure {
            "__Host-orders-session"
        } else {
            "orders-session"
        })
        .with_http_only(true)
        .with_secure(secure)
        .with_same_site(SameSite::Lax)
        .with_path("/")
        .with_expiry(Expiry::OnSessionEnd)
        .with_always_save(false);
    let backend = Backend::new(store.clone()).await?;
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(backend, sessions).build();
    let css = format!(
        "{}\n{}",
        stucco_leptos::stylesheet(stucco_theme::Theme::seeded(7)),
        include_str!("style.css")
    );
    let assets = format!("{}/{}", options.site_root, options.site_pkg_dir);
    let public_assets = Router::new()
        .route(
            "/theme.css",
            get(move || {
                let css = css.clone();
                async move { ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], css) }
            }),
        )
        .nest_service("/pkg", ServeDir::new(assets))
        .layer(axum::middleware::from_fn(public_asset_cache));
    let exports = crate::exports::Exports::new(store.clone()).await?;
    let state = AppState {
        store,
        options,
        auth,
        exports: exports.clone(),
    };
    Ok((
        Router::new()
            .merge(crate::attachments::routes(state.store.clone()))
            .merge(crate::export_routes::routes(exports.clone()))
            .route("/", get(|| async { Redirect::to("/orders") }))
            .route("/login", get(index).post(login_native))
            .route("/logout", post(logout_native))
            .route("/components", get(index))
            .route("/exports", get(index))
            .route("/exports/{token}", get(index))
            .route("/orders", get(index))
            .route("/orders/new", get(index).post(create_native))
            .route("/orders/{id}", get(index))
            .route("/orders/{id}/edit", get(index).post(save_native))
            .route("/orders/{id}/delete", get(index).post(delete_native))
            .route(
                "/api/{*fn_name}",
                get(server_functions).post(server_functions),
            )
            .fallback(index)
            .layer(axum::middleware::from_fn(enforce_deadline))
            .layer(auth_layer)
            .layer(csrf)
            .layer(axum::middleware::from_fn(csrf_api_response))
            .route("/ready", get(ready))
            .merge(public_assets)
            .layer(axum::middleware::from_fn(response_headers))
            .with_state(state),
        exports,
    ))
}

/// The reference application is served at its origin's root with the checked-in
/// cargo-leptos layout. Reject overrides that would disconnect its fixed routes
/// from the generated client; changing these requires changing both builds.
pub fn validate_runtime_options(options: &LeptosOptions) -> Result<(), StoreError> {
    if options.output_name.as_ref() != "orders-fullstack" {
        return Err(
            "Use LEPTOS_OUTPUT_NAME=orders-fullstack with the matching cargo-leptos bundle".into(),
        );
    }
    if options.site_pkg_dir.as_ref() != "pkg" {
        return Err("Use LEPTOS_SITE_PKG_DIR=pkg; this application serves assets at /pkg and does not support a URL prefix".into());
    }
    if options.hash_files {
        return Err("Use LEPTOS_HASH_FILES=false; this application bundles unversioned assets with cache revalidation".into());
    }
    if options
        .server_fn_prefix
        .as_deref()
        .is_some_and(|prefix| prefix != "/api")
        || options.disable_server_fn_hash
        || options.server_fn_mod_path
    {
        return Err("Use the default server-function routes: unset SERVER_FN_PREFIX, DISABLE_SERVER_FN_HASH and SERVER_FN_MOD_PATH".into());
    }
    Ok(())
}

// Only these public routes can opt into caching. Their names are unversioned,
// so even successful responses must be revalidated after a bundle replacement.
async fn public_asset_cache(req: Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(req).await;
    if response.status().is_success() || response.status() == StatusCode::NOT_MODIFIED {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }
    response
}

// Outermost policy also covers framework rejections, auth/session failures,
// readiness, redirects and static-file errors. SSR sets its nonce policy in
// the render context; script-free native error pages use this fallback.
async fn response_headers(req: Request, next: axum::middleware::Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers
        .entry(header::CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("private, no-store"));
    headers.entry(header::CONTENT_SECURITY_POLICY).or_insert(HeaderValue::from_static(
        "default-src 'none'; style-src 'self'; object-src 'none'; base-uri 'none'; frame-src 'none'; frame-ancestors 'none'; form-action 'self'",
    ));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("same-origin"),
    );
    response
}

// tower-sessions restores record data, but not the per-request Expiry option.
// Persist the absolute deadline and reapply it only when a request changes data.
// Read-only requests must not save a session that another tab just logged out.
async fn enforce_deadline(
    session: Session,
    mut auth: AuthSession,
    mut req: Request,
    next: axum::middleware::Next,
) -> Response {
    let deadline = match session.get::<i64>("auth_deadline").await {
        Ok(value) => value,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    if (auth.user.is_some() != deadline.is_some()) || deadline.is_some_and(|value| value <= now) {
        if session.flush().await.is_err() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        auth.user = None;
        req.extensions_mut().insert(auth);
    }
    let response = next.run(req).await;
    if session.is_modified() && !session.is_empty().await {
        let deadline = match session.get::<i64>("auth_deadline").await {
            Ok(Some(value)) => Some(value),
            Ok(None) => match session.get::<i64>("recovery_deadline").await {
                Ok(value) => value,
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            },
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        };
        if let Some(deadline) = deadline {
            if deadline <= time::OffsetDateTime::now_utc().unix_timestamp() {
                if session.flush().await.is_err() {
                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
            } else if let Ok(deadline) = time::OffsetDateTime::from_unix_timestamp(deadline) {
                session.set_expiry(Some(Expiry::AtDateTime(deadline)));
            } else {
                let _ = session.flush().await;
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
        }
    }
    response
}
async fn ready(State(state): State<AppState>) -> StatusCode {
    if state.store.health().await.is_ok() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}
fn provide(state: &AppState, auth: &AuthSession, session: &Session) {
    provide_context(state.store.clone());
    provide_context(state.exports.clone());
    provide_context(state.auth.clone());
    provide_context(auth.clone());
    provide_context(session.clone());
}
async fn server_functions(
    State(state): State<AppState>,
    auth: AuthSession,
    session: Session,
    req: Request,
) -> Response {
    leptos_axum::handle_server_fns_with_context(move || provide(&state, &auth, &session), req)
        .await
        .into_response()
}
async fn index(
    State(state): State<AppState>,
    auth: AuthSession,
    session: Session,
    req: Request,
) -> Response {
    if ["/orders", "/exports", "/components"]
        .iter()
        .any(|prefix| req.uri().path().starts_with(prefix))
        && auth.user.is_none()
    {
        let destination = req
            .uri()
            .path_and_query()
            .map(|v| v.as_str())
            .unwrap_or("/orders");
        return Redirect::to(&login_url(destination)).into_response();
    }
    let mut editor = None;
    let mut deletion = None;
    if let Some(user) = &auth.user {
        match session.get::<Recovery>("native_recovery").await {
            Ok(Some(recovery)) if recovery.path == req.uri().path() => {
                if recovery.principal == user.principal().key
                    && recovery.created > time::OffsetDateTime::now_utc().unix_timestamp() - 900
                {
                    let order = if recovery.id == 0 {
                        Some(crate::domain::new_order())
                    } else {
                        match state.store.load_by_id(&user.principal(), recovery.id).await {
                            Ok(order) => order,
                            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
                        }
                    };
                    // Keep the draft if loading fails or the current account is
                    // different; consume it only once its response can be built.
                    if session.remove::<Recovery>("native_recovery").await.is_err() {
                        return StatusCode::SERVICE_UNAVAILABLE.into_response();
                    }
                    if order.is_none() {
                        return missing_recovery(recovery.input);
                    }
                    if let Some(order) = order {
                        let csrf = token(&req).unwrap_or_default();
                        match recovery.input {
                            RecoveryInput::Save(draft) => {
                                editor = Some(EditorState {
                                    order,
                                    draft,
                                    error: None,
                                    csrf,
                                    uncertain: false,
                                })
                            }
                            RecoveryInput::Delete(expected_revision) => {
                                deletion = Some(DeleteState {
                                    order,
                                    expected_revision,
                                    error: None,
                                    csrf,
                                })
                            }
                        }
                    }
                }
            }
            Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            _ => {}
        }
    }
    render(state, auth, session, req, editor, deletion, None).await
}
async fn render(
    state: AppState,
    auth: AuthSession,
    session: Session,
    req: Request,
    editor: Option<EditorState>,
    deletion: Option<DeleteState>,
    login: Option<LoginState>,
) -> Response {
    let options = state.options.clone();
    leptos_axum::render_app_async_with_context(
        move || {
            // leptos_axum provides this nonce before invoking our context
            // callback. HydrationScripts and resource scripts use the same one.
            let nonce = leptos::nonce::use_nonce().expect("Leptos provides an SSR nonce");
            expect_context::<leptos_axum::ResponseOptions>().insert_header(
                header::CONTENT_SECURITY_POLICY,
                HeaderValue::from_str(&format!(
                    "default-src 'self'; script-src 'self' 'nonce-{nonce}' 'wasm-unsafe-eval'; style-src 'self'; object-src 'none'; base-uri 'none'; frame-src 'none'; frame-ancestors 'none'; form-action 'self'",
                ))
                .expect("Leptos generates a URL-safe nonce"),
            );
            provide(&state, &auth, &session);
            if let Some(initial) = editor.clone() {
                provide_context(initial);
            }
            if let Some(initial) = deletion.clone() {
                provide_context(initial);
            }
            if let Some(initial) = login.clone() {
                provide_context(initial);
            }
        },
        move || shell(options.clone()),
    )(req)
    .await
}
fn token(req: &Request) -> Option<String> {
    req.extensions()
        .get::<CsrfToken>()
        .map(|t| t.as_str().to_owned())
}

pub async fn sign_in(
    mut auth: AuthSession,
    session: Session,
    config: AuthConfig,
    username: String,
    password: String,
) -> Result<(SessionView, HeaderValue), LoginError> {
    let user = auth
        .authenticate(Credentials { username, password })
        .await
        .map_err(|_| LoginError::Unavailable)?
        .ok_or(LoginError::InvalidCredentials)?;
    // Prepare entropy before modifying authentication. Failed session writes
    // below clear in-memory auth data so middleware cannot save a partial login.
    let (csrf, cookie) = config
        .csrf
        .rotate_token()
        .map_err(|_| LoginError::Unavailable)?;
    let result: Result<(), LoginError> = async {
        // Rotate on every login, including switching accounts in a live session.
        session
            .cycle_id()
            .await
            .map_err(|_| LoginError::Unavailable)?;
        let deadline = time::OffsetDateTime::now_utc().unix_timestamp() + config.session_seconds;
        session
            .insert("auth_deadline", deadline)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        session.set_expiry(Some(Expiry::AtDateTime(
            time::OffsetDateTime::from_unix_timestamp(deadline)
                .map_err(|_| LoginError::Unavailable)?,
        )));
        auth.login(&user)
            .await
            .map_err(|_| LoginError::Unavailable)?;
        if let Some(recovery) = session
            .get::<Recovery>("native_recovery")
            .await
            .map_err(|_| LoginError::Unavailable)?
            && recovery.principal != user.principal().key
        {
            session
                .remove::<Recovery>("native_recovery")
                .await
                .map_err(|_| LoginError::Unavailable)?;
        }
        session.save().await.map_err(|_| LoginError::Unavailable)?;
        Ok(())
    }
    .await;
    if result.is_err() {
        let _ = session.flush().await;
        return Err(LoginError::Unavailable);
    }

    Ok((
        SessionView {
            principal: Some(user.principal()),
            csrf: csrf.as_str().to_owned(),
        },
        cookie,
    ))
}
pub async fn sign_out(
    mut auth: AuthSession,
    config: AuthConfig,
) -> Result<(SessionView, HeaderValue), StoreError> {
    let (csrf, cookie) = config.csrf.rotate_token()?;
    auth.logout().await?;
    Ok((
        SessionView {
            principal: None,
            csrf: csrf.as_str().to_owned(),
        },
        cookie,
    ))
}
#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    #[serde(default)]
    return_to: String,
}
async fn login_native(
    State(state): State<AppState>,
    mut auth: AuthSession,
    session: Session,
    req: Request,
) -> Response {
    let (parts, body) = req.into_parts();
    let Form(input) =
        match Form::<LoginForm>::from_request(Request::from_parts(parts.clone(), body), &state)
            .await
        {
            Ok(v) => v,
            Err(e) => return e.into_response(),
        };
    let username = input.username.clone();
    match sign_in(
        auth.clone(),
        session.clone(),
        state.auth.clone(),
        input.username,
        input.password,
    )
    .await
    {
        Ok((_, cookie)) => {
            let mut response = Redirect::to(safe_return_to(&input.return_to)).into_response();
            response.headers_mut().append(header::SET_COOKIE, cookie);
            response
        }
        Err(error) => {
            if error == LoginError::Unavailable {
                auth.user = None;
            }
            let mut response = render(
                state,
                auth,
                session,
                Request::from_parts(parts, Body::empty()),
                None,
                None,
                Some(LoginState {
                    username: username.chars().take(128).collect(),
                    error: Some(error),
                    return_to: safe_return_to(&input.return_to).to_owned(),
                }),
            )
            .await;
            *response.status_mut() = if error == LoginError::InvalidCredentials {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            response
        }
    }
}
async fn logout_native(State(state): State<AppState>, auth: AuthSession) -> Response {
    match sign_out(auth, state.auth).await {
        Ok((_, cookie)) => {
            let mut response = Redirect::to("/login").into_response();
            response.headers_mut().append(header::SET_COOKIE, cookie);
            response
        }
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
pub fn safe_return_to(value: &str) -> &str {
    if value.len() <= 2048
        && !value.chars().any(|c| c.is_control() || c == '\\')
        && ["/orders", "/exports", "/components"].iter().any(|prefix| {
            value == *prefix
                || value
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('?'))
        })
    {
        value
    } else {
        "/orders"
    }
}
fn login_url(path: &str) -> String {
    format!(
        "/login?{}",
        form_urlencoded::Serializer::new(String::new())
            .append_pair("return_to", safe_return_to(path))
            .finish()
    )
}

#[derive(Deserialize)]
struct SaveForm {
    #[serde(flatten)]
    draft: OrderDraft,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
}
async fn create_native(
    State(state): State<AppState>,
    auth: AuthSession,
    session: Session,
    req: Request,
) -> Response {
    save(state, auth, session, req, 0).await
}
async fn save_native(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    auth: AuthSession,
    session: Session,
    req: Request,
) -> Response {
    if id == 0 {
        return StatusCode::NOT_FOUND.into_response();
    }
    save(state, auth, session, req, id).await
}
async fn save(
    state: AppState,
    auth: AuthSession,
    session: Session,
    req: Request,
    id: u64,
) -> Response {
    let csrf = token(&req).unwrap_or_default();
    let path = if id == 0 {
        "/orders/new".to_owned()
    } else {
        format!("/orders/{id}/edit")
    };
    let (parts, body) = req.into_parts();
    let Form(input) = match Form::<SaveForm>::from_request(
        Request::from_parts(parts.clone(), body),
        &state,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return e.into_response(),
    };
    let expected = PrincipalKey {
        user_id: input.expected_user_id,
        tenant_id: input.expected_tenant_id,
        generation: input.expected_generation,
    };
    let principal = match authorize(&auth, expected) {
        Ok(p) => p,
        Err(reason) => {
            return recover(
                &session,
                expected,
                path,
                id,
                RecoveryInput::Save(input.draft),
                reason,
            )
            .await;
        }
    };
    let result = if id == 0 {
        state.store.create(&principal, input.draft).await
    } else {
        state.store.save_by_id(&principal, id, input.draft).await
    };
    match result {
        Ok(saved) => Redirect::to(&format!("/orders/{}", saved.id)).into_response(),
        Err(error) => {
            let (status, draft) = match &error {
                SaveError::Validation { draft, .. } => {
                    (StatusCode::UNPROCESSABLE_ENTITY, draft.clone())
                }
                SaveError::Conflict { draft, .. } => (StatusCode::CONFLICT, draft.clone()),
                SaveError::NotFound { draft } => (StatusCode::NOT_FOUND, draft.clone()),
                SaveError::Unavailable { draft } => {
                    (StatusCode::SERVICE_UNAVAILABLE, draft.clone())
                }
                SaveError::Access { draft, .. } => (StatusCode::UNAUTHORIZED, draft.clone()),
            };
            let order = match state.store.load_by_id(&principal, id).await {
                Ok(Some(order)) => order,
                Ok(None) if id == 0 => crate::domain::new_order(),
                Ok(None) => {
                    let mut r = render(
                        state,
                        auth,
                        session,
                        Request::from_parts(parts, Body::empty()),
                        None,
                        None,
                        None,
                    )
                    .await;
                    *r.status_mut() = StatusCode::NOT_FOUND;
                    return r;
                }
                Err(_) => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        "The order could not be loaded",
                    )
                        .into_response();
                }
            };
            let initial = EditorState {
                order,
                draft,
                error: Some(error),
                csrf,
                uncertain: false,
            };
            let mut response = render(
                state,
                auth,
                session,
                Request::from_parts(parts, Body::empty()),
                Some(initial),
                None,
                None,
            )
            .await;
            *response.status_mut() = status;
            response
        }
    }
}
#[derive(Deserialize)]
struct DeleteForm {
    expected_revision: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
}
async fn delete_native(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    auth: AuthSession,
    session: Session,
    req: Request,
) -> Response {
    let csrf = token(&req).unwrap_or_default();
    let path = format!("/orders/{id}/delete");
    let (parts, body) = req.into_parts();
    let Form(input) =
        match Form::<DeleteForm>::from_request(Request::from_parts(parts.clone(), body), &state)
            .await
        {
            Ok(v) => v,
            Err(e) => return e.into_response(),
        };
    let expected = PrincipalKey {
        user_id: input.expected_user_id,
        tenant_id: input.expected_tenant_id,
        generation: input.expected_generation,
    };
    let principal = match authorize(&auth, expected) {
        Ok(p) => p,
        Err(reason) => {
            return recover(
                &session,
                expected,
                path,
                id,
                RecoveryInput::Delete(input.expected_revision),
                reason,
            )
            .await;
        }
    };
    match state
        .store
        .delete(&principal, id, input.expected_revision)
        .await
    {
        Ok(()) => Redirect::to("/orders").into_response(),
        Err(error) => {
            let (status, expected_revision) = match &error {
                DeleteError::Validation {
                    expected_revision, ..
                } => (StatusCode::UNPROCESSABLE_ENTITY, expected_revision.clone()),
                DeleteError::Conflict {
                    expected_revision, ..
                } => (StatusCode::CONFLICT, expected_revision.clone()),
                DeleteError::NotFound { expected_revision } => {
                    (StatusCode::NOT_FOUND, expected_revision.clone())
                }
                DeleteError::Unavailable { expected_revision } => {
                    (StatusCode::SERVICE_UNAVAILABLE, expected_revision.clone())
                }
                DeleteError::Access {
                    expected_revision, ..
                } => (StatusCode::UNAUTHORIZED, expected_revision.clone()),
            };
            let order = match state.store.load_by_id(&principal, id).await {
                Ok(Some(order)) => order,
                Ok(None) => {
                    let mut r = render(
                        state,
                        auth,
                        session,
                        Request::from_parts(parts, Body::empty()),
                        None,
                        None,
                        None,
                    )
                    .await;
                    *r.status_mut() = StatusCode::NOT_FOUND;
                    return r;
                }
                Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
            };
            let initial = DeleteState {
                order,
                expected_revision,
                error: Some(error),
                csrf,
            };
            let mut response = render(
                state,
                auth,
                session,
                Request::from_parts(parts, Body::empty()),
                None,
                Some(initial),
                None,
            )
            .await;
            *response.status_mut() = status;
            response
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Recovery {
    principal: PrincipalKey,
    path: String,
    id: u64,
    input: RecoveryInput,
    created: i64,
}
#[derive(Clone, Serialize, Deserialize)]
enum RecoveryInput {
    Save(OrderDraft),
    Delete(String),
}
async fn recover(
    session: &Session,
    principal: PrincipalKey,
    path: String,
    id: u64,
    input: RecoveryInput,
    reason: AccessError,
) -> Response {
    // One bounded, short-lived recovery per browser session. Only submitted text
    // is stored here; tenant records are not read until matching reauthentication.
    let size = match &input {
        RecoveryInput::Save(d) => {
            d.customer.len()
                + d.quantity_one.len()
                + d.quantity_two.len()
                + d.expected_revision.len()
        }
        RecoveryInput::Delete(r) => r.len(),
    };
    if size > 1024 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            "Submitted draft is too large to recover",
        )
            .into_response();
    }
    let recovery = Recovery {
        principal,
        path: path.clone(),
        id,
        input,
        created: time::OffsetDateTime::now_utc().unix_timestamp(),
    };
    if reason == AccessError::Unauthenticated {
        let deadline = time::OffsetDateTime::now_utc().unix_timestamp() + 900;
        if session.insert("recovery_deadline", deadline).await.is_err() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        session.set_expiry(Some(Expiry::AtDateTime(
            time::OffsetDateTime::from_unix_timestamp(deadline).expect("bounded deadline"),
        )));
    }
    if session.insert("native_recovery", recovery).await.is_err() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    let status = if reason == AccessError::Unauthenticated {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::FORBIDDEN
    };
    (status, Html(format!("<!doctype html><html lang=\"en\"><head><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Sign in to recover your draft</title><link rel=\"stylesheet\" href=\"/theme.css\"></head><body><main><h1>Sign in to recover your draft</h1><p role=\"alert\">{reason}</p><p>Your submission was not applied. Its original values and revision are held for 15 minutes. Sign in with the original account, then review and submit again.</p><a href=\"{}\">Sign in</a></main></body></html>", login_url(&path)))).into_response()
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn missing_recovery(input: RecoveryInput) -> Response {
    let fields = match input {
        RecoveryInput::Save(draft) => format!(
            "<dl><dt>Customer</dt><dd>{}</dd><dt>Notebook quantity</dt><dd>{}</dd><dt>Print quantity</dt><dd>{}</dd><dt>Original revision</dt><dd>{}</dd></dl>",
            escape(&draft.customer),
            escape(&draft.quantity_one),
            escape(&draft.quantity_two),
            escape(&draft.expected_revision)
        ),
        RecoveryInput::Delete(revision) => {
            format!("<p>Original revision: {}</p>", escape(&revision))
        }
    };
    (StatusCode::NOT_FOUND, Html(format!("<!doctype html><html lang=\"en\"><head><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Order not found</title><link rel=\"stylesheet\" href=\"/theme.css\"></head><body><main class=\"orders-shell\"><section class=\"order-card\" data-hydrated=\"false\"><h1>Order not found</h1><p>The order was deleted. Your submitted values are shown below for copying; nothing was saved.</p>{fields}<a href=\"/orders\">Back to orders</a></section></main></body></html>"))).into_response()
}

// Adapt only Stucco's own pre-handler rejection to the normal Leptos error
// transport. An arbitrary network/proxy 403 must remain an uncertain outcome.
async fn csrf_api_response(req: Request, next: axum::middleware::Next) -> Response {
    let api = req.uri().path().starts_with("/api/");
    let mut response = next.run(req).await;
    if api
        && response
            .extensions()
            .get::<stucco_tower::CsrfRejection>()
            .is_some()
    {
        use leptos::server_fn::{Encodes, error::ServerFnErrorEncoding};
        let error: ServerFnError = ServerFnError::MiddlewareError("stucco.csrf.rejected".into());
        *response.body_mut() = Body::from(
            ServerFnErrorEncoding::encode(&error).expect("serialize fixed server function error"),
        );
        response.headers_mut().remove(header::CONTENT_LENGTH);
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        );
    }
    response
}
