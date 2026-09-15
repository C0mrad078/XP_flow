use async_trait::async_trait;

use crate::domain::auth_error::AuthError;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::platform_connector::{PlatformConnector, PlatformConnectorError};
use crate::domain::provider_identity::{ConnectedIdentity, RefreshedCredentials};
use crate::domain::publication::Publication;
use crate::infrastructure::auth::BrokerClient;

/// Kwai's post-connection operations are pure broker delegation, same
/// shape as TikTok's (section 19: refresh-token rotation is handled
/// broker-side — the desktop never sees or assumes anything about token
/// lifetimes).
pub struct KwaiConnector {
    broker: BrokerClient,
}

impl KwaiConnector {
    pub fn new(broker: BrokerClient) -> Self {
        Self { broker }
    }

    fn connection_id(account: &PlatformAccount) -> Result<&str, AuthError> {
        account
            .provider_connection_id
            .as_deref()
            .ok_or(AuthError::TokenRevoked)
    }
}

#[async_trait]
impl PlatformConnector for KwaiConnector {
    fn platform(&self) -> Platform {
        Platform::Kwai
    }

    async fn validate_connection(
        &self,
        account: &PlatformAccount,
    ) -> Result<ConnectedIdentity, AuthError> {
        let refreshed = self.refresh_connection(account).await?;
        Ok(ConnectedIdentity {
            provider_account_id: account.provider_account_id.clone().unwrap_or_default(),
            display_name: account.display_name.clone(),
            username_or_handle: account.username_or_handle.clone(),
            avatar_url: account.avatar_url.clone(),
            granted_scopes: account.granted_scopes.clone(),
            access_expires_at: refreshed.access_expires_at,
            refresh_expires_at: refreshed.refresh_expires_at,
            provider_connection_id: account.provider_connection_id.clone(),
            local_credential: None,
        })
    }

    async fn refresh_connection(
        &self,
        account: &PlatformAccount,
    ) -> Result<RefreshedCredentials, AuthError> {
        let connection = self
            .broker
            .refresh_connection(Self::connection_id(account)?)
            .await?;
        Ok(RefreshedCredentials {
            access_expires_at: connection.access_expires_at,
            refresh_expires_at: connection.refresh_expires_at,
            local_credential: None,
        })
    }

    async fn disconnect(&self, account: &PlatformAccount) -> Result<(), AuthError> {
        let Ok(connection_id) = Self::connection_id(account) else {
            return Ok(());
        };
        let _ = self.broker.revoke_connection(connection_id).await;
        Ok(())
    }

    async fn get_profile(&self, account: &PlatformAccount) -> Result<ConnectedIdentity, AuthError> {
        self.validate_connection(account).await
    }

    async fn publish_video(
        &self,
        _publication: &Publication,
    ) -> Result<String, PlatformConnectorError> {
        Err(PlatformConnectorError::NotImplemented {
            platform: Platform::Kwai,
        })
    }

    async fn get_publication_status(
        &self,
        _remote_id: &str,
    ) -> Result<String, PlatformConnectorError> {
        Err(PlatformConnectorError::NotImplemented {
            platform: Platform::Kwai,
        })
    }

    async fn fetch_metrics(
        &self,
        _remote_id: &str,
    ) -> Result<serde_json::Value, PlatformConnectorError> {
        Err(PlatformConnectorError::NotImplemented {
            platform: Platform::Kwai,
        })
    }

    async fn fetch_comments(
        &self,
        _remote_id: &str,
    ) -> Result<serde_json::Value, PlatformConnectorError> {
        Err(PlatformConnectorError::NotImplemented {
            platform: Platform::Kwai,
        })
    }
}
