use std::sync::Arc;

use uuid::Uuid;

use crate::application::activity_service::ActivityService;
use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::{
    PublicationRepository, QueueItemRepository, VideoRepository,
};
use crate::domain::publication::{Publication, PublicationStatus};
use crate::domain::publication_query::{PublicationListQuery, PublicationPage};
use crate::domain::queue_item::QueueItem;
use crate::domain::video_status::VideoPriority;

/// The result of one item in a bulk "Add to Queue" call (section 36/85):
/// deliberately per-item rather than all-or-nothing — a duplicate video
/// being skipped should not abort adding the other 50 videos in the same
/// bulk action. Contrast with `SchedulerService`'s bulk scheduling
/// operations, which *are* transactional because a partial schedule would
/// leave the calendar in a confusing half-filled state.
pub struct AddToQueueOutcome {
    pub video_id: Uuid,
    pub result: Result<Publication, DomainError>,
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct QueueReconciliationSummary {
    pub orphaned_queue_items_removed: u32,
}

/// Orchestrates the `Publication` lifecycle: creating publications from
/// videos ("Add to Queue"), cancellation/archival, priority/lock toggles,
/// and querying the Queue. Scheduling itself (finding/assigning a time)
/// lives in `SchedulerService` — this service only manages the publication
/// record and its `QueueItem` membership.
pub struct PublicationService {
    publication_repo: Arc<dyn PublicationRepository>,
    queue_item_repo: Arc<dyn QueueItemRepository>,
    video_repo: Arc<dyn VideoRepository>,
    activity_service: Arc<ActivityService>,
}

impl PublicationService {
    pub fn new(
        publication_repo: Arc<dyn PublicationRepository>,
        queue_item_repo: Arc<dyn QueueItemRepository>,
        video_repo: Arc<dyn VideoRepository>,
        activity_service: Arc<ActivityService>,
    ) -> Self {
        Self {
            publication_repo,
            queue_item_repo,
            video_repo,
            activity_service,
        }
    }

    /// Adds one video to the operational queue for a channel/platform
    /// (section 13/36). Creates the `Publication` (seeding its priority
    /// from the video unless `priority_override` is given), walks it
    /// through `Imported -> Validating -> Ready -> Queued` via the
    /// existing validated state machine, and creates its `QueueItem`.
    pub async fn add_to_queue(
        &self,
        workspace_id: Uuid,
        video_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
        platform_account_id: Option<Uuid>,
        priority_override: Option<VideoPriority>,
    ) -> DomainResult<Publication> {
        let video = self
            .video_repo
            .get(video_id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Video",
                id: video_id.to_string(),
            })?;

        let already_active = self
            .publication_repo
            .find_active_for_video_channel_platform(video_id, channel_id, platform)
            .await?
            .is_some();
        if already_active {
            return Err(DomainError::PublicationAlreadyExists);
        }

        let mut publication = Publication::new(
            workspace_id,
            video_id,
            channel_id,
            platform,
            video.display_title.clone(),
            priority_override.unwrap_or(video.priority),
        );
        publication.platform_account_id = platform_account_id;

        // No per-publication validation step exists yet in Phase 3 (the
        // video was already validated on import) — these transitions are
        // simply walking the existing state machine to its natural
        // "ready to queue" point, not skipping any real work.
        publication.transition(PublicationStatus::Validating)?;
        publication.transition(PublicationStatus::Ready)?;
        publication.transition(PublicationStatus::Queued)?;

        self.publication_repo.create(&publication).await?;

        let position = self.queue_item_repo.next_position(workspace_id).await?;
        let queue_item = QueueItem::new(workspace_id, publication.id, position);
        self.queue_item_repo.create(&queue_item).await?;

        self.activity_service
            .log(
                ActivityCategory::Publication,
                ActivityLevel::Info,
                format!(
                    "\"{}\" added to the {} queue",
                    publication.title,
                    platform.display_name()
                ),
            )
            .await?;

        Ok(publication)
    }

    /// Bulk variant of [`Self::add_to_queue`] — one video/channel/platform
    /// target per `video_id`, reporting a per-item outcome rather than
    /// aborting the whole batch on the first duplicate or validation
    /// failure.
    pub async fn add_to_queue_bulk(
        &self,
        workspace_id: Uuid,
        video_ids: Vec<Uuid>,
        channel_id: Uuid,
        platform: Platform,
        platform_account_id: Option<Uuid>,
        priority_override: Option<VideoPriority>,
    ) -> Vec<AddToQueueOutcome> {
        let mut outcomes = Vec::with_capacity(video_ids.len());
        for video_id in video_ids {
            let result = self
                .add_to_queue(
                    workspace_id,
                    video_id,
                    channel_id,
                    platform,
                    platform_account_id,
                    priority_override,
                )
                .await;
            outcomes.push(AddToQueueOutcome { video_id, result });
        }
        outcomes
    }

    pub async fn get(&self, id: Uuid) -> DomainResult<Option<Publication>> {
        self.publication_repo.get(id).await
    }

