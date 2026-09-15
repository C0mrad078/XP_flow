pub mod application;
pub mod commands;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod jobs;
pub mod persistence;
pub mod platform;
pub mod services;
pub mod state;

use std::sync::Arc;

use application::activity_service::ActivityService;
use application::settings_service::SettingsService;
use application::workspace_service::WorkspaceService;
use domain::activity_event::{ActivityCategory, ActivityLevel};
use domain::ports::media_service::MediaService;
use domain::ports::repositories::{
    ActivityRepository, NotificationRepository, SettingsRepository, WorkspaceRepository,
};
use infrastructure::media::FfmpegMediaService;
use infrastructure::repositories::{
    SqliteActivityRepository, SqliteNotificationRepository, SqliteSettingsRepository,
    SqliteWorkspaceRepository,
};
use platform::paths::AppPaths;
use services::media_status_service::MediaStatusService;
use services::notification_service::NotificationService;
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = AppPaths::resolve().expect("failed to resolve application data directories");

    // The WorkerGuard must outlive every `tracing::*!` call; leaking it is
    // deliberate — it is dropped only at process exit, which is exactly
    // when we want the log writer to flush and stop.
    let log_guard = infrastructure::logging::init(&paths.log_dir);
    Box::leak(Box::new(log_guard));

    tracing::info!(data_dir = %paths.data_dir.display(), "XP FLOW starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let paths = paths.clone();
            let state = tauri::async_runtime::block_on(async move { bootstrap(paths).await })
                .expect("failed to initialize XP FLOW backend");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::workspace_commands::get_current_workspace,
            commands::workspace_commands::create_workspace,
            commands::settings_commands::get_settings,
            commands::settings_commands::update_settings,
            commands::activity_commands::list_recent_activity,
            commands::notification_commands::list_recent_notifications,
            commands::notification_commands::unread_notification_count,
            commands::notification_commands::mark_notification_read,
            commands::system_commands::get_app_info,
            commands::system_commands::get_media_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Opens the database, runs pending migrations, wires every repository into
/// its application service, and records the startup trail the Activity
/// screen shows on first launch (section 30's worked example: "Application
/// started" / "Database migration completed").
async fn bootstrap(paths: AppPaths) -> Result<AppState, Box<dyn std::error::Error>> {
    let pool = persistence::db::init_pool(&paths.database_path()).await?;
    tracing::info!("database migrations applied");

    let workspace_repo: Arc<dyn WorkspaceRepository> =
        Arc::new(SqliteWorkspaceRepository::new(pool.clone()));
    let activity_repo: Arc<dyn ActivityRepository> =
        Arc::new(SqliteActivityRepository::new(pool.clone()));
    let settings_repo: Arc<dyn SettingsRepository> =
        Arc::new(SqliteSettingsRepository::new(pool.clone()));
    let notification_repo: Arc<dyn NotificationRepository> =
        Arc::new(SqliteNotificationRepository::new(pool.clone()));
    let media_service: Arc<dyn MediaService> = Arc::new(FfmpegMediaService::new());

    let activity_service = Arc::new(ActivityService::new(activity_repo.clone()));
    let workspace_service = Arc::new(WorkspaceService::new(workspace_repo, activity_repo));
    let settings_service = Arc::new(SettingsService::new(settings_repo));
    let notification_service = Arc::new(NotificationService::new(notification_repo));
    let media_status_service = Arc::new(MediaStatusService::new(media_service));

    activity_service
        .log(
            ActivityCategory::System,
            ActivityLevel::Info,
            "Application started",
        )
        .await?;
    activity_service
        .log(
            ActivityCategory::System,
            ActivityLevel::Success,
            "Database migration completed",
        )
        .await?;

    Ok(AppState {
        workspace_service,
        settings_service,
        activity_service,
        media_status_service,
        notification_service,
        paths,
    })
}
