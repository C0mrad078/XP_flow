use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::activity_event::ActivityEvent;
use crate::domain::app_settings::AppSettings;
use crate::domain::channel::Channel;
use crate::domain::duplicate_match::DuplicateMatch;
use crate::domain::errors::DomainResult;
use crate::domain::notification::Notification;
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::publication::Publication;
use crate::domain::publication_query::{PublicationListQuery, PublicationPage};
use crate::domain::queue_item::QueueItem;
use crate::domain::schedule_exception::ScheduleException;
use crate::domain::schedule_slot::ScheduleSlot;
use crate::domain::video::Video;
use crate::domain::video_query::{VideoLibrarySummary, VideoListQuery, VideoPage};
use crate::domain::video_source::VideoSource;
use crate::domain::workspace::Workspace;

/// Repository contracts (ports) that the domain/application layers depend
/// on. Concrete implementations live in `infrastructure::repositories` and
/// talk to SQLite through SQLx — nothing in `domain` or `application` knows
/// that SQLite exists, which is what lets Phase 2 add remote sync without a
/// domain rewrite.
#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn create(&self, workspace: &Workspace) -> DomainResult<()>;
    async fn update(&self, workspace: &Workspace) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<Workspace>>;
    async fn list(&self) -> DomainResult<Vec<Workspace>>;
    /// The workspace XP FLOW treats as "current" for Phase 1's
    /// single-workspace UI (the most recently created one).
    async fn get_current(&self) -> DomainResult<Option<Workspace>>;
}

#[async_trait]
pub trait ChannelRepository: Send + Sync {
    async fn create(&self, channel: &Channel) -> DomainResult<()>;
    async fn update(&self, channel: &Channel) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<Channel>>;
    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Channel>>;
}

