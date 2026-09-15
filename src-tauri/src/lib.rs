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
#[cfg(test)]
pub mod test_support;

use std::sync::Arc;
use std::time::Duration;

use application::activity_service::ActivityService;
use application::channel_service::ChannelService;
use application::content_service::ContentService;
use application::media_ingestion_service::MediaIngestionService;
use application::settings_service::SettingsService;
use application::source_service::SourceService;
use application::workspace_service::WorkspaceService;
use domain::activity_event::{ActivityCategory, ActivityLevel};
use domain::ports::hashing::{ContentHashService, PerceptualHashService};
use domain::ports::media_service::{MediaProbeService, MediaService, ThumbnailService};
use domain::ports::repositories::{
    ActivityRepository, ChannelRepository, DuplicateMatchRepository, NotificationRepository,
    SettingsRepository, VideoRepository, VideoSourceRepository, WorkspaceRepository,
};
use infrastructure::hashing::{DHashPerceptualHashService, Sha256ContentHashService};
use infrastructure::media::{FfmpegMediaService, FfmpegThumbnailService, FfprobeMediaProbeService};
use infrastructure::repositories::{
    SqliteActivityRepository, SqliteChannelRepository, SqliteDuplicateMatchRepository,
    SqliteJobRepository, SqliteNotificationRepository, SqliteSettingsRepository,
    SqliteVideoRepository, SqliteVideoSourceRepository, SqliteWorkspaceRepository,
};
use infrastructure::watcher::FolderWatcherService;
use jobs::JobRepository;
use platform::paths::AppPaths;
use services::job_runner::JobRunner;
use services::media_status_service::MediaStatusService;
use services::notification_service::NotificationService;
use state::AppState;
use tauri::Manager;

/// Reconciliation backstop interval (section 17) — real-time watching
/// handles almost everything; this exists only to catch filesystem events
/// the OS watcher might have dropped.
const PERIODIC_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let paths = AppPaths::resolve().expect("failed to resolve application data directories");

    // The WorkerGuard must outlive every `tracing::*!` call; leaking it is
    // deliberate — it is dropped only at process exit, which is exactly
    // when we want the log writer to flush and stop.
    let log_guard = infrastructure::logging::init(&paths.log_dir);
    Box::leak(Box::new(log_guard));

    tracing::info!(data_dir = %paths.data_dir.display(), "XP FLOW starting up");

    let builder = tauri::Builder::default()
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
            commands::system_commands::get_cache_info,
            commands::system_commands::clear_temp_cache,
            commands::content_commands::list_content,
            commands::content_commands::get_content_summary,
            commands::content_commands::get_video_detail,
            commands::content_commands::update_video,
            commands::content_commands::bulk_update_videos,
            commands::content_commands::set_video_archived,
            commands::content_commands::remove_video,
            commands::content_commands::revalidate_video,
            commands::content_commands::regenerate_video_thumbnail,
            commands::content_commands::reveal_video_in_file_manager,
            commands::source_commands::list_sources,
            commands::source_commands::create_source,
            commands::source_commands::update_source,
            commands::source_commands::delete_source,
            commands::source_commands::scan_source_now,
            commands::channel_commands::list_channels,
            commands::channel_commands::create_channel,
            commands::import_commands::import_files,
            commands::import_commands::import_folder,
        ]);

    let builder = commands::media_protocol::register(builder);

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        // Graceful shutdown (section 86): stop accepting new filesystem
        // events and let in-flight ingestion jobs finish naturally (they
        // hold their own semaphore permits and aren't force-cancelled).
        if let tauri::RunEvent::Exit = event {
            let state = app_handle.state::<AppState>();
            state.job_runner.shutdown();
        }
    });
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
    let video_repo: Arc<dyn VideoRepository> = Arc::new(SqliteVideoRepository::new(pool.clone()));
    let source_repo: Arc<dyn VideoSourceRepository> =
        Arc::new(SqliteVideoSourceRepository::new(pool.clone()));
    let duplicate_repo: Arc<dyn DuplicateMatchRepository> =
        Arc::new(SqliteDuplicateMatchRepository::new(pool.clone()));
    let job_repo: Arc<dyn JobRepository> = Arc::new(SqliteJobRepository::new(pool.clone()));
    let channel_repo: Arc<dyn ChannelRepository> =
        Arc::new(SqliteChannelRepository::new(pool.clone()));

    let media_service: Arc<dyn MediaService> = Arc::new(FfmpegMediaService::new());
    let probe_service: Arc<dyn MediaProbeService> = Arc::new(FfprobeMediaProbeService::new());
    let thumbnail_service: Arc<dyn ThumbnailService> = Arc::new(FfmpegThumbnailService::new());
    let hash_service: Arc<dyn ContentHashService> = Arc::new(Sha256ContentHashService::new());
    let perceptual_service: Arc<dyn PerceptualHashService> =
        Arc::new(DHashPerceptualHashService::new());

    let activity_service = Arc::new(ActivityService::new(activity_repo.clone()));
    let workspace_service = Arc::new(WorkspaceService::new(workspace_repo, activity_repo));
    let settings_service = Arc::new(SettingsService::new(settings_repo));
    let notification_service = Arc::new(NotificationService::new(notification_repo));
    let media_status_service = Arc::new(MediaStatusService::new(media_service));
    let source_service = Arc::new(SourceService::new(
        source_repo.clone(),
        video_repo.clone(),
        activity_service.clone(),
    ));
    let channel_service = Arc::new(ChannelService::new(channel_repo));

    let ingestion = Arc::new(MediaIngestionService::new(
        video_repo.clone(),
        duplicate_repo.clone(),
        probe_service,
        thumbnail_service,
        hash_service,
        perceptual_service,
        paths.clone(),
    ));
    let content_service = Arc::new(ContentService::new(
        video_repo.clone(),
        duplicate_repo,
        ingestion.clone(),
        activity_service.clone(),
    ));

    let (watcher, watch_events) = FolderWatcherService::start();
    let watcher = Arc::new(watcher);

    let job_runner = Arc::new(JobRunner::new(
        job_repo,
        video_repo.clone(),
        source_repo,
        ingestion,
        activity_service.clone(),
        notification_service.clone(),
        watcher,
    ));

    let recovered = job_runner.recover_orphaned_jobs().await;
    if recovered > 0 {
        tracing::warn!(
            count = recovered,
            "recovered jobs orphaned by a previous shutdown"
        );
    }

    job_runner.clone().spawn_watch_dispatch_loop(watch_events);

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

    // Folder sources only exist once a workspace does (first launch shows
    // onboarding first) — reconciliation/watching for a freshly created
    // workspace starts from `create_workspace`'s own source instead.
    if let Some(workspace) = workspace_service.get_current().await? {
        source_service
            .ensure_manual_import_source(workspace.id)
            .await?;
        job_runner.startup_reconciliation(workspace.id).await;
        job_runner
            .clone()
            .spawn_periodic_reconciliation(workspace.id, PERIODIC_RECONCILIATION_INTERVAL);
    }

    Ok(AppState {
        workspace_service,
        settings_service,
        activity_service,
        media_status_service,
        notification_service,
        content_service,
        source_service,
        channel_service,
        job_runner,
        video_repo,
        paths,
    })
}
