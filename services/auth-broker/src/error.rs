use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Every error the broker's HTTP surface can return, normalized to a
/// stable code (section 69) and a message that is always safe to log and
/// return — never a raw provider response body, never a `Debug` dump of
/// an internal error (section 26: "safe error responses").
#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("state did not match")]
    StateMismatch,
    #[error("authorization code was invalid or already used")]
    CodeInvalid,
    #[error("session not found or already consumed")]
    SessionNotFound,
    #[error("session expired")]
    SessionExpired,
    #[error("connection not found")]
    ConnectionNotFound,
    #[error("the provider revoked this connection")]
    TokenRevoked,
    #[error("the user denied the requested permissions")]
    PermissionDenied,
    #[error("this provider is not configured on this broker instance")]
    ProviderNotConfigured,
    #[error("the provider is currently unavailable")]
    ProviderUnavailable,
    #[error("provider rate limit exceeded")]
    RateLimited,
    #[error("request rate limit exceeded")]
    TooManyRequests,
    #[error("invalid request")]
    BadRequest(String),
    #[error("internal error")]
    Internal,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

impl BrokerError {
    fn code(&self) -> &'static str {
        match self {
            BrokerError::StateMismatch => "AUTH_STATE_MISMATCH",
            BrokerError::CodeInvalid => "AUTH_CODE_INVALID",
            BrokerError::SessionNotFound => "SESSION_NOT_FOUND",
            BrokerError::SessionExpired => "SESSION_EXPIRED",
            BrokerError::ConnectionNotFound => "CONNECTION_NOT_FOUND",
            BrokerError::TokenRevoked => "TOKEN_REVOKED",
            BrokerError::PermissionDenied => "PERMISSION_DENIED",
            BrokerError::ProviderNotConfigured => "BROKER_CONFIGURATION_ERROR",
            BrokerError::ProviderUnavailable => "PROVIDER_UNAVAILABLE",
            BrokerError::RateLimited => "PROVIDER_RATE_LIMITED",
            BrokerError::TooManyRequests => "TOO_MANY_REQUESTS",
            BrokerError::BadRequest(_) => "BAD_REQUEST",
            BrokerError::Internal => "INTERNAL_ERROR",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            BrokerError::StateMismatch
            | BrokerError::CodeInvalid
            | BrokerError::BadRequest(_)
            | BrokerError::PermissionDenied => StatusCode::BAD_REQUEST,
            BrokerError::SessionNotFound | BrokerError::ConnectionNotFound => StatusCode::NOT_FOUND,
            BrokerError::SessionExpired | BrokerError::TokenRevoked => StatusCode::GONE,
            BrokerError::ProviderNotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            BrokerError::ProviderUnavailable => StatusCode::BAD_GATEWAY,
            BrokerError::RateLimited | BrokerError::TooManyRequests => {
                StatusCode::TOO_MANY_REQUESTS
            }
            BrokerError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for BrokerError {
    fn into_response(self) -> Response {
        // Never echoes `self.to_string()` for the Internal variant — that
        // message might carry a wrapped sqlx/reqwest detail (see the
        // `From` impls below); everything user/caller-facing beyond the
        // stable code stays generic for that one variant specifically.
        let message = match &self {
            BrokerError::Internal => "an internal error occurred".to_string(),
            other => other.to_string(),
        };
        let body = ErrorBody {
            code: self.code().to_string(),
            message,
        };
        (self.status(), Json(body)).into_response()
    }
}

impl From<sqlx::Error> for BrokerError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!(error = %err, "database error");
        BrokerError::Internal
    }
}

impl From<crate::crypto::CryptoError> for BrokerError {
    fn from(err: crate::crypto::CryptoError) -> Self {
        tracing::error!(error = %err, "crypto error");
        BrokerError::Internal
    }
}
