//! Typed reads and removal for the attachment panel; multipart uploads use the
//! same native endpoint from the browser's Rust FormData client.
use crate::{
    attachments::{Attachment, AttachmentError},
    auth::AccessError,
};
use leptos::{prelude::*, server_fn::codec::GetUrl};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum AttachmentFailure {
    Access(AccessError),
    Storage(AttachmentError),
}

#[cfg(feature = "ssr")]
fn principal(
    user_id: i64,
    tenant_id: i64,
    generation: u64,
) -> Result<crate::auth::Principal, AttachmentFailure> {
    crate::auth::authorize(
        &expect_context::<crate::auth::AuthSession>(),
        crate::auth::PrincipalKey {
            user_id,
            tenant_id,
            generation,
        },
    )
    .map_err(AttachmentFailure::Access)
}
#[cfg(feature = "ssr")]
fn failure(error: AttachmentError) -> AttachmentFailure {
    match error {
        AttachmentError::IdentityChanged => AttachmentFailure::Access(AccessError::IdentityChanged),
        other => AttachmentFailure::Storage(other),
    }
}

#[server(input = GetUrl)]
pub async fn load_attachments(
    order_id: u64,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<Vec<Attachment>, AttachmentFailure>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(principal) => principal,
        Err(error) => return Ok(Err(error)),
    };
    Ok(crate::attachments::list(
        &expect_context::<crate::store::OrderStore>(),
        &principal,
        order_id,
    )
    .await
    .map_err(failure))
}

#[server]
pub async fn remove_attachment(
    order_id: u64,
    attachment_id: u64,
    _csrf: String,
    expected_user_id: i64,
    expected_tenant_id: i64,
    expected_generation: u64,
) -> Result<Result<(), AttachmentFailure>, ServerFnError> {
    let principal = match principal(expected_user_id, expected_tenant_id, expected_generation) {
        Ok(principal) => principal,
        Err(error) => return Ok(Err(error)),
    };
    Ok(crate::attachments::remove(
        &expect_context::<crate::store::OrderStore>(),
        &principal,
        order_id,
        attachment_id,
    )
    .await
    .map_err(failure))
}
