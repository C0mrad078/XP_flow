use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::activity_event::ActivityEvent;
use crate::domain::app_settings::AppSettings;
use crate::domain::channel::Channel;
use crate::domain::errors::DomainResult;
use crate::domain::notification::Notification;
use crate::domain::publication::Publication;
use crate::domain::video::Video;
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
    async fn list_for_workspace(&self, workspace_id: Uuid) -> DomainResult<Vec<Video>>;
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
