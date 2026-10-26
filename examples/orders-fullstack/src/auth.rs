//! Session identity shared by the Rust client and server. Client identity fields
//! are stale-tab assertions only; authorization always uses the server session.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrincipalKey {
    pub user_id: i64,
    pub tenant_id: i64,
    /// Changes on restore so old browser drafts cannot target reused IDs.
    pub generation: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Principal {
    pub key: PrincipalKey,
    pub display_name: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionView {
    pub principal: Option<Principal>,
    pub csrf: String,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum AccessError {
    Unauthenticated,
    IdentityChanged,
}
impl std::fmt::Display for AccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unauthenticated => "Your session expired. Sign in again to continue.",
            Self::IdentityChanged => "The signed-in account changed. Sign in with the original account to recover this draft.",
        })
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LoginError {
    InvalidCredentials,
    Unavailable,
}
impl std::fmt::Display for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCredentials => "The username or password is incorrect.",
            Self::Unavailable => "Sign in is temporarily unavailable. Please try again.",
        })
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LoginState {
    pub return_to: String,
    pub username: String,
    pub error: Option<LoginError>,
}

#[cfg(feature = "ssr")]
mod backend {
    use super::*;
    use crate::store::{OrderStore, StoreError, UserRecord};
    use axum_login::{AuthUser, AuthnBackend, UserId};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    #[derive(Clone)]
    pub struct User {
        record: UserRecord,
        auth_hash: Vec<u8>,
        generation: u64,
    }
    impl std::fmt::Debug for User {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("User")
                .field("id", &self.record.id)
                .finish_non_exhaustive()
        }
    }
    impl User {
        fn from_record(record: UserRecord, generation: u64) -> Self {
            let auth_hash = format!(
                "{}:{}:{}:{}",
                generation, record.tenant_id, record.session_epoch, record.password_hash
            )
            .into_bytes();
            Self {
                record,
                auth_hash,
                generation,
            }
        }
        pub fn principal(&self) -> Principal {
            Principal {
                key: PrincipalKey {
                    user_id: self.record.id,
                    tenant_id: self.record.tenant_id,
                    generation: self.generation,
                },
                display_name: self.record.display_name.clone(),
            }
        }
    }
    impl AuthUser for User {
        type Id = i64;
        fn id(&self) -> i64 {
            self.record.id
        }
        fn session_auth_hash(&self) -> &[u8] {
            &self.auth_hash
        }
    }
    // Credentials deliberately do not implement Debug or Serialize.
    pub struct Credentials {
        pub username: String,
        pub password: String,
    }
    #[derive(Debug)]
    pub struct BackendError;
    impl std::fmt::Display for BackendError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Authentication unavailable")
        }
    }
    impl std::error::Error for BackendError {}
    #[derive(Clone)]
    pub struct Backend {
        store: OrderStore,
        hashing: Arc<Semaphore>,
        dummy_hash: Arc<String>,
        generation: u64,
    }
    impl Backend {
        pub async fn new(store: OrderStore) -> Result<Self, BackendError> {
            let dummy_hash = tokio::task::spawn_blocking(|| {
                password_auth::generate_hash("unusable dummy credential")
            })
            .await
            .map_err(|_| BackendError)?;
            let generation = store.generation().await.map_err(|_| BackendError)?;
            Ok(Self {
                store,
                generation,
                hashing: Arc::new(Semaphore::new(4)),
                dummy_hash: Arc::new(dummy_hash),
            })
        }
    }
    #[async_trait::async_trait]
    impl AuthnBackend for Backend {
        type User = User;
        type Credentials = Credentials;
        type Error = BackendError;
        async fn authenticate(&self, creds: Credentials) -> Result<Option<User>, BackendError> {
            if creds.username.len() > 128 || creds.password.len() > 1024 {
                return Ok(None);
            }
            let permit = self
                .hashing
                .clone()
                .try_acquire_owned()
                .map_err(|_| BackendError)?;
            let user = self
                .store
                .find_user_by_username(&creds.username)
                .await
                .map_err(|_| BackendError)?
                .filter(|u| u.enabled);
            let hash = user
                .as_ref()
                .map(|u| u.password_hash.clone())
                .unwrap_or_else(|| self.dummy_hash.as_ref().clone());
            let valid = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                password_auth::verify_password(creds.password, &hash).is_ok()
            })
            .await
            .map_err(|_| BackendError)?;
            Ok(if valid {
                user.map(|u| User::from_record(u, self.generation))
            } else {
                None
            })
        }
        async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<User>, BackendError> {
            self.store
                .find_user_by_id(*user_id)
                .await
                .map(|u| {
                    u.filter(|u| u.enabled)
                        .map(|u| User::from_record(u, self.generation))
                })
                .map_err(|_| BackendError)
        }
    }
    pub type AuthSession = axum_login::AuthSession<Backend>;
    pub fn authorize(auth: &AuthSession, expected: PrincipalKey) -> Result<Principal, AccessError> {
        let principal = auth
            .user
            .as_ref()
            .ok_or(AccessError::Unauthenticated)?
            .principal();
        if principal.key != expected {
            return Err(AccessError::IdentityChanged);
        }
        Ok(principal)
    }
    /// Provisioning is an explicit operator action, never an HTTP signup path.
    pub async fn add_user(
        store: &OrderStore,
        tenant: &str,
        username: &str,
        display_name: &str,
        password: String,
    ) -> Result<Principal, StoreError> {
        if tenant.trim().is_empty()
            || tenant.len() > 128
            || username.trim().is_empty()
            || username.len() > 128
            || display_name.trim().is_empty()
            || display_name.len() > 128
        {
            return Err("Tenant, username and display name must contain 1–128 bytes".into());
        }
        if password.len() < 12 || password.len() > 1024 {
            return Err("Password must contain 12–1024 bytes".into());
        }
        let hash =
            tokio::task::spawn_blocking(move || password_auth::generate_hash(password)).await?;
        store
            .insert_user(tenant, username, display_name, &hash)
            .await
    }
    pub async fn principal_for_username(
        store: &OrderStore,
        username: &str,
    ) -> Result<Principal, StoreError> {
        let user = store
            .find_user_by_username(username)
            .await?
            .filter(|u| u.enabled)
            .ok_or("Account not found")?;
        Ok(User::from_record(user, store.generation().await?).principal())
    }
}
#[cfg(feature = "ssr")]
pub use backend::*;
