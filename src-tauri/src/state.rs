use std::sync::Arc;

use crate::application::activity_service::ActivityService;
use crate::application::settings_service::SettingsService;
use crate::application::workspace_service::WorkspaceService;
use crate::platform::paths::AppPaths;
use crate::services::media_status_service::MediaStatusService;
use crate::services::notification_service::NotificationService;

/// Everything a Tauri command needs, assembled once at startup and shared
/// (via `Arc`) across every invocation. Commands depend only on this
/// struct's application-layer services — never on a repository or the
/// SQLite pool directly (Rule 1: the frontend, and by extension the
/// command layer that talks to it, never touches SQLite itself).
pub struct AppState {
    pub workspace_service: Arc<WorkspaceService>,
    pub settings_service: Arc<SettingsService>,
    pub activity_service: Arc<ActivityService>,
    pub media_status_service: Arc<MediaStatusService>,
    pub notification_service: Arc<NotificationService>,
    pub paths: AppPaths,
}
