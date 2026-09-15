use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The work a [`Job`] performs. This is the complete set section 44 of the
/// Phase 1 brief asks for; only the *types* exist in this phase — there is
/// no scheduler, no persistence, and no worker pool executing them yet
/// (see `docs/architecture.md` for what Phase 2 adds on top of this).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    ValidateVideo,
    PublishVideo,
    CollectMetrics,
    FetchComments,
    GenerateThumbnail,
    Backup,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// A unit of background work. Modeled now so the future job runner
/// (Tokio-task based, see section 45) and its persistence table can be
/// added without redesigning what a "job" is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub last_error: Option<String>,
}

impl Job {
    pub fn new(job_type: JobType) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type,
            status: JobStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            attempts: 0,
            last_error: None,
        }
    }
}
