//! Typed export operations use the same service as the native form routes.
use crate::{
    auth::AccessError,
    exports::{ExportError, ExportView},
};
use leptos::{prelude::*, server_fn::codec::GetUrl};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ExportProblem {
    Access(AccessError),
    Service(ExportError),
}
impl std::fmt::Display for ExportProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Access(_) => f.write_str("Your session changed. Sign in again to continue."),
            Self::Service(error) => error.fmt(f),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportListState {
    pub jobs: Vec<ExportView>,
    pub new_token: String,
}

#[cfg(feature = "ssr")]
fn principal(
    user_id: i64,
    tenant_id: i64,
    generation: u64,
) -> Result<crate::auth::Principal, ExportProblem> {
    crate::auth::authorize(
        &expect_context::<crate::auth::AuthSession>(),
        crate::auth::PrincipalKey {
            user_id,
            tenant_id,
            generation,
        },
    )
    .map_err(ExportProblem::Access)
}
#[server(input = GetUrl)]
pub async fn list_exports(
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<ExportListState, ExportProblem>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    let jobs = match expect_context::<crate::exports::Exports>()
        .list(&principal)
        .await
    {
        Ok(jobs) => jobs,
        Err(e) => return Ok(Err(ExportProblem::Service(e))),
    };
    let new_token = crate::exports::new_token()
        .map_err(|_| ServerFnError::new("Could not prepare an export"))?;
    Ok(Ok(ExportListState { jobs, new_token }))
}
#[server(input = GetUrl)]
pub async fn export_status(
    token: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<ExportView, ExportProblem>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    Ok(expect_context::<crate::exports::Exports>()
        .status(&principal, &token)
        .await
        .map_err(ExportProblem::Service))
}
#[server]
pub async fn start_export(
    token: String,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<ExportView, ExportProblem>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    Ok(expect_context::<crate::exports::Exports>()
        .start(&principal, token)
        .await
        .map_err(ExportProblem::Service))
}
#[server]
pub async fn retry_export(
    token: String,
    expected_attempt: u32,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<ExportView, ExportProblem>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    Ok(expect_context::<crate::exports::Exports>()
        .retry(&principal, &token, expected_attempt)
        .await
        .map_err(ExportProblem::Service))
}
#[server]
pub async fn cancel_export(
    token: String,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<ExportView, ExportProblem>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    Ok(expect_context::<crate::exports::Exports>()
        .cancel(&principal, &token)
        .await
        .map_err(ExportProblem::Service))
}
