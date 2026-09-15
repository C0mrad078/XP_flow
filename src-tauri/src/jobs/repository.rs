use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::errors::DomainResult;

use super::Job;

/// Persistence for [`Job`] rows. Implemented in
/// `infrastructure::repositories::SqliteJobRepository`; lives alongside
/// `Job` in `jobs/` rather than under `domain::ports` because the job
/// vocabulary is its own self-contained concept, not a domain aggregate.
#[async_trait]
pub trait JobRepository: Send + Sync {
    /// Inserts a new job. If `dedupe_key` collides with an already
    /// pending/running job (see the partial unique index in
    /// `migrations/0002_media_library.sql`), returns the *existing* job
    /// instead of erroring — this is what makes enqueueing idempotent
    /// (section 58).
    async fn enqueue(&self, job: &Job) -> DomainResult<Job>;
    async fn mark_running(&self, id: Uuid) -> DomainResult<()>;
    async fn mark_succeeded(&self, id: Uuid) -> DomainResult<()>;
    async fn mark_failed(&self, id: Uuid, error: &str) -> DomainResult<()>;
    /// Startup recovery (section 84/86): any job still `running` from a
    /// previous process that was killed gets reset so it isn't stuck
    /// forever. Returns how many were reset.
    async fn fail_orphaned_running_jobs(&self, reason: &str) -> DomainResult<u64>;
}
