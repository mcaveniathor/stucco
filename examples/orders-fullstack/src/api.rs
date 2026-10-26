//! Typed application operations shared by SSR, native forms and Rust/WASM.
// Leptos generates a client function with the same flat native-form fields.
#![allow(clippy::too_many_arguments)]
use crate::{
    auth::{AccessError, LoginError, SessionView},
    domain::{DeleteError, Order, OrderDraft, OrderPage, SaveError},
};
use leptos::{prelude::*, server_fn::codec::GetUrl};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EditorState {
    pub order: Order,
    pub draft: OrderDraft,
    pub error: Option<SaveError>,
    pub csrf: String,
    pub uncertain: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeleteState {
    pub order: Order,
    pub expected_revision: String,
    pub error: Option<DeleteError>,
    pub csrf: String,
}
#[cfg(feature = "ssr")]
fn csrf() -> Result<String, ServerFnError> {
    expect_context::<axum::http::request::Parts>()
        .extensions
        .get::<stucco_tower::CsrfToken>()
        .map(|token| token.as_str().to_owned())
        .ok_or_else(|| ServerFnError::new("Missing form protection"))
}
#[cfg(feature = "ssr")]
fn principal(
    user_id: i64,
    tenant_id: i64,
    generation: u64,
) -> Result<crate::auth::Principal, AccessError> {
    crate::auth::authorize(
        &expect_context::<crate::auth::AuthSession>(),
        crate::auth::PrincipalKey {
            user_id,
            tenant_id,
            generation,
        },
    )
}
#[server(input = GetUrl)]
pub async fn session() -> Result<SessionView, ServerFnError> {
    Ok(SessionView {
        principal: expect_context::<crate::auth::AuthSession>()
            .user
            .as_ref()
            .map(crate::auth::User::principal),
        csrf: csrf()?,
    })
}
#[server]
pub async fn login(
    username: String,
    password: String,
    _csrf: String,
) -> Result<Result<SessionView, LoginError>, ServerFnError> {
    let result = crate::server::sign_in(
        expect_context::<crate::auth::AuthSession>(),
        expect_context::<tower_sessions::Session>(),
        expect_context::<crate::server::AuthConfig>(),
        username,
        password,
    )
    .await;
    Ok(result.map(|(view, cookie)| {
        expect_context::<leptos_axum::ResponseOptions>()
            .append_header(axum::http::header::SET_COOKIE, cookie);
        view
    }))
}
#[server]
pub async fn logout(_csrf: String) -> Result<SessionView, ServerFnError> {
    let (view, cookie) = crate::server::sign_out(
        expect_context::<crate::auth::AuthSession>(),
        expect_context::<crate::server::AuthConfig>(),
    )
    .await
    .map_err(|_| ServerFnError::new("Sign out could not be confirmed. Please try again."))?;
    expect_context::<leptos_axum::ResponseOptions>()
        .append_header(axum::http::header::SET_COOKIE, cookie);
    Ok(view)
}
#[server(input = GetUrl)]
pub async fn list_orders(
    query: String,
    page: u32,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<OrderPage, AccessError>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    expect_context::<crate::store::OrderStore>()
        .list(&principal, query, page)
        .await
        .map(Ok)
        .map_err(|e| ServerFnError::new(e.to_string()))
}
#[server(input = GetUrl)]
pub async fn load_order(
    id: u64,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<Option<EditorState>, AccessError>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    if let Some(state) = use_context::<EditorState>()
        && state.order.id == id
    {
        return Ok(Ok(Some(state)));
    }
    let order = if id == 0 {
        Some(crate::domain::new_order())
    } else {
        expect_context::<crate::store::OrderStore>()
            .load_by_id(&principal, id)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
    };
    let csrf = csrf()?;
    Ok(Ok(order.map(|order| EditorState {
        draft: order.draft(),
        order,
        error: None,
        csrf,
        uncertain: false,
    })))
}
#[server(input = GetUrl)]
pub async fn load_delete(
    id: u64,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<Option<DeleteState>, AccessError>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    if let Some(state) = use_context::<DeleteState>()
        && state.order.id == id
    {
        return Ok(Ok(Some(state)));
    }
    let order = expect_context::<crate::store::OrderStore>()
        .load_by_id(&principal, id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    let csrf = csrf()?;
    Ok(Ok(order.map(|order| DeleteState {
        expected_revision: order.revision.to_string(),
        order,
        error: None,
        csrf,
    })))
}
#[server]
pub async fn save_order(
    id: u64,
    customer: String,
    quantity_one: String,
    quantity_two: String,
    expected_revision: String,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<Order, SaveError>, ServerFnError> {
    let draft = OrderDraft {
        customer,
        quantity_one,
        quantity_two,
        expected_revision,
    };
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(reason) => return Ok(Err(SaveError::Access { draft, reason })),
    };
    Ok(expect_context::<crate::store::OrderStore>()
        .save_by_id(&principal, id, draft)
        .await)
}
#[server]
pub async fn create_order(
    customer: String,
    quantity_one: String,
    quantity_two: String,
    expected_revision: String,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<Order, SaveError>, ServerFnError> {
    let draft = OrderDraft {
        customer,
        quantity_one,
        quantity_two,
        expected_revision,
    };
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(reason) => return Ok(Err(SaveError::Access { draft, reason })),
    };
    Ok(expect_context::<crate::store::OrderStore>()
        .create(&principal, draft)
        .await)
}
#[server]
pub async fn delete_order(
    id: u64,
    expected_revision: String,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<(), DeleteError>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(reason) => {
            return Ok(Err(DeleteError::Access {
                expected_revision,
                reason,
            }));
        }
    };
    Ok(expect_context::<crate::store::OrderStore>()
        .delete(&principal, id, expected_revision)
        .await)
}
