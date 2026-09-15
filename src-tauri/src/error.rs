use serde::Serialize;

use crate::domain::errors::DomainError;
use crate::domain::ports::platform_connector::PlatformConnectorError;
use crate::domain::ports::secure_storage::SecureStorageError;

/// Error category surfaced to the frontend. The frontend switches on this
/// to decide how to react (retry affordance, "reconnect" prompt, etc.)
/// without parsing message text.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Validation,
    Database,
    Authentication,
    Network,
    RateLimit,
    Media,
    Platform,
    Internal,
}

/// The only error type Tauri commands return. Never `Debug`-dumps a Rust
/// error to the user: `user_message` is always a short, safe sentence,
/// while `developer_message` (logged, and included for local dev builds)
/// carries the detail. Rule: no Rust panic/debug output ever reaches the UI
/// verbatim (section 15).
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub user_message: String,
    pub developer_message: String,
}

impl AppError {
    pub fn new(
        code: ErrorCode,
        user_message: impl Into<String>,
        developer_message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            user_message: user_message.into(),
            developer_message: developer_message.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.developer_message)
    }
}

impl std::error::Error for AppError {}

impl From<DomainError> for AppError {
    fn from(err: DomainError) -> Self {
        match &err {
            DomainError::Validation(msg) => {
                AppError::new(ErrorCode::Validation, msg.clone(), err.to_string())
            }
            DomainError::NotFound { entity, id } => AppError::new(
                ErrorCode::Validation,
                format!("{entity} not found"),
                format!("{entity} {id} not found"),
            ),
            DomainError::InvalidTransition { .. } => AppError::new(
                ErrorCode::Validation,
                "That action isn't valid right now.",
                err.to_string(),
            ),
            DomainError::Conflict(msg) => {
                AppError::new(ErrorCode::Validation, msg.clone(), err.to_string())
            }
            DomainError::Repository(_) => AppError::new(
                ErrorCode::Database,
                "A local database error occurred. Please try again.",
                err.to_string(),
            ),
        }
    }
}

impl From<SecureStorageError> for AppError {
    fn from(err: SecureStorageError) -> Self {
        AppError::new(
            ErrorCode::Internal,
            "Secure storage is unavailable on this system.",
            err.to_string(),
        )
    }
}

impl From<PlatformConnectorError> for AppError {
    fn from(err: PlatformConnectorError) -> Self {
        let code = match err {
            PlatformConnectorError::AuthRequired { .. } => ErrorCode::Authentication,
            PlatformConnectorError::RateLimited { .. } => ErrorCode::RateLimit,
            PlatformConnectorError::NotImplemented { .. } => ErrorCode::Platform,
            PlatformConnectorError::RequestFailed { .. } => ErrorCode::Network,
        };
        AppError::new(
            code,
            "This platform integration isn't available yet.",
            err.to_string(),
        )
    }
}
