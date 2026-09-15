use thiserror::Error;

/// Structured platform-authentication errors (section 69 — "do not make
/// the UI interpret raw provider JSON"). Every provider/broker failure is
/// normalized into one of these before it ever reaches a command boundary.
#[derive(Debug, Error, Clone)]
pub enum AuthError {
    #[error("authorization was cancelled")]
    AuthCancelled,

    #[error("authorization timed out")]
    AuthTimeout,

    #[error("authorization state did not match")]
    AuthStateMismatch,

    #[error("authorization code was invalid or already used")]
    AuthCodeInvalid,

    #[error("token exchange failed: {detail}")]
    TokenExchangeFailed { detail: String },

    #[error("token refresh failed: {detail}")]
    TokenRefreshFailed { detail: String },

    #[error("the provider revoked this connection")]
    TokenRevoked,

    #[error("the user denied the requested permissions")]
    PermissionDenied,

    #[error("a required permission is missing: {capability}")]
    PermissionMissing { capability: String },

    #[error("the authorized account does not match the previously connected identity")]
    AccountIdentityMismatch {
        previous_account_id: String,
        new_account_id: String,
    },

    #[error("provider rate limit exceeded")]
    ProviderRateLimited { retry_after_seconds: Option<u64> },

    #[error("provider is currently unavailable: {detail}")]
    ProviderUnavailable { detail: String },

    #[error("the authentication broker is unreachable")]
    BrokerUnavailable,

    #[error("the authentication broker is misconfigured: {detail}")]
    BrokerConfigurationError { detail: String },

    #[error("this provider is not configured: {detail}")]
    ProviderNotConfigured { detail: String },

    #[error("network is offline")]
    NetworkOffline,
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::AuthCancelled => "AUTH_CANCELLED",
            AuthError::AuthTimeout => "AUTH_TIMEOUT",
            AuthError::AuthStateMismatch => "AUTH_STATE_MISMATCH",
            AuthError::AuthCodeInvalid => "AUTH_CODE_INVALID",
            AuthError::TokenExchangeFailed { .. } => "TOKEN_EXCHANGE_FAILED",
            AuthError::TokenRefreshFailed { .. } => "TOKEN_REFRESH_FAILED",
            AuthError::TokenRevoked => "TOKEN_REVOKED",
            AuthError::PermissionDenied => "PERMISSION_DENIED",
            AuthError::PermissionMissing { .. } => "PERMISSION_MISSING",
            AuthError::AccountIdentityMismatch { .. } => "ACCOUNT_IDENTITY_MISMATCH",
            AuthError::ProviderRateLimited { .. } => "PROVIDER_RATE_LIMITED",
            AuthError::ProviderUnavailable { .. } => "PROVIDER_UNAVAILABLE",
            AuthError::BrokerUnavailable => "BROKER_UNAVAILABLE",
            AuthError::BrokerConfigurationError { .. } => "BROKER_CONFIGURATION_ERROR",
            AuthError::ProviderNotConfigured { .. } => "PROVIDER_NOT_CONFIGURED",
            AuthError::NetworkOffline => "NETWORK_OFFLINE",
        }
    }

    /// Safe to show a user directly — never a raw provider/broker response body.
    pub fn user_message(&self) -> String {
        match self {
            AuthError::AuthCancelled => "Authorization was cancelled.".to_string(),
            AuthError::AuthTimeout => "Authorization timed out. Please try again.".to_string(),
            AuthError::AuthStateMismatch | AuthError::AuthCodeInvalid => {
                "That authorization request is no longer valid. Please try connecting again.".to_string()
            }
            AuthError::TokenExchangeFailed { .. } | AuthError::TokenRefreshFailed { .. } => {
                "XP FLOW couldn't complete authorization with the provider.".to_string()
            }
            AuthError::TokenRevoked => "This connection was revoked by the provider. Please reconnect.".to_string(),
            AuthError::PermissionDenied => "The requested permissions were not granted.".to_string(),
            AuthError::PermissionMissing { capability } => {
                format!("This account is missing a required permission ({capability}).")
            }
            AuthError::AccountIdentityMismatch { .. } => {
                "You authorized a different account than the one already connected.".to_string()
            }
            AuthError::ProviderRateLimited { .. } => {
                "The platform is rate-limiting requests right now. Please try again shortly.".to_string()
            }
            AuthError::ProviderUnavailable { .. } => "The platform is currently unavailable.".to_string(),
            AuthError::BrokerUnavailable => {
                "Unable to reach the XP FLOW authentication service. Your local content and schedules are unaffected.".to_string()
            }
            AuthError::BrokerConfigurationError { .. } => {
                "The authentication service is misconfigured. Please contact support.".to_string()
            }
            AuthError::ProviderNotConfigured { .. } => {
                "This platform isn't configured yet in this build of XP FLOW.".to_string()
            }
            AuthError::NetworkOffline => "You appear to be offline. Please check your connection.".to_string(),
        }
    }

    /// Whether a limited automatic retry is safe (section 68). Invalid-
    /// grant/denied/state-mismatch errors must never be retried — they
    /// require the user (or a fresh session) to act.
    pub fn is_safely_retryable(&self) -> bool {
        matches!(
            self,
            AuthError::ProviderUnavailable { .. }
                | AuthError::BrokerUnavailable
                | AuthError::NetworkOffline
        )
    }
}
