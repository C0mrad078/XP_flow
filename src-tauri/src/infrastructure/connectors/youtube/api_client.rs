use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::domain::auth_error::AuthError;
use crate::infrastructure::auth::http_client::build_auth_http_client;

use super::config::YouTubeAuthConfig;

// Hardcoded, allowlisted Google endpoints (section 25's no-generic-proxy
// principle applies just as much to a direct-provider client as to the
// broker — never build a URL from caller input here).
const AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const REVOKE_ENDPOINT: &str = "https://oauth2.googleapis.com/revoke";
const USERINFO_ENDPOINT: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const CHANNELS_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/channels";

#[derive(Debug, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub granted_scopes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GoogleIdentity {
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub handle: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChannelsResponse {
    items: Vec<ChannelItem>,
}

#[derive(Debug, Deserialize)]
struct ChannelItem {
    id: String,
    snippet: Option<ChannelSnippet>,
}

#[derive(Debug, Deserialize)]
struct ChannelSnippet {
    title: Option<String>,
    #[serde(rename = "customUrl")]
    custom_url: Option<String>,
    thumbnails: Option<ChannelThumbnails>,
}

#[derive(Debug, Deserialize)]
struct ChannelThumbnails {
    default: Option<ChannelThumbnail>,
}

#[derive(Debug, Deserialize)]
struct ChannelThumbnail {
    url: String,
}

/// Thin, provider-specific HTTP transport for Google's OAuth + YouTube
/// Data API (section 36 — kept separate from `mapper`/auth-flow
/// orchestration so this file only ever knows "how to call Google").
pub struct YouTubeApiClient {
    http: reqwest::Client,
    config: YouTubeAuthConfig,
}

impl YouTubeApiClient {
    pub fn new(config: YouTubeAuthConfig) -> Self {
        Self {
            http: build_auth_http_client(),
            config,
        }
    }

    pub fn build_authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        code_challenge: &str,
        scopes: &[String],
    ) -> String {
        let scope_param = scopes.join(" ");
        let mut url = url::Url::parse(AUTHORIZATION_ENDPOINT).expect("static URL is valid");
        url.query_pairs_mut()
            .append_pair("client_id", &self.config.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", &scope_param)
            .append_pair("state", state)
            .append_pair("code_challenge", code_challenge)
            .append_pair("code_challenge_method", "S256")
            // Ensures a refresh token is actually issued (section 37 needs
            // one) even on a repeat authorization for the same account.
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");
        url.to_string()
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        code_verifier: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse, AuthError> {
        let mut params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ];
        if let Some(secret) = &self.config.client_secret {
            params.push(("client_secret", secret));
        }
        self.post_token_request(&params).await
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, AuthError> {
        let mut params = vec![
            ("client_id", self.config.client_id.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ];
        if let Some(secret) = &self.config.client_secret {
            params.push(("client_secret", secret));
        }
        self.post_token_request(&params).await
    }

    async fn post_token_request(
        &self,
        params: &[(&str, &str)],
    ) -> Result<TokenResponse, AuthError> {
        let response = self
            .http
            .post(TOKEN_ENDPOINT)
            .form(params)
            .send()
            .await
            .map_err(map_transport_err)?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(classify_token_error(status.as_u16()));
        }

        let raw: RawTokenResponse =
            response
                .json()
                .await
                .map_err(|e| AuthError::TokenExchangeFailed {
                    detail: format!("malformed token response: {e}"),
                })?;

        Ok(TokenResponse {
            access_token: raw.access_token,
            refresh_token: raw.refresh_token,
            expires_at: Utc::now() + Duration::seconds(raw.expires_in.max(0)),
            granted_scopes: raw
                .scope
                .map(|s| s.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
        })
    }

    /// Fetches the authenticated user's channel — the authoritative
    /// identity for a YouTube `PlatformAccount` (section 9).
    pub async fn fetch_authenticated_channel(
        &self,
        access_token: &str,
    ) -> Result<GoogleIdentity, AuthError> {
        let response = self
            .http
            .get(CHANNELS_ENDPOINT)
            .bearer_auth(access_token)
            .query(&[("part", "snippet"), ("mine", "true")])
            .send()
            .await
            .map_err(map_transport_err)?;

        if !response.status().is_success() {
            return Err(classify_token_error(response.status().as_u16()));
        }

        let body: ChannelsResponse =
            response
                .json()
                .await
                .map_err(|e| AuthError::TokenExchangeFailed {
                    detail: format!("malformed channels response: {e}"),
                })?;

        let channel =
            body.items
                .into_iter()
                .next()
                .ok_or_else(|| AuthError::TokenExchangeFailed {
                    detail: "the authorized Google account has no YouTube channel".to_string(),
                })?;
        let snippet = channel.snippet.unwrap_or(ChannelSnippet {
            title: None,
            custom_url: None,
            thumbnails: None,
        });

        Ok(GoogleIdentity {
            provider_account_id: channel.id,
            display_name: snippet.title,
            handle: snippet.custom_url,
            avatar_url: snippet.thumbnails.and_then(|t| t.default).map(|d| d.url),
        })
    }

    /// Best-effort revocation (section 56) — Google's revoke endpoint;
    /// failures here must never block a local disconnect.
    pub async fn revoke(&self, token: &str) -> Result<(), AuthError> {
        let response = self
            .http
            .post(REVOKE_ENDPOINT)
            .form(&[("token", token)])
            .send()
            .await
            .map_err(map_transport_err)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(classify_token_error(response.status().as_u16()))
        }
    }

    /// Lightweight identity check via the OpenID `userinfo` endpoint —
    /// used by `validate_connection` when a full channel re-fetch isn't
    /// necessary.
    pub async fn fetch_userinfo_subject(&self, access_token: &str) -> Result<String, AuthError> {
        #[derive(Deserialize)]
        struct UserInfo {
            sub: String,
        }
        let response = self
            .http
            .get(USERINFO_ENDPOINT)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(map_transport_err)?;
        if !response.status().is_success() {
            return Err(classify_token_error(response.status().as_u16()));
        }
        let info: UserInfo = response
            .json()
            .await
            .map_err(|e| AuthError::TokenExchangeFailed {
                detail: format!("malformed userinfo response: {e}"),
            })?;
        Ok(info.sub)
    }
}

fn classify_token_error(status: u16) -> AuthError {
    match status {
        401 | 403 => AuthError::TokenRevoked,
        429 => AuthError::ProviderRateLimited {
            retry_after_seconds: None,
        },
        500..=599 => AuthError::ProviderUnavailable {
            detail: format!("Google returned HTTP {status}"),
        },
        _ => AuthError::TokenExchangeFailed {
            detail: format!("Google returned HTTP {status}"),
        },
    }
}

fn map_transport_err(err: reqwest::Error) -> AuthError {
    if err.is_timeout() || err.is_connect() {
        AuthError::NetworkOffline
    } else {
        AuthError::TokenExchangeFailed {
            detail: err.to_string(),
        }
    }
}
