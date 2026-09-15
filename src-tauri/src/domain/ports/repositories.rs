use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::activity_event::ActivityEvent;
use crate::domain::app_settings::AppSettings;
use crate::domain::channel::Channel;
use crate::domain::duplicate_match::DuplicateMatch;
use crate::domain::errors::DomainResult;
use crate::domain::notification::Notification;
use crate::domain::publication::Publication;
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
    async fn get(&self, id: Uuid) -> DomainResult<Option<Workspace>>;
    async fn list(&self) -> DomainResult<Vec<Workspace>>;
    /// The workspace XP FLOW treats as "current" for Phase 1's
    /// single-workspace UI (the most recently created one).
    async fn get_current(&self) -> DomainResult<Option<Workspace>>;
}

#[async_trait]
pub trait ChannelRepository: Send + Sync {
    async fn create(&self, channel: &Channel) -> DomainResult<()>;
    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Channel>>;
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
    async fn list_for_video(&self, video_id: Uuid) -> DomainResult<Vec<Publication>>;
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
