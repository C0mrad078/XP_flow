use std::time::Duration;

use async_trait::async_trait;
use opener::open;
use tokio::sync::oneshot;

use crate::domain::auth_error::AuthError;
use crate::domain::capability::default_requested_scopes;
use crate::domain::oauth::AuthSession;
use crate::domain::platform::Platform;
use crate::domain::ports::platform_auth_provider::PlatformAuthProvider;
use crate::domain::provider_identity::ConnectedIdentity;
use crate::infrastructure::auth::broker_client::ExchangeRequest;
use crate::infrastructure::auth::{BrokerClient, LoopbackListener};

use super::config::TikTokAuthConfig;

const AUTHORIZATION_ENDPOINT: &str = "https://www.tiktok.com/v2/auth/authorize/";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

/// TikTok's real desktop flow (section 10-13): the desktop drives
/// everything up through capturing the authorization code itself (its
/// `client_key` is not confidential) — only the final code-for-token
/// *exchange* needs TikTok's confidential `client_secret`, so that single
/// step is delegated to the Auth Broker. TikTok's PKCE uses its own
/// strategy type (`TikTokPkceStrategy`, section 11) even though it
/// currently computes the same RFC 7636 S256 transform as Google's.
pub struct TikTokAuthProvider {
    config: TikTokAuthConfig,
    broker: BrokerClient,
}

impl TikTokAuthProvider {
    pub fn new(config: TikTokAuthConfig, broker: BrokerClient) -> Self {
        Self { config, broker }
    }

    fn build_authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scopes: &[String],
    ) -> String {
        let mut url = url::Url::parse(AUTHORIZATION_ENDPOINT).expect("static URL is valid");
        url.query_pairs_mut()
            .append_pair("client_key", &self.config.client_key)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &scopes.join(","))
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256");
        url.to_string()
    }
}

#[async_trait]
impl PlatformAuthProvider for TikTokAuthProvider {
    fn platform(&self) -> Platform {
        Platform::TikTok
    }

    async fn authenticate(
        &self,
        session: AuthSession,
        cancel: oneshot::Receiver<()>,
    ) -> Result<ConnectedIdentity, AuthError> {
        let listener = LoopbackListener::start("tiktok").await?;
        let redirect_uri = listener.redirect_uri.clone();
        let scopes = default_requested_scopes(Platform::TikTok);

        let authorize_url = self.build_authorization_url(
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

        let connection = self
            .broker
            .exchange(
                "tiktok",
                &ExchangeRequest {
                    session_id: &session.id.to_string(),
                    code: &code,
                    code_verifier: &session.pkce.verifier,
                    redirect_uri: &redirect_uri,
                },
            )
            .await?;

        Ok(ConnectedIdentity {
            provider_account_id: connection.provider_account_id,
            display_name: connection.display_name,
            username_or_handle: connection.username_or_handle,
            avatar_url: connection.avatar_url,
            granted_scopes: if connection.granted_scopes.is_empty() {
                scopes
            } else {
                connection.granted_scopes
            },
            access_expires_at: connection.access_expires_at,
            refresh_expires_at: connection.refresh_expires_at,
            provider_connection_id: Some(connection.connection_id),
            local_credential: None,
        })
    }
}
