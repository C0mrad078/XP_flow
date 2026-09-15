use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The work a [`Job`] performs. Section 44 of the Phase 1 brief defined the
/// vocabulary; Phase 2 (section 57) is the first phase that actually
/// executes some of these — `ScanFolderJob`, `IngestVideoJob`,
/// `ValidateVideoJob`, `GenerateThumbnailJob` and `ReconcileSourceJob` run
/// for real through [`crate::services::job_runner::JobRunner`]. The
/// publishing-related variants remain unused placeholders until Phase 3.
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
    ScanFolder,
    IngestVideo,
    ReconcileSource,
    /// Auto-places one publication into its channel's next available slot
    /// (section 22/25).
    AutoSchedule,
    /// Recomputes `scheduled_at` for every unlocked scheduled/queued
    /// publication on a channel, e.g. after its schedule slots changed
    /// (section 30/82 "Rebuild Schedule").
    RebuildSchedule,
    /// Walks a channel's near-term calendar looking for empty slots and
    /// auto-fills them from the unscheduled queue, priority-first
    /// (section 26 "Fill Empty Slots").
    FillScheduleGaps,
    /// Startup/periodic consistency sweep: detects orphaned queue items,
    /// stale locks and other queue/schedule drift (section 91/113).
    QueueReconciliation,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::ValidateVideo => "validate_video",
            JobType::PublishVideo => "publish_video",
            JobType::CollectMetrics => "collect_metrics",
            JobType::FetchComments => "fetch_comments",
            JobType::GenerateThumbnail => "generate_thumbnail",
            JobType::Backup => "backup",
            JobType::Cleanup => "cleanup",
            JobType::ScanFolder => "scan_folder",
            JobType::IngestVideo => "ingest_video",
            JobType::ReconcileSource => "reconcile_source",
            JobType::AutoSchedule => "auto_schedule",
            JobType::RebuildSchedule => "rebuild_schedule",
            JobType::FillScheduleGaps => "fill_schedule_gaps",
            JobType::QueueReconciliation => "queue_reconciliation",
        }
    }
}

impl std::str::FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "validate_video" => JobType::ValidateVideo,
            "publish_video" => JobType::PublishVideo,
            "collect_metrics" => JobType::CollectMetrics,
            "fetch_comments" => JobType::FetchComments,
            "generate_thumbnail" => JobType::GenerateThumbnail,
            "backup" => JobType::Backup,
            "cleanup" => JobType::Cleanup,
            "scan_folder" => JobType::ScanFolder,
            "ingest_video" => JobType::IngestVideo,
            "reconcile_source" => JobType::ReconcileSource,
            "auto_schedule" => JobType::AutoSchedule,
            "rebuild_schedule" => JobType::RebuildSchedule,
            "fill_schedule_gaps" => JobType::FillScheduleGaps,
            "queue_reconciliation" => JobType::QueueReconciliation,
            other => return Err(format!("unknown job type: {other}")),
        })
    }
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

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Succeeded => "succeeded",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pending" => JobStatus::Pending,
            "running" => JobStatus::Running,
            "succeeded" => JobStatus::Succeeded,
            "failed" => JobStatus::Failed,
            "cancelled" => JobStatus::Cancelled,
            other => return Err(format!("unknown job status: {other}")),
        })
    }
}

/// A unit of background work, persisted in the `jobs` table.
///
/// `dedupe_key` is how ingestion stays idempotent (section 58): a partial
/// unique index (see `migrations/0002_media_library.sql`) rejects a second
/// `pending`/`running` job sharing the same key, so three duplicate
/// filesystem events for the same file enqueue exactly one ingest job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub payload_json: String,
    pub dedupe_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempts: i32,
    pub last_error: Option<String>,
}

impl Job {
    pub fn new(job_type: JobType, payload_json: String, dedupe_key: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            job_type,
            status: JobStatus::Pending,
            payload_json,
            dedupe_key,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            attempts: 0,
            last_error: None,
        }
    }
}
