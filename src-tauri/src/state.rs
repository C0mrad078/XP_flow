use std::sync::Arc;

use crate::application::activity_service::ActivityService;
use crate::application::channel_service::ChannelService;
use crate::application::content_service::ContentService;
use crate::application::settings_service::SettingsService;
use crate::application::source_service::SourceService;
use crate::application::workspace_service::WorkspaceService;
use crate::domain::ports::repositories::VideoRepository;
use crate::platform::paths::AppPaths;
use crate::services::job_runner::JobRunner;
use crate::services::media_status_service::MediaStatusService;
use crate::services::notification_service::NotificationService;

/// Everything a Tauri command needs, assembled once at startup and shared
/// (via `Arc`) across every invocation. Commands depend only on this
/// struct's application-layer services — never on a repository or the
/// SQLite pool directly (Rule 1: the frontend, and by extension the
/// command layer that talks to it, never touches SQLite itself).
///
/// `video_repo` is the one exception: the local-video-preview protocol
/// handler (`commands::media_protocol`) needs to resolve a video id to a
/// file path on every request and doing that through `ContentService`
/// would pull in duplicate-match/warning computation it doesn't need.
pub struct AppState {
    pub workspace_service: Arc<WorkspaceService>,
    pub settings_service: Arc<SettingsService>,
    pub activity_service: Arc<ActivityService>,
    pub media_status_service: Arc<MediaStatusService>,
    pub notification_service: Arc<NotificationService>,
    pub content_service: Arc<ContentService>,
    pub source_service: Arc<SourceService>,
    pub channel_service: Arc<ChannelService>,
    pub job_runner: Arc<JobRunner>,
    pub video_repo: Arc<dyn VideoRepository>,
    pub paths: AppPaths,
}
