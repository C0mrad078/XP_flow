use thiserror::Error;

/// Every publishing failure normalized into one of these before it ever
/// reaches an attempt row or a command boundary (section 70) — mirrors
/// `domain::auth_error::AuthError`'s exact shape and rationale.
#[derive(Debug, Error, Clone)]
pub enum PublishError {
    #[error("a transient network error occurred")]
    NetworkTransient,

    #[error("the request timed out")]
    Timeout,

    #[error("the provider returned a server error")]
    ProviderServerError { status: Option<u16> },

    #[error("provider rate limit exceeded")]
    RateLimited { retry_after_seconds: Option<u64> },

    #[error("the account's access credential has expired")]
    AuthExpired,

    #[error("the account's connection was revoked")]
    AuthRevoked,

    #[error("a required publishing permission is missing")]
    PermissionMissing { capability: String },

    #[error("the media file is invalid for this provider: {detail}")]
    InvalidMedia { detail: String },

    #[error("publication metadata is invalid: {detail}")]
    InvalidMetadata { detail: String },

    #[error("the provider failed to process the uploaded media: {detail}")]
    RemoteProcessingFailed { detail: String },

    #[error("the upload session expired before the transfer completed")]
    UploadSessionExpired,

    #[error("the remote outcome of the last request is unknown")]
    UnknownRemoteResult,

    #[error("express consent is required before this can be published")]
    ConsentRequired,

    #[error("this provider application is not approved for this operation: {detail}")]
    PlatformNotApproved { detail: String },

    #[error("publishing was cancelled")]
    Cancelled,

    #[error("the source video file is missing or changed since it was queued")]
    VideoUnavailable,

    #[error("internal error: {detail}")]
    Internal { detail: String },
}

impl PublishError {
    pub fn code(&self) -> &'static str {
        match self {
            PublishError::NetworkTransient => "NETWORK_TRANSIENT",
            PublishError::Timeout => "TIMEOUT",
            PublishError::ProviderServerError { .. } => "PROVIDER_5XX",
            PublishError::RateLimited { .. } => "RATE_LIMIT",
            PublishError::AuthExpired => "AUTH_EXPIRED",
            PublishError::AuthRevoked => "AUTH_REVOKED",
            PublishError::PermissionMissing { .. } => "PERMISSION_MISSING",
            PublishError::InvalidMedia { .. } => "INVALID_MEDIA",
            PublishError::InvalidMetadata { .. } => "INVALID_METADATA",
            PublishError::RemoteProcessingFailed { .. } => "REMOTE_PROCESSING_FAILED",
            PublishError::UploadSessionExpired => "UPLOAD_SESSION_EXPIRED",
            PublishError::UnknownRemoteResult => "UNKNOWN_REMOTE_RESULT",
            PublishError::ConsentRequired => "CONSENT_REQUIRED",
            PublishError::PlatformNotApproved { .. } => "PLATFORM_NOT_APPROVED",
            PublishError::Cancelled => "CANCELLED",
            PublishError::VideoUnavailable => "VIDEO_UNAVAILABLE",
            PublishError::Internal { .. } => "INTERNAL",
        }
    }

    /// Safe to show a user directly — never a raw provider response body.
    pub fn user_message(&self) -> String {
        match self {
            PublishError::NetworkTransient => {
                "A network error interrupted the upload. It will be retried.".to_string()
            }
            PublishError::Timeout => "The request timed out. It will be retried.".to_string(),
            PublishError::ProviderServerError { .. } => {
                "The platform reported a temporary server error. It will be retried.".to_string()
            }
            PublishError::RateLimited { .. } => {
                "The platform is rate-limiting requests right now.".to_string()
            }
            PublishError::AuthExpired => {
                "This account's connection expired. Reconnect to continue publishing.".to_string()
            }
            PublishError::AuthRevoked => {
                "This account's connection was revoked. Reconnect to continue publishing."
                    .to_string()
            }
            PublishError::PermissionMissing { capability } => {
                format!("Publishing permission is missing on this account ({capability}). Reconnect and grant it.")
            }
            PublishError::InvalidMedia { detail } => {
                format!("This video can't be published to this platform: {detail}")
            }
            PublishError::InvalidMetadata { detail } => {
                format!("This publication's metadata is invalid: {detail}")
            }
            PublishError::RemoteProcessingFailed { .. } => {
                "The platform failed to process the uploaded video.".to_string()
            }
            PublishError::UploadSessionExpired => {
                "The upload session expired. A new one will be started.".to_string()
            }
            PublishError::UnknownRemoteResult => {
                "The platform's response was unclear — verifying before trying again.".to_string()
            }
            PublishError::ConsentRequired => {
                "This publication needs your approval before it can go out.".to_string()
            }
            PublishError::PlatformNotApproved { detail } => {
                format!("This platform hasn't approved this feature for this app yet: {detail}")
            }
            PublishError::Cancelled => "Publishing was cancelled.".to_string(),
            PublishError::VideoUnavailable => {
                "The source video is missing or changed since it was queued.".to_string()
            }
            PublishError::Internal { .. } => "Something went wrong while publishing.".to_string(),
        }
    }

    /// Whether an automatic retry is safe (section 71/72). Anything that
    /// requires user action (auth, permissions, metadata, consent,
    /// invalid media) must never be silently retried — and an
    /// `UnknownRemoteResult` is *never* automatically retryable: retrying
    /// blind after an ambiguous outcome is exactly how a duplicate post
    /// gets created (section 3/69). It can only move forward after an
    /// explicit remote-state verification step, not a generic retry.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            PublishError::NetworkTransient
                | PublishError::Timeout
                | PublishError::ProviderServerError { .. }
                | PublishError::RateLimited { .. }
                | PublishError::UploadSessionExpired
        )
    }

    pub fn retry_after_seconds(&self) -> Option<u64> {
        match self {
            PublishError::RateLimited {
                retry_after_seconds,
            } => *retry_after_seconds,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_remote_result_is_never_automatically_retryable() {
        assert!(!PublishError::UnknownRemoteResult.is_retryable());
    }

    #[test]
    fn user_action_required_errors_are_never_retryable() {
        assert!(!PublishError::AuthExpired.is_retryable());
        assert!(!PublishError::AuthRevoked.is_retryable());
        assert!(!PublishError::PermissionMissing {
            capability: "upload_video".into()
        }
        .is_retryable());
        assert!(!PublishError::InvalidMetadata {
            detail: String::new()
        }
        .is_retryable());
        assert!(!PublishError::InvalidMedia {
            detail: String::new()
        }
        .is_retryable());
        assert!(!PublishError::ConsentRequired.is_retryable());
        assert!(!PublishError::Cancelled.is_retryable());
    }

    #[test]
    fn transient_errors_are_retryable() {
        assert!(PublishError::NetworkTransient.is_retryable());
        assert!(PublishError::Timeout.is_retryable());
        assert!(PublishError::ProviderServerError { status: Some(503) }.is_retryable());
        assert!(PublishError::RateLimited {
            retry_after_seconds: Some(60)
        }
        .is_retryable());
        assert!(PublishError::UploadSessionExpired.is_retryable());
    }
}