#[async_trait]
pub trait PlatformAccountRepository: Send + Sync {
    async fn create(&self, account: &PlatformAccount) -> DomainResult<()>;
    async fn update(&self, account: &PlatformAccount) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<PlatformAccount>>;
    async fn list_for_channel(&self, channel_id: Uuid) -> DomainResult<Vec<PlatformAccount>>;
    /// Every platform account in the workspace, regardless of channel —
    /// backs the Integrations page and the Dashboard's platform-health
    /// summary (sections 47/72) in one query rather than one per channel.
    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<PlatformAccount>>;
    /// The account "Add to Queue" should default to for this channel and
    /// platform (section 49) — the one flagged `default_target`, or the
    /// channel's only account on that platform if there is exactly one.
    async fn get_default_for_channel_platform(
        &self,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<Option<PlatformAccount>>;
    /// Looks up an existing connection by the provider's own account id
    /// (section 53/59) — used both to reject a duplicate connection and to
    /// detect "you authorized a different account than before" on
    /// reconnect.
    async fn find_by_provider_identity(
        &self,
        workspace_id: Uuid,
        platform: Platform,
        provider_account_id: &str,
    ) -> DomainResult<Option<PlatformAccount>>;
    /// Every `Connected` account whose `access_expires_at` falls at or
    /// before `before` — the token-lifecycle background job's work queue
    /// (section 37/39).
    async fn list_due_for_refresh(
        &self,
        before: DateTime<Utc>,
    ) -> DomainResult<Vec<PlatformAccount>>;
}

#[async_trait]
pub trait VideoRepository: Send + Sync {
    async fn create(&self, video: &Video) -> DomainResult<()>;
    async fn update(&self, video: &Video) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<Video>>;
    async fn get_by_hash(
        &self,
        workspace_id: Uuid,
        content_hash: &str,
    ) -> DomainResult<Option<Video>>;
    async fn get_by_path(&self, workspace_id: Uuid, file_path: &str)
        -> DomainResult<Option<Video>>;
    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Video>>;
    /// `(id, file_path)` for every non-archived video from one source —
    /// used by reconciliation to diff a folder scan against what's
    /// already indexed without loading full rows (section 16/17).
    async fn list_paths_for_source(&self, source_id: Uuid) -> DomainResult<Vec<(Uuid, String)>>;
    async fn count_for_source(&self, source_id: Uuid) -> DomainResult<i64>;
    /// Recent perceptual hashes in the workspace, for near-duplicate
    /// comparison (section 28) — bounded so this never becomes an
    /// O(library size) scan on every import.
    async fn list_recent_perceptual_hashes(
        &self,
        workspace_id: Uuid,
        limit: i64,
    ) -> DomainResult<Vec<(Uuid, String)>>;
    async fn list_paginated(&self, query: &VideoListQuery) -> DomainResult<VideoPage>;
    async fn summary(&self, workspace_id: Uuid) -> DomainResult<VideoLibrarySummary>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
}

#[async_trait]
pub trait VideoSourceRepository: Send + Sync {
    async fn create(&self, source: &VideoSource) -> DomainResult<()>;
    async fn update(&self, source: &VideoSource) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<VideoSource>>;
    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<VideoSource>>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
}

#[async_trait]
pub trait DuplicateMatchRepository: Send + Sync {
    async fn create(&self, duplicate: &DuplicateMatch) -> DomainResult<()>;
    async fn list_for_video(&self, video_id: Uuid) -> DomainResult<Vec<DuplicateMatch>>;
}

#[async_trait]
pub trait PublicationRepository: Send + Sync {
    async fn create(&self, publication: &Publication) -> DomainResult<()>;
    async fn update(&self, publication: &Publication) -> DomainResult<()>;
    /// Persists every publication in `publications` inside a single
    /// database transaction — all-or-nothing. Used by bulk scheduling
    /// operations (auto-schedule, rebuild, fill-gaps) so a mid-batch slot
    /// conflict never leaves half the batch scheduled and half not
    /// (section 85/113).
    async fn bulk_update(&self, publications: &[Publication]) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<Publication>>;
    async fn list_for_video(&self, video_id: Uuid) -> DomainResult<Vec<Publication>>;
    /// Any publication for this exact (video, channel, platform) triple
    /// that is not yet in a terminal/cancelled state — used to enforce
    /// "one active publication per video/channel/platform" at the
    /// application layer (section 50/51), backstopped by a DB index.
    async fn find_active_for_video_channel_platform(
        &self,
        video_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<Option<Publication>>;
    async fn list_paginated(&self, query: &PublicationListQuery) -> DomainResult<PublicationPage>;
    /// Every `Scheduled` publication in `[from, to)` UTC, for a given
    /// channel or the whole workspace when `channel_id` is `None` — backs
    /// the Calendar (section 41-45) and slot-conflict checks.
    async fn list_scheduled_in_range(
        &self,
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>>;
    /// `Scheduled` publications whose `scheduled_at` is at or before `now`
    /// — the due-publication query Phase 5's real uploader will consume
    /// (section 111), unused for actual publishing in Phase 3.
    async fn list_due(
        &self,
        workspace_id: Uuid,
        now: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>>;
    /// All unlocked, non-terminal, non-scheduled queue publications for a
    /// channel — the candidate pool the auto-scheduler draws from.
    async fn list_unscheduled_for_channel(
        &self,
        channel_id: Uuid,
    ) -> DomainResult<Vec<Publication>>;
    /// `(channel_id, count)` of `Queued`/`Scheduled` publications for
    /// every channel in the workspace, in one query — the aggregate the
    /// Channels screen's overview needs instead of one query per card
    /// (section 96's N+1 fix).
    async fn count_active_grouped_by_channel(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<(Uuid, i64)>>;

    /// Atomically claims one due, unlocked, still-`Scheduled` publication
    /// for execution — `Scheduled -> Uploading` only succeeds if the row
    /// is exactly in that state when the `UPDATE ... WHERE` runs, so two
    /// concurrent callers (a periodic scan tick and a manual "Publish
    /// Now," or two overlapping ticks) can never both win the same
    /// publication (section 12/13/66). `execution_key` is only set if
    /// still `NULL` — a retry reuses the same key; a claim never
    /// generates a new one for a publication that already has one.
    /// Returns `true` iff this call won the claim.
    #[allow(clippy::too_many_arguments)]
    async fn try_claim_due(
        &self,
        id: Uuid,
        candidate_execution_key: Uuid,
        claim_token: &str,
        lease_expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> DomainResult<bool>;

    /// `Uploading`/`Processing` publications whose lease has already
    /// expired — an abandoned claim from a process that crashed before
    /// releasing it (section 13/88). The crash-recovery pass reconciles
    /// each of these against its persisted attempt/session state before
    /// anything is allowed to touch them again.
    async fn list_with_expired_leases(
        &self,
        workspace_id: Uuid,
        now: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>>;
}

#[async_trait]
pub trait QueueItemRepository: Send + Sync {
    async fn create(&self, item: &QueueItem) -> DomainResult<()>;
    async fn update(&self, item: &QueueItem) -> DomainResult<()>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
    /// Removes the queue-membership row for a publication, if any — called
    /// on cancel/archive since a `QueueItem` only exists while a
    /// publication is under active manual-queue management.
    async fn delete_for_publication(&self, publication_id: Uuid) -> DomainResult<()>;
    async fn get_for_publication(&self, publication_id: Uuid) -> DomainResult<Option<QueueItem>>;
    /// Unscheduled items only, ordered by `position` — scheduled
    /// publications are ordered by `scheduled_at` instead and are read
    /// through `PublicationRepository`.
    async fn list_unscheduled_for_workspace(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<QueueItem>>;
    /// Every `QueueItem` row in the workspace regardless of whether its
    /// publication is scheduled — used only by reconciliation, which needs
    /// to find rows whose publication has gone missing or terminal
    /// (section 91/113), not just the unscheduled ones the UI cares about.
    async fn list_all_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<QueueItem>>;
    async fn next_position(&self, workspace_id: Uuid) -> DomainResult<i64>;
}

#[async_trait]
pub trait ScheduleSlotRepository: Send + Sync {
    async fn create(&self, slot: &ScheduleSlot) -> DomainResult<()>;
    async fn update(&self, slot: &ScheduleSlot) -> DomainResult<()>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
    async fn get(&self, id: Uuid) -> DomainResult<Option<ScheduleSlot>>;
    async fn list_for_channel(&self, channel_id: Uuid) -> DomainResult<Vec<ScheduleSlot>>;
    /// `(channel_id, active_slot_count)` for every channel in the
    /// workspace, in one query (section 96's N+1 fix — see
    /// `PublicationRepository::count_active_grouped_by_channel`).
    async fn count_active_grouped_by_channel(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<(Uuid, i64)>>;
}

#[async_trait]
pub trait ScheduleExceptionRepository: Send + Sync {
    async fn create(&self, exception: &ScheduleException) -> DomainResult<()>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
    async fn list_for_channel_in_range(
        &self,
        channel_id: Uuid,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> DomainResult<Vec<ScheduleException>>;
    async fn exists_for_channel_date(
        &self,
        channel_id: Uuid,
        date: chrono::NaiveDate,
    ) -> DomainResult<bool>;
}

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn record(&self, event: &ActivityEvent) -> DomainResult<()>;
    async fn list_recent(&self, limit: i64) -> DomainResult<Vec<ActivityEvent>>;
}

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn create(&self, notification: &Notification) -> DomainResult<()>;
    async fn list_recent(&self, limit: i64) -> DomainResult<Vec<Notification>>;
    async fn unread_count(&self) -> DomainResult<i64>;
    async fn mark_read(&self, id: Uuid) -> DomainResult<()>;
}

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get(&self) -> DomainResult<AppSettings>;
    async fn save(&self, settings: &AppSettings) -> DomainResult<()>;
}
