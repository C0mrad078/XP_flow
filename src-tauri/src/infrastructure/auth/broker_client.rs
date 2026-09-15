use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::auth_error::AuthError;

use super::broker_config::AuthBrokerConfig;
use super::http_client::{build_auth_http_client, BROKER_POLL_TIMEOUT};

#[derive(Debug, Clone, Serialize)]
pub struct StartSessionRequest<'a> {
    pub workspace_id: String,
    pub channel_id: String,
    pub redirect_uri: Option<&'a str>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrokerSessionStart {
    pub session_id: String,
    pub authorize_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExchangeRequest<'a> {
    pub session_id: &'a str,
    pub workspace_id: &'a str,
    pub code: &'a str,
    pub code_verifier: &'a str,
    pub redirect_uri: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrokerConnection {
    pub connection_id: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub username_or_handle: Option<String>,
    pub avatar_url: Option<String>,
    pub granted_scopes: Vec<String>,
    pub access_expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BrokerSessionStatus {
    Pending,
    Completed { connection: BrokerConnection },
    Failed { code: String, message: String },
    Expired,
}

#[derive(Debug, Deserialize)]
struct BrokerErrorBody {
    code: String,
    message: String,
}

/// Desktop-side HTTP client for the Auth Broker (section 20/24). Every
/// route it calls is hardcoded here — this client never accepts an
/// arbitrary URL from a caller, matching the broker's own no-generic-proxy
/// rule from the other direction (section 25).
#[derive(Clone)]
pub struct BrokerClient {
    http: reqwest::Client,
    base_url: String,
}

impl BrokerClient {
    pub fn new(config: &AuthBrokerConfig) -> Self {
        Self {
            http: build_auth_http_client(),
            base_url: config.base_url().to_string(),
        }
    }

    pub async fn start_session(
        &self,
        platform: &str,
        workspace_id: &str,
        channel_id: &str,
        redirect_uri: Option<&str>,
    ) -> Result<BrokerSessionStart, AuthError> {
        let url = format!("{}/v1/auth/{platform}/start", self.base_url);
        let body = StartSessionRequest {
            workspace_id: workspace_id.to_string(),
            channel_id: channel_id.to_string(),
            redirect_uri,
        };
        self.post_json(&url, &body).await
    }

    pub async fn exchange(
        &self,
        platform: &str,
        request: &ExchangeRequest<'_>,
    ) -> Result<BrokerConnection, AuthError> {
        let url = format!("{}/v1/auth/{platform}/exchange", self.base_url);
        self.post_json(&url, request).await
    }

    pub async fn get_session_status(
        &self,
        session_id: &str,
    ) -> Result<BrokerSessionStatus, AuthError> {
        let url = format!("{}/v1/auth/sessions/{session_id}", self.base_url);
        let response = self
            .http
            .get(&url)
            .timeout(BROKER_POLL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        Self::parse_response(response).await
    }

    pub async fn refresh_connection(
        &self,
        connection_id: &str,
    ) -> Result<BrokerConnection, AuthError> {
        let url = format!("{}/v1/connections/{connection_id}/refresh", self.base_url);
        self.post_empty(&url).await
    }

    pub async fn revoke_connection(&self, connection_id: &str) -> Result<(), AuthError> {
        let url = format!("{}/v1/connections/{connection_id}/revoke", self.base_url);
        let response = self
            .http
            .post(&url)
            .send()
            .await
            .map_err(map_transport_err)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(Self::error_from_response(response).await)
        }
    }

    async fn post_json<B: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<R, AuthError> {
        let response = self
            .http
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(map_transport_err)?;
        Self::parse_response(response).await
    }

    async fn post_empty<R: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<R, AuthError> {
        let response = self
            .http
            .post(url)
            .send()
            .await
            .map_err(map_transport_err)?;
        Self::parse_response(response).await
    }

    async fn parse_response<R: for<'de> Deserialize<'de>>(
        response: reqwest::Response,
    ) -> Result<R, AuthError> {
        if response.status().is_success() {
            response
                .json::<R>()
                .await
                .map_err(|e| AuthError::TokenExchangeFailed {
                    detail: format!("malformed broker response: {e}"),
                })
        } else {
            Err(Self::error_from_response(response).await)
        }
    }

    async fn error_from_response(response: reqwest::Response) -> AuthError {
        let status = response.status();
        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            return AuthError::ProviderRateLimited {
                retry_after_seconds: retry_after,
            };
        }
        match response.json::<BrokerErrorBody>().await {
            Ok(body) => match body.code.as_str() {
                "AUTH_STATE_MISMATCH" => AuthError::AuthStateMismatch,
                "AUTH_CODE_INVALID" => AuthError::AuthCodeInvalid,
                "TOKEN_REVOKED" => AuthError::TokenRevoked,
                "PERMISSION_DENIED" => AuthError::PermissionDenied,
                _ => AuthError::TokenExchangeFailed {
                    detail: body.message,
                },
            },
            Err(_) => AuthError::TokenExchangeFailed {
                detail: format!("broker returned HTTP {status}"),
            },
        }
    }
}

fn map_transport_err(err: reqwest::Error) -> AuthError {
    if err.is_timeout() || err.is_connect() {
        AuthError::BrokerUnavailable
    } else {
        AuthError::TokenExchangeFailed {
            detail: err.to_string(),
        }
    }
}
