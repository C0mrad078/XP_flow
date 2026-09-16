use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::domain::auth_error::AuthError;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::platform_connector::PlatformConnector;
use crate::domain::ports::secure_storage::SecureStorage;
use crate::domain::provider_identity::{ConnectedIdentity, LocalCredential, RefreshedCredentials};

use super::api_client::YouTubeApiClient;

/// Storage key every YouTube credential lives under in the OS keychain
/// (section 87) — never SQLite. Keyed by the `PlatformAccount` id so a
/// reconnect/disconnect can never accidentally touch another account's
/// credential.
fn credential_key(account_id: uuid::Uuid) -> String {
    format!("xpflow.youtube.credential.{account_id}")
}

#[derive(Serialize, Deserialize)]
struct StoredCredential {
    access_token: String,
    refresh_token: Option<String>,
}

pub struct YouTubeConnector {
    api: YouTubeApiClient,
    secure_storage: Arc<dyn SecureStorage>,
}

impl YouTubeConnector {
    pub fn new(api: YouTubeApiClient, secure_storage: Arc<dyn SecureStorage>) -> Self {
        Self {
            api,
            secure_storage,
        }
    }

    /// Persists a freshly-obtained credential, called by the orchestrating
    /// service right after a successful `authenticate()`/refresh — kept
    /// here (rather than in `PlatformAuthService`) since this is the one
    /// place that knows YouTube's storage key shape.
    pub async fn store_credential(
        &self,
        account_id: uuid::Uuid,
        credential: &LocalCredential,
    ) -> Result<(), AuthError> {
        let stored = StoredCredential {
            access_token: credential.access_token.clone(),
            refresh_token: credential.refresh_token.clone(),
        };
        let json = serde_json::to_string(&stored).map_err(|e| AuthError::TokenExchangeFailed {
            detail: format!("failed to serialize credential: {e}"),
        })?;
        self.secure_storage
            .set(&credential_key(account_id), &json)
            .await
            .map_err(|e| AuthError::TokenExchangeFailed {
                detail: format!("secure storage write failed: {e}"),
            })
    }

    async fn load_credential(&self, account_id: uuid::Uuid) -> Result<StoredCredential, AuthError> {
        let raw = self
            .secure_storage
            .get(&credential_key(account_id))
            .await
            .map_err(|e| AuthError::TokenExchangeFailed {
                detail: format!("secure storage read failed: {e}"),
            })?
            .ok_or(AuthError::TokenRevoked)?;
        serde_json::from_str(&raw).map_err(|e| AuthError::TokenExchangeFailed {
            detail: format!("corrupted stored credential: {e}"),
        })
    }
}

#[async_trait]
impl PlatformConnector for YouTubeConnector {
    fn platform(&self) -> Platform {
        Platform::YouTube
    }

    async fn validate_connection(
        &self,
        account: &PlatformAccount,
    ) -> Result<ConnectedIdentity, AuthError> {
        self.get_profile(account).await
    }

    async fn refresh_connection(
        &self,
        account: &PlatformAccount,
    ) -> Result<RefreshedCredentials, AuthError> {
        let stored = self.load_credential(account.id).await?;
        let refresh_token = stored.refresh_token.ok_or(AuthError::TokenRevoked)?;
        let token = self.api.refresh_token(&refresh_token).await?;

        // Google does not always re-issue a refresh_token on refresh — a
        // missing one in the response means "the old one is still valid",
        // never "there is no longer a refresh token" (section 19's
        // "never assume an old refresh token remains reusable" cuts the
        // other way too: don't *discard* a still-valid one either).
        let effective_refresh_token = token.refresh_token.or(Some(refresh_token));

        self.store_credential(
            account.id,
            &LocalCredential {
                access_token: token.access_token.clone(),
                refresh_token: effective_refresh_token.clone(),
            },
        )
        .await?;

        Ok(RefreshedCredentials {
            access_expires_at: Some(token.expires_at),
            refresh_expires_at: None,
            local_credential: Some(LocalCredential {
                access_token: token.access_token,
                refresh_token: effective_refresh_token,
            }),
        })
    }

    async fn disconnect(&self, account: &PlatformAccount) -> Result<(), AuthError> {
        if let Ok(stored) = self.load_credential(account.id).await {
            // Best-effort provider-side revocation (section 56) — a
            // failure here must never block the local disconnect, which
            // the caller performs regardless of this result.
            let _ = self.api.revoke(&stored.access_token).await;
        }
        self.secure_storage
            .delete(&credential_key(account.id))
            .await
            .map_err(|e| AuthError::TokenExchangeFailed {
                detail: format!("secure storage delete failed: {e}"),
            })
    }

    async fn get_profile(&self, account: &PlatformAccount) -> Result<ConnectedIdentity, AuthError> {
        let stored = self.load_credential(account.id).await?;
        let identity = self
            .api
            .fetch_authenticated_channel(&stored.access_token)
            .await?;
        Ok(ConnectedIdentity {
            provider_account_id: identity.provider_account_id,
            display_name: identity.display_name,
            username_or_handle: identity.handle,
            avatar_url: identity.avatar_url,
            granted_scopes: account.granted_scopes.clone(),
            access_expires_at: account.access_expires_at,
            refresh_expires_at: account.refresh_expires_at,
            provider_connection_id: None,
            local_credential: None,
        })
    }

    async fn store_local_credential(
        &self,
        account_id: uuid::Uuid,
        credential: &LocalCredential,
    ) -> Result<(), AuthError> {
        self.store_credential(account_id, credential).await
    }

    async fn acquire_access_token(&self, account: &PlatformAccount) -> Result<String, AuthError> {
        self.load_credential(account.id)
            .await
            .map(|c| c.access_token)
    }
}
