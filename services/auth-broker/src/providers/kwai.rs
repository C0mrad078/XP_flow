use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::config::KwaiCredentials;
use crate::error::BrokerError;

use super::{ProviderIdentity, ProviderTokens};

// Hardcoded, allowlisted Kwai Open Platform endpoints (section 25).
const AUTHORIZATION_ENDPOINT: &str = "https://open.kwai.com/oauth2/authorize";
const TOKEN_ENDPOINT: &str = "https://open.kwai.com/oauth2/access_token";
const REFRESH_ENDPOINT: &str = "https://open.kwai.com/oauth2/refresh_token";
const USER_INFO_ENDPOINT: &str = "https://open.kwai.com/openapi/user/info";

#[derive(Debug, Deserialize)]
struct TokenResponse {
    result: Option<i32>,
    error_msg: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    refresh_token_expires_in: Option<i64>,
    scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    result: Option<i32>,
    open_id: Option<String>,
    nick_name: Option<String>,
    head_url: Option<String>,
}

pub struct KwaiProviderClient {
    http: reqwest::Client,
    credentials: KwaiCredentials,
    authorize_endpoint: String,
    token_endpoint: String,
    refresh_endpoint: String,
    user_info_endpoint: String,
}

impl KwaiProviderClient {
    pub fn new(credentials: KwaiCredentials) -> Self {
        Self {
            http: reqwest::Client::new(),
            credentials,
            authorize_endpoint: AUTHORIZATION_ENDPOINT.to_string(),
            token_endpoint: TOKEN_ENDPOINT.to_string(),
            refresh_endpoint: REFRESH_ENDPOINT.to_string(),
            user_info_endpoint: USER_INFO_ENDPOINT.to_string(),
        }
    }

    /// Test-only seam — see `TikTokProviderClient::with_base_url`.
    pub fn with_base_url(credentials: KwaiCredentials, base_url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            credentials,
            authorize_endpoint: format!("{base_url}/oauth2/authorize"),
            token_endpoint: format!("{base_url}/oauth2/access_token"),
            refresh_endpoint: format!("{base_url}/oauth2/refresh_token"),
            user_info_endpoint: format!("{base_url}/user/info"),
        }
    }

    pub fn build_authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
        scopes: &[String],
    ) -> String {
        let mut url = url::Url::parse(&self.authorize_endpoint).expect("static/test URL is valid");
        url.query_pairs_mut()
            .append_pair("app_id", &self.credentials.app_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &scopes.join(","))
            .append_pair("state", state);
        url.to_string()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<ProviderTokens, BrokerError> {
        let params = [
            ("app_id", self.credentials.app_id.as_str()),
            ("app_secret", self.credentials.app_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];
        self.post_token(&self.token_endpoint, &params).await
    }

    pub async fn refresh(&self, refresh_token: &str) -> Result<ProviderTokens, BrokerError> {
        let params = [
            ("app_id", self.credentials.app_id.as_str()),
            ("app_secret", self.credentials.app_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        self.post_token(&self.refresh_endpoint, &params).await
    }

    pub async fn revoke(&self, _access_token: &str) -> Result<(), BrokerError> {
        // Kwai's Open Platform does not currently document a token-
        // revocation endpoint (section 56 only requires attempting
        // provider revocation "where supported") — the local connection
        // is still marked revoked by the caller regardless.
        Ok(())
    }

    pub async fn fetch_identity(
        &self,
        access_token: &str,
    ) -> Result<ProviderIdentity, BrokerError> {
        let response = self
            .http
            .get(&self.user_info_endpoint)
            .bearer_auth(access_token)
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
        if body.result != Some(1) {
            return Err(BrokerError::ProviderUnavailable);
        }
        Ok(ProviderIdentity {
            provider_account_id: body.open_id.ok_or(BrokerError::ProviderUnavailable)?,
            display_name: body.nick_name,
            username_or_handle: None,
            avatar_url: body.head_url,
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

        if body.result != Some(1) {
            tracing::warn!(error = ?body.error_msg, "kwai token request failed");
            return Err(if status.as_u16() == 429 {
                BrokerError::RateLimited
            } else {
                BrokerError::CodeInvalid
            });
        }

        Ok(ProviderTokens {
            access_token: body.access_token.ok_or(BrokerError::ProviderUnavailable)?,
            refresh_token: body.refresh_token,
            access_expires_at: body
                .expires_in
                .map(|s| Utc::now() + Duration::seconds(s.max(0))),
            refresh_expires_at: body
                .refresh_token_expires_in
                .map(|s| Utc::now() + Duration::seconds(s.max(0))),
            granted_scopes: body.scopes.unwrap_or_default(),
        })
    }
}

fn map_transport_err(_err: reqwest::Error) -> BrokerError {
    BrokerError::ProviderUnavailable
}
