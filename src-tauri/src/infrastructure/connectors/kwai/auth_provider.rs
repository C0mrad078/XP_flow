use std::time::Duration;

use async_trait::async_trait;
use opener::open;
use tokio::sync::oneshot;

use crate::domain::auth_error::AuthError;
use crate::domain::oauth::AuthSession;
use crate::domain::platform::Platform;
use crate::domain::ports::platform_auth_provider::PlatformAuthProvider;
use crate::domain::provider_identity::ConnectedIdentity;
use crate::infrastructure::auth::broker_client::BrokerSessionStatus;
use crate::infrastructure::auth::BrokerClient;

/// Section 18: the desktop polls the broker for completion rather than
/// aggressively/forever — a short, bounded interval while the auth dialog
/// is actually open.
const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Section 46: give up (and surface `AuthTimeout`) if the user hasn't
/// completed the Kwai authorization in the browser within this window.
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

/// Kwai's authorization flow is entirely broker-owned (section 17/18):
/// the broker creates the session and authorize URL, Kwai's own redirect
/// lands on the broker's registered callback endpoint (not a desktop
/// loopback), and the desktop's only job is to open the browser and poll
/// for completion.
pub struct KwaiAuthProvider {
    broker: BrokerClient,
}

impl KwaiAuthProvider {
    pub fn new(broker: BrokerClient) -> Self {
        Self { broker }
    }
}

#[async_trait]
impl PlatformAuthProvider for KwaiAuthProvider {
    fn platform(&self) -> Platform {
        Platform::Kwai
    }

    async fn authenticate(
        &self,
        session: AuthSession,
        mut cancel: oneshot::Receiver<()>,
    ) -> Result<ConnectedIdentity, AuthError> {
        let start = self
            .broker
            .start_session(
                "kwai",
                &session.workspace_id.to_string(),
                &session.channel_id.to_string(),
                None,
            )
            .await?;

        open(&start.authorize_url).map_err(|e| AuthError::TokenExchangeFailed {
            detail: format!("failed to open the system browser: {e}"),
        })?;

        let deadline = tokio::time::Instant::now() + POLL_TIMEOUT;
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(AuthError::AuthTimeout);
            }

            let status = tokio::select! {
                status = self.broker.get_session_status(&start.session_id) => status?,
                _ = &mut cancel => return Err(AuthError::AuthCancelled),
            };

            match status {
                BrokerSessionStatus::Completed { connection } => {
                    return Ok(ConnectedIdentity {
                        provider_account_id: connection.provider_account_id,
                        display_name: connection.display_name,
                        username_or_handle: connection.username_or_handle,
                        avatar_url: connection.avatar_url,
                        granted_scopes: connection.granted_scopes,
                        access_expires_at: connection.access_expires_at,
                        refresh_expires_at: connection.refresh_expires_at,
                        provider_connection_id: Some(connection.connection_id),
                        local_credential: None,
                    });
                }
                BrokerSessionStatus::Failed { code, message } => {
                    return Err(match code.as_str() {
                        "AUTH_CANCELLED" => AuthError::AuthCancelled,
                        "PERMISSION_DENIED" => AuthError::PermissionDenied,
                        _ => AuthError::TokenExchangeFailed { detail: message },
                    });
                }
                BrokerSessionStatus::Expired => return Err(AuthError::AuthTimeout),
                BrokerSessionStatus::Pending => {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    let wait = POLL_INTERVAL.min(remaining);
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = &mut cancel => return Err(AuthError::AuthCancelled),
                    }
                }
            }
        }
    }
}