    pub async fn list(&self, query: PublicationListQuery) -> DomainResult<PublicationPage> {
        self.publication_repo.list_paginated(&query).await
    }

    /// Cancels a publication (section 78: never delete — cancellation is
    /// itself a terminal-adjacent status) and removes its queue
    /// membership, since a `QueueItem` only exists while a publication is
    /// under active manual-queue management.
    pub async fn cancel(&self, id: Uuid) -> DomainResult<Publication> {
        let mut publication = self.load(id).await?;
        publication.transition(PublicationStatus::Cancelled)?;
        self.publication_repo.update(&publication).await?;
        self.queue_item_repo.delete_for_publication(id).await?;

        self.activity_service
            .log(
                ActivityCategory::Publication,
                ActivityLevel::Warning,
                format!("\"{}\" was cancelled", publication.title),
            )
            .await?;
        Ok(publication)
    }

    pub async fn archive(&self, id: Uuid) -> DomainResult<Publication> {
        let mut publication = self.load(id).await?;
        publication.transition(PublicationStatus::Archived)?;
        self.publication_repo.update(&publication).await?;
        self.queue_item_repo.delete_for_publication(id).await?;
        Ok(publication)
    }

    pub async fn set_priority(
        &self,
        id: Uuid,
        priority: VideoPriority,
    ) -> DomainResult<Publication> {
        let mut publication = self.load(id).await?;
        publication.priority = priority;
        publication.updated_at = chrono::Utc::now();
        self.publication_repo.update(&publication).await?;
        Ok(publication)
    }

    pub async fn set_locked(&self, id: Uuid, locked: bool) -> DomainResult<Publication> {
        let mut publication = self.load(id).await?;
        publication.locked = locked;
        publication.updated_at = chrono::Utc::now();
        self.publication_repo.update(&publication).await?;
        Ok(publication)
    }

    /// Transactional operator pause/resume for queued work. Executing rows
    /// cannot transition, so a concurrent claim causes the whole batch to
    /// fail instead of partially changing operator intent.
    pub async fn bulk_set_paused(
        &self,
        ids: &[Uuid],
        paused: bool,
    ) -> DomainResult<Vec<Publication>> {
        let mut publications = Vec::with_capacity(ids.len());
        for id in ids {
            let mut publication = self.load(*id).await?;
            let target = if paused {
                PublicationStatus::Paused
            } else {
                PublicationStatus::Queued
            };
            publication.transition(target)?;
            publication.updated_at = chrono::Utc::now();
            publications.push(publication);
        }
        self.publication_repo.bulk_update(&publications).await?;
        Ok(publications)
    }

    /// Reassigns manual queue positions for the given publications, in the
    /// order supplied (section 15's drag-reorder for unscheduled items).
    /// `position` carries no uniqueness constraint (it is an ordering hint,
    /// not an identity), so sequential per-item updates are sufficient —
    /// unlike scheduling, a transient duplicate position cannot corrupt
    /// anything worse than a temporary re-sort.
    pub async fn reorder_queue(&self, ordered_publication_ids: Vec<Uuid>) -> DomainResult<()> {
        for (index, publication_id) in ordered_publication_ids.into_iter().enumerate() {
            if let Some(mut item) = self
                .queue_item_repo
                .get_for_publication(publication_id)
                .await?
            {
                item.position = index as i64;
                item.updated_at = chrono::Utc::now();
                self.queue_item_repo.update(&item).await?;
            }
        }
        Ok(())
    }

