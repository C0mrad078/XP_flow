use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::config::TikTokCredentials;
use crate::error::BrokerError;

use super::{ProviderIdentity, ProviderTokens};

// Hardcoded, allowlisted TikTok endpoints (section 25 — no generic proxy,
// no caller-supplied URL ever reaches this client).
const TOKEN_ENDPOINT: &str = "https://open.tiktokapis.com/v2/oauth/token/";
const REVOKE_ENDPOINT: &str = "https://open.tiktokapis.com/v2/oauth/revoke/";
const USER_INFO_ENDPOINT: &str = "https://open.tiktokapis.com/v2/user/info/";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    // Not present on an error response — must stay optional or a
    // provider error body (no access_token at all) fails to deserialize
    // and gets misreported as a generic transport/`ProviderUnavailable`
    // failure instead of the specific error TikTok actually returned.
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    refresh_expires_in: Option<i64>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    data: Option<UserInfoData>,
    error: Option<UserInfoError>,
}

#[derive(Debug, Deserialize)]
struct UserInfoData {
    user: UserInfoUser,
}

#[derive(Debug, Deserialize)]
struct UserInfoUser {
    open_id: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoError {
    code: Option<String>,
    message: Option<String>,
}

pub struct TikTokProviderClient {
    http: reqwest::Client,
    credentials: TikTokCredentials,
    token_endpoint: String,
    revoke_endpoint: String,
    user_info_endpoint: String,
}

impl TikTokProviderClient {
    pub fn new(credentials: TikTokCredentials) -> Self {
        Self {
            http: reqwest::Client::new(),
            credentials,
            token_endpoint: TOKEN_ENDPOINT.to_string(),
            revoke_endpoint: REVOKE_ENDPOINT.to_string(),
            user_info_endpoint: USER_INFO_ENDPOINT.to_string(),
        }
    }

    /// Test-only seam so integration tests can point this client at a
    /// local mock server — every production route handler always
    /// constructs via `new` (section 25/90), never this; it's `pub` only
    /// because `tests/` is a separate crate that needs it, not because
    /// any HTTP-facing code path can reach it.
    pub fn with_base_url(credentials: TikTokCredentials, base_url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            credentials,
            token_endpoint: format!("{base_url}/oauth/token"),
            revoke_endpoint: format!("{base_url}/oauth/revoke"),
            user_info_endpoint: format!("{base_url}/user/info"),
        }
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, BrokerError> {
        let params = [
            ("client_key", self.credentials.client_key.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];
        self.post_token(&self.token_endpoint, &params).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<ProviderTokens, BrokerError> {
        let params = [
            ("client_key", self.credentials.client_key.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        self.post_token(&self.token_endpoint, &params).await
    }

    pub async fn revoke(&self, access_token: &str) -> Result<(), BrokerError> {
        let params = [
            ("client_key", self.credentials.client_key.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("token", access_token),
        ];
        let response = self
            .http
            .post(&self.revoke_endpoint)
            .form(&params)
            .send()
            .await
            .map_err(map_transport_err)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(BrokerError::ProviderUnavailable)
        }
    }

    pub async fn fetch_identity(
        &self,
        access_token: &str,
    ) -> Result<ProviderIdentity, BrokerError> {
        let response = self
            .http
            .get(&self.user_info_endpoint)
            .bearer_auth(access_token)
            .query(&[("fields", "open_id,display_name,avatar_url")])
            .send()
            .await
            .map_err(map_transport_err)?;

        if !response.status().is_success() {
            return Err(BrokerError::ProviderUnavailable);
        }
        let body: UserInfoResponse = response
            .json()
            .await
            .map_err(|_| BrokerError::ProviderUnavailable)?;
        if let Some(error) = body.error {
            if error.code.as_deref().is_some_and(|c| c != "ok") {
                tracing::warn!(code = ?error.code, message = ?error.message, "tiktok user info request failed");
                return Err(BrokerError::ProviderUnavailable);
            }
        }
        let user = body.data.ok_or(BrokerError::ProviderUnavailable)?.user;
        Ok(ProviderIdentity {
            provider_account_id: user.open_id,
            display_name: user.display_name,
            username_or_handle: None,
            avatar_url: user.avatar_url,
        })
    }

    async fn post_token(
        &self,
        endpoint: &str,
        params: &[(&str, &str)],
    ) -> Result<ProviderTokens, BrokerError> {
        let response = self
            .http
            .post(endpoint)
            .form(params)
            .send()
            .await
            .map_err(map_transport_err)?;
        let status = response.status();
        let body: TokenResponse = response
            .json()
            .await
            .map_err(|_| BrokerError::ProviderUnavailable)?;

        if let Some(error) = body.error {
            tracing::warn!(error, detail = ?body.error_description, "tiktok token request failed");
            return Err(match error.as_str() {
                "access_denied" => BrokerError::PermissionDenied,
                "invalid_grant" => BrokerError::CodeInvalid,
                _ if status.as_u16() == 429 => BrokerError::RateLimited,
                _ => BrokerError::ProviderUnavailable,
            });
        }

        Ok(ProviderTokens {
            access_token: body.access_token.ok_or(BrokerError::ProviderUnavailable)?,
            refresh_token: body.refresh_token,
            access_expires_at: body
                .expires_in
                .map(|s| Utc::now() + Duration::seconds(s.max(0))),
            refresh_expires_at: body
                .refresh_expires_in
                .map(|s| Utc::now() + Duration::seconds(s.max(0))),
            granted_scopes: body
                .scope
                .map(|s| s.split(',').map(str::to_string).collect())
                .unwrap_or_default(),
        })
    }
}

fn map_transport_err(_err: reqwest::Error) -> BrokerError {
    BrokerError::ProviderUnavailable
}
