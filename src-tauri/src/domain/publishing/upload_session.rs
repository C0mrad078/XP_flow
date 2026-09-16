use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::remote_state::RemoteUploadState;
use crate::domain::platform::Platform;

/// What kind of multi-step transfer this session represents — purely
/// descriptive (used for display/debugging), never branched on by shared
/// engine code (section 5/7: providers are allowed genuinely different
/// upload shapes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    /// YouTube's resumable upload protocol (section 40).
    Resumable,
    /// TikTok's Direct Post FILE_UPLOAD flow (section 49-52).
    DirectPost,
    /// Kwai's start/upload/complete/publish flow (section 58-61).
    Stepwise,
}

impl SessionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionType::Resumable => "resumable",
            SessionType::DirectPost => "direct_post",
            SessionType::Stepwise => "stepwise",
        }
    }
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SessionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "resumable" => SessionType::Resumable,
            "direct_post" => SessionType::DirectPost,
            "stepwise" => SessionType::Stepwise,
            other => return Err(format!("unknown session type: {other}")),
        })
    }
}

/// Recoverable, provider-specific upload progress (section 10). This is
/// what a crash-recovery pass reads to decide whether an interrupted
/// upload can resume in place, must be re-verified, or is safe to
/// restart — see [`RemoteUploadState::safe_to_restart`].
///
/// `remote_upload_url` and `remote_upload_token` are real bearer-
/// equivalent secrets for the duration of their validity (typically
/// minutes to an hour) — a leaked TikTok `upload_url`, for instance, lets
/// anyone with it complete the pending post. They're stored locally (this
/// is local-first, single-user SQLite, the same trust tier as everything
/// else in this database) but never allowed into a log line, which is
/// what the manual `Debug` impl below enforces at the type level, same
/// discipline as `LocalCredential`/`Pkce`.
#[derive(Clone, Serialize, Deserialize)]
pub struct UploadSession {
    pub id: Uuid,
    pub publication_id: Uuid,
    pub attempt_id: Uuid,
    pub provider: Platform,
    pub session_type: SessionType,
    pub remote_session_id: Option<String>,
    pub remote_upload_url: Option<String>,
    pub remote_publish_id: Option<String>,
    pub remote_upload_token: Option<String>,
    pub bytes_total: Option<i64>,
    pub bytes_committed: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub state: RemoteUploadState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for UploadSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadSession")
            .field("id", &self.id)
            .field("publication_id", &self.publication_id)
            .field("attempt_id", &self.attempt_id)
            .field("provider", &self.provider)
            .field("session_type", &self.session_type)
            .field("remote_session_id", &self.remote_session_id)
            .field(
                "remote_upload_url",
                &self.remote_upload_url.as_ref().map(|_| "[redacted]"),
            )
            .field("remote_publish_id", &self.remote_publish_id)
            .field(
                "remote_upload_token",
                &self.remote_upload_token.as_ref().map(|_| "[redacted]"),
            )
            .field("bytes_total", &self.bytes_total)
            .field("bytes_committed", &self.bytes_committed)
            .field("expires_at", &self.expires_at)
            .field("state", &self.state)
            .finish()
    }
}

impl UploadSession {
    pub fn new(
        publication_id: Uuid,
        attempt_id: Uuid,
        provider: Platform,
        session_type: SessionType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            publication_id,
            attempt_id,
            provider,
            session_type,
            remote_session_id: None,
            remote_upload_url: None,
            remote_publish_id: None,
            remote_upload_token: None,
            bytes_total: None,
            bytes_committed: 0,
            expires_at: None,
            state: RemoteUploadState::NotStarted,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_the_upload_url_or_token() {
        let mut session = UploadSession::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Platform::TikTok,
            SessionType::DirectPost,
        );
        session.remote_upload_url =
            Some("https://upload.tiktokapis.com/secret-signed-url".to_string());
        session.remote_upload_token = Some("super-secret-token".to_string());

        let debug_output = format!("{session:?}");
        assert!(!debug_output.contains("secret-signed-url"));
        assert!(!debug_output.contains("super-secret-token"));
        assert!(debug_output.contains("[redacted]"));
    }

    #[test]
    fn is_expired_respects_the_boundary() {
        let mut session = UploadSession::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Platform::Kwai,
            SessionType::Stepwise,
        );
        let now = Utc::now();
        session.expires_at = Some(now - chrono::Duration::seconds(1));
        assert!(session.is_expired(now));
        session.expires_at = Some(now + chrono::Duration::seconds(1));
        assert!(!session.is_expired(now));
        session.expires_at = None;
        assert!(!session.is_expired(now));
    }
}
