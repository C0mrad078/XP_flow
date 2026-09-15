use std::time::Duration;

use async_trait::async_trait;
use opener::open;
use tokio::sync::oneshot;

use crate::domain::auth_error::AuthError;
use crate::domain::capability::default_requested_scopes;
use crate::domain::oauth::AuthSession;
use crate::domain::platform::Platform;
use crate::domain::ports::platform_auth_provider::PlatformAuthProvider;
use crate::domain::provider_identity::{ConnectedIdentity, LocalCredential};
use crate::infrastructure::auth::LoopbackListener;

use super::api_client::YouTubeApiClient;
use super::config::YouTubeAuthConfig;

/// Section 46: an authorization attempt can't hang forever waiting for a
/// browser callback that will never come.
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

/// YouTube's real desktop flow (section 5): OAuth 2.0 + PKCE (S256) +
/// system browser + a loopback callback — no Auth Broker involved, since
/// Google does not treat an installed app's client_secret as confidential
/// (see `config::YouTubeAuthConfig`'s doc comment).
pub struct YouTubeAuthProvider {
    api: YouTubeApiClient,
}

impl YouTubeAuthProvider {
    pub fn new(config: YouTubeAuthConfig) -> Self {
        Self {
            api: YouTubeApiClient::new(config),
        }
    }
}

#[async_trait]
impl PlatformAuthProvider for YouTubeAuthProvider {
    fn platform(&self) -> Platform {
        Platform::YouTube
    }

    async fn authenticate(
        &self,
        session: AuthSession,
        cancel: oneshot::Receiver<()>,
    ) -> Result<ConnectedIdentity, AuthError> {
        let listener = LoopbackListener::start("youtube").await?;
        // The session was built with a placeholder redirect_uri before the
        // listener's OS-assigned port was known — the authorize URL must
        // use the listener's *actual* redirect_uri, not the session's.
        let redirect_uri = listener.redirect_uri.clone();

        let scopes = default_requested_scopes(Platform::YouTube);
        let authorize_url = self.api.build_authorization_url(
            &redirect_uri,
            &session.state,
            &session.pkce.challenge,
            &scopes,
        );

        open(&authorize_url).map_err(|e| AuthError::TokenExchangeFailed {
            detail: format!("failed to open the system browser: {e}"),
        })?;

        let code = listener
            .wait_for_callback(&session.state, cancel, CALLBACK_TIMEOUT)
            .await?;

        let token = self
            .api
            .exchange_code(&code, &session.pkce.verifier, &redirect_uri)
            .await?;
        let identity = self
            .api
            .fetch_authenticated_channel(&token.access_token)
            .await?;

        Ok(ConnectedIdentity {
            provider_account_id: identity.provider_account_id,
            display_name: identity.display_name,
            username_or_handle: identity.handle,
            avatar_url: identity.avatar_url,
            granted_scopes: if token.granted_scopes.is_empty() {
                scopes
            } else {
                token.granted_scopes
            },
            access_expires_at: Some(token.expires_at),
            refresh_expires_at: None,
            provider_connection_id: None,
            local_credential: Some(LocalCredential {
                access_token: token.access_token,
                refresh_token: token.refresh_token,
            }),
        })
    }
}
