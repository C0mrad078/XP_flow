use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::publish_error::PublishError;
use crate::domain::platform::Platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl AttemptStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AttemptStatus::Pending => "pending",
            AttemptStatus::Running => "running",
            AttemptStatus::Succeeded => "succeeded",
            AttemptStatus::Failed => "failed",
            AttemptStatus::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AttemptStatus::Succeeded | AttemptStatus::Failed | AttemptStatus::Cancelled
        )
    }
}

impl std::fmt::Display for AttemptStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AttemptStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pending" => AttemptStatus::Pending,
            "running" => AttemptStatus::Running,
            "succeeded" => AttemptStatus::Succeeded,
            "failed" => AttemptStatus::Failed,
            "cancelled" => AttemptStatus::Cancelled,
            other => return Err(format!("unknown attempt status: {other}")),
        })
    }
}

/// One execution attempt of a [`crate::domain::publication::Publication`]
/// (section 9) — durable history that survives across retries, never
/// overwritten. `retry_count`/`last_error` on `Publication` stay as the
/// coarse "current state" summary; this is the full trail behind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationAttempt {
    pub id: Uuid,
    pub publication_id: Uuid,
    pub attempt_number: i32,
    pub provider: Platform,
    pub status: AttemptStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub bytes_total: Option<i64>,
    pub bytes_uploaded: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retryable: Option<bool>,
    pub remote_operation_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PublicationAttempt {
    pub fn new(publication_id: Uuid, attempt_number: i32, provider: Platform) -> Self {
        Self {
            id: Uuid::new_v4(),
            publication_id,
            attempt_number,
            provider,
            status: AttemptStatus::Pending,
            started_at: None,
            completed_at: None,
            bytes_total: None,
            bytes_uploaded: None,
            error_code: None,
            error_message: None,
            retryable: None,
            remote_operation_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn start(&mut self) {
        self.status = AttemptStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn succeed(&mut self, remote_operation_id: Option<String>) {
        self.status = AttemptStatus::Succeeded;
        self.completed_at = Some(Utc::now());
        self.retryable = Some(false);
        if remote_operation_id.is_some() {
            self.remote_operation_id = remote_operation_id;
        }
    }

    pub fn fail(&mut self, error: &PublishError) {
        self.status = AttemptStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_code = Some(error.code().to_string());
        self.error_message = Some(error.user_message());
        self.retryable = Some(error.is_retryable());
    }

    pub fn cancel(&mut self) {
        self.status = AttemptStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.retryable = Some(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_attempt_starts_pending_with_no_timestamps() {
        let attempt = PublicationAttempt::new(Uuid::new_v4(), 1, Platform::YouTube);
        assert_eq!(attempt.status, AttemptStatus::Pending);
        assert!(attempt.started_at.is_none());
        assert!(attempt.completed_at.is_none());
    }

    #[test]
    fn failing_records_the_retryable_verdict_from_the_error() {
        let mut attempt = PublicationAttempt::new(Uuid::new_v4(), 1, Platform::YouTube);
        attempt.start();
        attempt.fail(&PublishError::NetworkTransient);
        assert_eq!(attempt.status, AttemptStatus::Failed);
        assert_eq!(attempt.retryable, Some(true));
        assert_eq!(attempt.error_code.as_deref(), Some("NETWORK_TRANSIENT"));
    }

    #[test]
    fn succeeding_never_leaves_retryable_ambiguous() {
        let mut attempt = PublicationAttempt::new(Uuid::new_v4(), 1, Platform::YouTube);
        attempt.start();
        attempt.succeed(Some("remote-123".to_string()));
        assert_eq!(attempt.status, AttemptStatus::Succeeded);
        assert_eq!(attempt.retryable, Some(false));
        assert_eq!(attempt.remote_operation_id.as_deref(), Some("remote-123"));
    }
}