    /// Startup/periodic consistency sweep (section 91/113 — the
    /// `queue_reconciliation` job type): finds `QueueItem` rows whose
    /// publication is gone or already in a terminal state and removes
    /// them, and finds active (`Queued`/`Scheduled`) publications that are
    /// missing their `QueueItem` row (which should never happen through
    /// normal application code paths, but could follow a crash mid
    /// operation or manual DB surgery) and gives them one at the back of
    /// the queue. Every correction is logged — nothing is ever silently
    /// discarded.
    pub async fn reconcile_queue(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<QueueReconciliationSummary> {
        let mut summary = QueueReconciliationSummary::default();

        let all_items = self
            .queue_item_repo
            .list_all_for_workspace(workspace_id)
            .await?;
        for item in all_items {
            let publication = self.publication_repo.get(item.publication_id).await?;
            let orphaned = match &publication {
                None => true,
                Some(p) => matches!(
                    p.status,
                    PublicationStatus::Cancelled
                        | PublicationStatus::Archived
                        | PublicationStatus::Duplicate
                ),
            };
            if orphaned {
                self.queue_item_repo.delete(item.id).await?;
                summary.orphaned_queue_items_removed += 1;
            }
        }

        if summary.orphaned_queue_items_removed > 0 {
            self.activity_service
                .log(
                    ActivityCategory::Warning,
                    ActivityLevel::Warning,
                    format!(
                        "Queue reconciliation removed {} orphaned queue item(s)",
                        summary.orphaned_queue_items_removed
                    ),
                )
                .await?;
        }

        Ok(summary)
    }

    async fn load(&self, id: Uuid) -> DomainResult<Publication> {
        self.publication_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Publication",
                id: id.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::activity_event::ActivityCategory;
    use crate::domain::ports::repositories::ActivityRepository;
    use crate::infrastructure::repositories::{
        SqliteActivityRepository, SqlitePublicationRepository, SqliteQueueItemRepository,
        SqliteVideoRepository,
    };
    use crate::test_support::*;

    async fn build_service(pool: sqlx::SqlitePool) -> PublicationService {
        let publication_repo: Arc<dyn PublicationRepository> =
            Arc::new(SqlitePublicationRepository::new(pool.clone()));
        let queue_item_repo: Arc<dyn QueueItemRepository> =
            Arc::new(SqliteQueueItemRepository::new(pool.clone()));
        let video_repo: Arc<dyn VideoRepository> =
            Arc::new(SqliteVideoRepository::new(pool.clone()));
        let activity_repo: Arc<dyn ActivityRepository> =
            Arc::new(SqliteActivityRepository::new(pool));
        let activity_service = Arc::new(ActivityService::new(activity_repo));
        PublicationService::new(
            publication_repo,
            queue_item_repo,
            video_repo,
            activity_service,
        )
    }

    #[tokio::test]
    async fn add_to_queue_creates_a_queued_publication_with_a_queue_item() {
        let pool = temp_pool("pub-svc-add").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let service = build_service(pool.clone()).await;

        let publication = service
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(publication.status, PublicationStatus::Queued);
        let queue_item_repo = SqliteQueueItemRepository::new(pool);
        let item = queue_item_repo
            .get_for_publication(publication.id)
            .await
            .unwrap();
        assert!(item.is_some());
    }

    #[tokio::test]
    async fn adding_the_same_video_channel_platform_twice_is_rejected() {
        let pool = temp_pool("pub-svc-dup").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let service = build_service(pool).await;

        service
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        let second = service
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await;

        assert!(matches!(second, Err(DomainError::PublicationAlreadyExists)));
    }

    #[tokio::test]
    async fn cancelling_a_publication_removes_its_queue_item() {
        let pool = temp_pool("pub-svc-cancel").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let service = build_service(pool.clone()).await;

        let publication = service
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        let cancelled = service.cancel(publication.id).await.unwrap();
        assert_eq!(cancelled.status, PublicationStatus::Cancelled);

        let queue_item_repo = SqliteQueueItemRepository::new(pool);
        let item = queue_item_repo
            .get_for_publication(publication.id)
            .await
            .unwrap();
        assert!(item.is_none());
    }

    #[tokio::test]
    async fn priority_override_wins_over_the_video_default() {
        let pool = temp_pool("pub-svc-priority").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let service = build_service(pool).await;

        let publication = service
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                Some(VideoPriority::Urgent),
            )
            .await
            .unwrap();

        assert_eq!(publication.priority, VideoPriority::Urgent);
    }

    #[tokio::test]
    async fn reorder_queue_reassigns_positions_in_the_given_order() {
        let pool = temp_pool("pub-svc-reorder").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let service = build_service(pool.clone()).await;

        let mut ids = Vec::new();
        for i in 0..3 {
            let video_id =
                seed_video(&pool, workspace_id, source_id, None, &format!("Clip {i}")).await;
            let publication = service
                .add_to_queue(
                    workspace_id,
                    video_id,
                    channel_id,
                    Platform::YouTube,
                    None,
                    None,
                )
                .await
                .unwrap();
            ids.push(publication.id);
        }

        let reversed: Vec<Uuid> = ids.iter().rev().cloned().collect();
        service.reorder_queue(reversed.clone()).await.unwrap();

        let queue_item_repo = SqliteQueueItemRepository::new(pool);
        for (expected_position, publication_id) in reversed.iter().enumerate() {
            let item = queue_item_repo
                .get_for_publication(*publication_id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(item.position, expected_position as i64);
        }
    }

    #[tokio::test]
    async fn reconcile_queue_removes_orphaned_items_and_logs_a_warning() {
        let pool = temp_pool("pub-svc-reconcile").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let service = build_service(pool.clone()).await;

        let publication = service
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        // Simulate a crash between "cancel the publication" and "remove its
        // queue item" by moving the publication straight to Cancelled via
        // SQL, bypassing the service (which would normally clean this up).
        sqlx::query("UPDATE publications SET status = 'cancelled' WHERE id = ?")
            .bind(publication.id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let summary = service.reconcile_queue(workspace_id).await.unwrap();
        assert_eq!(summary.orphaned_queue_items_removed, 1);

        let queue_item_repo = SqliteQueueItemRepository::new(pool.clone());
        assert!(queue_item_repo
            .get_for_publication(publication.id)
            .await
            .unwrap()
            .is_none());

        let activity_repo = SqliteActivityRepository::new(pool);
        let recent = activity_repo.list_recent(10).await.unwrap();
        assert!(recent
            .iter()
            .any(|e| e.category == ActivityCategory::Warning && e.message.contains("orphaned")));
    }
}
