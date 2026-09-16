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
use application::credential_acquisition_service::CredentialAcquisitionService;
use application::media_ingestion_service::MediaIngestionService;
use application::platform_account_service::PlatformAccountService;
use application::platform_auth_service::PlatformAuthService;
use application::publication_service::PublicationService;
use application::publishing_engine_service::PublishingEngineService;
use application::publishing_readiness_service::PublishingReadinessService;
use application::schedule_slot_service::ScheduleSlotService;
use application::scheduler_service::SchedulerService;
use application::settings_service::SettingsService;
use application::source_service::SourceService;
use application::token_lifecycle_service::TokenLifecycleService;
use application::workspace_service::WorkspaceService;
use domain::activity_event::{ActivityCategory, ActivityLevel};
use domain::platform::Platform;
use domain::ports::hashing::{ContentHashService, PerceptualHashService};
use domain::ports::media_service::{MediaProbeService, MediaService, ThumbnailService};
use domain::ports::platform_auth_provider::PlatformAuthProvider;
use domain::ports::platform_connector::PlatformConnector;
use domain::ports::platform_publisher::PlatformPublisher;
use domain::ports::repositories::{
    ActivityRepository, ChannelRepository, DuplicateMatchRepository, NotificationRepository,
    PlatformAccountRepository, PublicationAttemptRepository, PublicationRepository,
    QueueItemRepository, ScheduleExceptionRepository, ScheduleSlotRepository, SettingsRepository,
    UploadSessionRepository, VideoRepository, VideoSourceRepository, WorkspaceRepository,
};
use infrastructure::auth::{AuthBrokerConfig, BrokerClient};
use infrastructure::connectors::kwai::{
    KwaiAuthProvider, KwaiConnector, KwaiPublishConfig, KwaiUploader,
};
use infrastructure::connectors::tiktok::{
    TikTokAuthConfig, TikTokAuthProvider, TikTokConnector, TikTokUploader,
};
use infrastructure::connectors::youtube::api_client::YouTubeApiClient;
use infrastructure::connectors::youtube::{
    YouTubeAuthConfig, YouTubeAuthProvider, YouTubeConnector, YouTubeUploader,
};
use infrastructure::connectors::{StubAuthProvider, StubConnector};
use infrastructure::hashing::{DHashPerceptualHashService, Sha256ContentHashService};
use infrastructure::media::{FfmpegMediaService, FfmpegThumbnailService, FfprobeMediaProbeService};
use infrastructure::publishing::StubPublisher;
use infrastructure::repositories::{
    SqliteActivityRepository, SqliteChannelRepository, SqliteDuplicateMatchRepository,
    SqliteJobRepository, SqliteNotificationRepository, SqlitePlatformAccountRepository,
    SqlitePublicationAttemptRepository, SqlitePublicationConsentRepository,
    SqlitePublicationRepository, SqliteQueueItemRepository, SqliteScheduleExceptionRepository,
    SqliteScheduleSlotRepository, SqliteSettingsRepository, SqliteUploadSessionRepository,
    SqliteVideoRepository, SqliteVideoSourceRepository, SqliteWorkspaceRepository,
};
use infrastructure::watcher::FolderWatcherService;
use jobs::JobRepository;
use platform::paths::AppPaths;
use platform::secure_storage_keyring::KeyringSecureStorage;
use services::job_runner::JobRunner;
use services::media_status_service::MediaStatusService;
use services::notification_service::NotificationService;
use state::AppState;
use tauri::Manager;

/// Reconciliation backstop interval (section 17) — real-time watching
/// handles almost everything; this exists only to catch filesystem events
/// the OS watcher might have dropped.
const PERIODIC_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Section 39: how often the background sweep checks for platform
/// credentials nearing expiry. Refreshing is cheap and idempotent
/// (section 38's buffer means nothing is actually due most sweeps), so a
/// relatively tight interval costs nothing while keeping the window
/// between "a token could have been refreshed" and "it actually was"
/// small.
const TOKEN_REFRESH_SWEEP_INTERVAL: Duration = Duration::from_secs(15 * 60);
/// Section 85: not a busy-loop — checks for due publications this often.
const PUBLISH_SCAN_INTERVAL: Duration = Duration::from_secs(30);
/// Section 125: processing polls are cheap reads, but still bounded —
/// nowhere near "every second for hours."
const PROCESSING_POLL_INTERVAL: Duration = Duration::from_secs(60);
/// Section 79/131: a conservative default until Settings → Publishing
/// exposes this as a real, user-configurable knob.
const DEFAULT_MAX_CONCURRENT_UPLOADS: usize = 2;

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
            commands::workspace_commands::update_workspace_timezone,
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
            commands::channel_commands::get_channel,
            commands::channel_commands::set_channel_status,
            commands::channel_commands::get_channel_operational_overview,
            commands::import_commands::import_files,
            commands::import_commands::import_folder,
            commands::publication_commands::list_publications,
            commands::publication_commands::get_publication,
            commands::publication_commands::add_to_queue,
            commands::publication_commands::add_to_queue_bulk,
            commands::publication_commands::cancel_publication,
            commands::publication_commands::archive_publication,
            commands::publication_commands::set_publication_priority,
            commands::publication_commands::set_publication_locked,
            commands::publication_commands::reorder_queue,
            commands::publishing_commands::publish_now,
            commands::publishing_commands::retry_publication,
            commands::publishing_commands::get_publication_attempts,
            commands::publishing_commands::get_publication_readiness,
            commands::publishing_commands::record_publication_consent,
            commands::scheduler_commands::schedule_publication,
            commands::scheduler_commands::unschedule_publication,
            commands::scheduler_commands::reschedule_publication_to_date,
            commands::scheduler_commands::auto_schedule_publication,
            commands::scheduler_commands::auto_schedule_channel,
            commands::scheduler_commands::fill_schedule_gaps,
            commands::scheduler_commands::rebuild_channel_schedule,
            commands::scheduler_commands::get_calendar_range,
            commands::scheduler_commands::list_due_publications,
            commands::schedule_slot_commands::list_schedule_slots,
            commands::schedule_slot_commands::create_schedule_slot,
            commands::schedule_slot_commands::set_schedule_slot_active,
            commands::schedule_slot_commands::delete_schedule_slot,
            commands::schedule_slot_commands::copy_schedule_day,
            commands::schedule_slot_commands::add_schedule_exception,
            commands::schedule_slot_commands::remove_schedule_exception,
            commands::schedule_slot_commands::list_schedule_exceptions,
            commands::platform_account_commands::list_platform_accounts,
            commands::platform_account_commands::list_platform_accounts_for_workspace,
            commands::platform_account_commands::set_default_platform_account,
            commands::platform_account_commands::reassign_platform_account_channel,
            commands::platform_auth_commands::begin_platform_connect,
            commands::platform_auth_commands::begin_platform_reconnect,
            commands::platform_auth_commands::poll_platform_connect_status,
            commands::platform_auth_commands::cancel_platform_connect,
            commands::platform_auth_commands::validate_platform_account,
            commands::platform_auth_commands::refresh_platform_account,
            commands::platform_auth_commands::disconnect_platform_account,
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
    let publication_repo: Arc<dyn PublicationRepository> =
        Arc::new(SqlitePublicationRepository::new(pool.clone()));
    let queue_item_repo: Arc<dyn QueueItemRepository> =
        Arc::new(SqliteQueueItemRepository::new(pool.clone()));
    let schedule_slot_repo: Arc<dyn ScheduleSlotRepository> =
        Arc::new(SqliteScheduleSlotRepository::new(pool.clone()));
    let schedule_exception_repo: Arc<dyn ScheduleExceptionRepository> =
        Arc::new(SqliteScheduleExceptionRepository::new(pool.clone()));
    let platform_account_repo: Arc<dyn PlatformAccountRepository> =
        Arc::new(SqlitePlatformAccountRepository::new(pool.clone()));
    let publication_attempt_repo: Arc<dyn PublicationAttemptRepository> =
        Arc::new(SqlitePublicationAttemptRepository::new(pool.clone()));
    let upload_session_repo: Arc<dyn UploadSessionRepository> =
        Arc::new(SqliteUploadSessionRepository::new(pool.clone()));

    let media_service: Arc<dyn MediaService> = Arc::new(FfmpegMediaService::new());
    let probe_service: Arc<dyn MediaProbeService> = Arc::new(FfprobeMediaProbeService::new());
    let thumbnail_service: Arc<dyn ThumbnailService> = Arc::new(FfmpegThumbnailService::new());
    let hash_service: Arc<dyn ContentHashService> = Arc::new(Sha256ContentHashService::new());
    let perceptual_service: Arc<dyn PerceptualHashService> =
        Arc::new(DHashPerceptualHashService::new());

    let activity_service = Arc::new(ActivityService::new(activity_repo.clone()));
    let workspace_service = Arc::new(WorkspaceService::new(workspace_repo.clone(), activity_repo));
    let settings_service = Arc::new(SettingsService::new(settings_repo));
    let notification_service = Arc::new(NotificationService::new(notification_repo));
    let media_status_service = Arc::new(MediaStatusService::new(media_service));
    let source_service = Arc::new(SourceService::new(
        source_repo.clone(),
        video_repo.clone(),
        activity_service.clone(),
    ));
    let channel_service = Arc::new(ChannelService::new(
        channel_repo.clone(),
        publication_repo.clone(),
        schedule_slot_repo.clone(),
        platform_account_repo.clone(),
    ));
    let publication_service = Arc::new(PublicationService::new(
        publication_repo.clone(),
        queue_item_repo.clone(),
        video_repo.clone(),
        activity_service.clone(),
    ));
    let schedule_slot_service = Arc::new(ScheduleSlotService::new(
        schedule_slot_repo.clone(),
        schedule_exception_repo.clone(),
    ));
    let scheduler_service = Arc::new(SchedulerService::new(
        publication_repo.clone(),
        channel_repo.clone(),
        workspace_repo,
        schedule_slot_repo,
        schedule_exception_repo,
        activity_service.clone(),
    ));
    let platform_account_service =
        Arc::new(PlatformAccountService::new(platform_account_repo.clone()));

    // --- Phase 4: platform authentication ------------------------------
    //
    // Every provider degrades independently (section 65): missing
    // developer configuration for one provider (or the Auth Broker being
    // entirely unconfigured) never stops the app from starting — the
    // affected platform's `PlatformAuthProvider`/`PlatformConnector` slot
    // is filled with a `Stub*` that reports `ProviderNotConfigured`
    // instead of the real implementation.
    let secure_storage: Arc<dyn domain::ports::secure_storage::SecureStorage> =
        Arc::new(KeyringSecureStorage::new());
    let broker_config = AuthBrokerConfig::resolve();
    let broker_client = broker_config.as_ref().map(BrokerClient::new);
    if let Some(config) = &broker_config {
        if !config.is_secure_enough() {
            tracing::warn!(
                base_url = config.base_url(),
                "Auth Broker URL is not HTTPS and is not a loopback address — refusing to trust it for TikTok/Kwai"
            );
        }
    }
    let broker_client =
        broker_client.filter(|_| broker_config.as_ref().is_some_and(|c| c.is_secure_enough()));

    let youtube_config = YouTubeAuthConfig::resolve();
    let tiktok_config = TikTokAuthConfig::resolve();
    let kwai_publish_config = KwaiPublishConfig::resolve();
    // `youtube_config` is consumed (by value) in the match below;
    // captured here so the publisher-wiring section further down still
    // knows whether a real `YouTubeUploader` can be registered.
    let youtube_configured = youtube_config.is_some();

    let mut auth_providers: std::collections::HashMap<Platform, Arc<dyn PlatformAuthProvider>> =
        std::collections::HashMap::new();
    let mut connectors: std::collections::HashMap<Platform, Arc<dyn PlatformConnector>> =
        std::collections::HashMap::new();

    match youtube_config {
        Some(config) => {
            auth_providers.insert(
                Platform::YouTube,
                Arc::new(YouTubeAuthProvider::new(config.clone())),
            );
            connectors.insert(
                Platform::YouTube,
                Arc::new(YouTubeConnector::new(
                    YouTubeApiClient::new(config),
                    secure_storage.clone(),
                )),
            );
        }
        None => {
            tracing::warn!(
                "YOUTUBE_CLIENT_ID not set — YouTube connections are unavailable in this build"
            );
            auth_providers.insert(
                Platform::YouTube,
                Arc::new(StubAuthProvider::new(
                    Platform::YouTube,
                    "YOUTUBE_CLIENT_ID is not configured",
                )),
            );
            connectors.insert(
                Platform::YouTube,
                Arc::new(StubConnector::new(
                    Platform::YouTube,
                    "YOUTUBE_CLIENT_ID is not configured",
                )),
            );
        }
    }

    match (&tiktok_config, &broker_client) {
        (Some(config), Some(broker)) => {
            auth_providers.insert(
                Platform::TikTok,
                Arc::new(TikTokAuthProvider::new(config.clone(), broker.clone())),
            );
            connectors.insert(
                Platform::TikTok,
                Arc::new(TikTokConnector::new(broker.clone())),
            );
        }
        _ => {
            let reason = if tiktok_config.is_none() {
                "TIKTOK_CLIENT_KEY is not configured"
            } else {
                "the Auth Broker is not configured or not trusted"
            };
            tracing::warn!(reason, "TikTok connections are unavailable in this build");
            auth_providers.insert(
                Platform::TikTok,
                Arc::new(StubAuthProvider::new(Platform::TikTok, reason)),
            );
            connectors.insert(
                Platform::TikTok,
                Arc::new(StubConnector::new(Platform::TikTok, reason)),
            );
        }
    }

    match &broker_client {
        Some(broker) => {
            auth_providers.insert(
                Platform::Kwai,
                Arc::new(KwaiAuthProvider::new(broker.clone())),
            );
            connectors.insert(Platform::Kwai, Arc::new(KwaiConnector::new(broker.clone())));
        }
        None => {
            let reason = "the Auth Broker is not configured or not trusted";
            tracing::warn!(reason, "Kwai connections are unavailable in this build");
            auth_providers.insert(
                Platform::Kwai,
                Arc::new(StubAuthProvider::new(Platform::Kwai, reason)),
            );
            connectors.insert(
                Platform::Kwai,
                Arc::new(StubConnector::new(Platform::Kwai, reason)),
            );
        }
    }

    // Cloned before `connectors` is moved into `PlatformAuthService`
    // below — `CredentialAcquisitionService` needs its own copy of the
    // same connector instances (each `Arc<dyn PlatformConnector>` is
    // cheap to clone).
    let connectors_for_credentials = connectors.clone();

    let platform_auth_service = Arc::new(PlatformAuthService::new(
        auth_providers,
        connectors,
        platform_account_repo.clone(),
        activity_service.clone(),
        notification_service.clone(),
    ));
    let token_lifecycle_service = Arc::new(TokenLifecycleService::new(
        Arc::new(SqlitePlatformAccountRepository::new(pool.clone())),
        platform_auth_service.clone(),
    ));

    let credential_service = Arc::new(CredentialAcquisitionService::new(
        connectors_for_credentials,
        platform_auth_service.clone(),
    ));
    // Real per-provider `PlatformPublisher` implementations land as each
    // one is built (section 7); until then every platform degrades to a
    // clear `PlatformNotApproved` instead of crashing or silently doing
    // nothing, same graceful-degradation discipline as the auth stubs
    // above.
    let mut publishers: std::collections::HashMap<Platform, Arc<dyn PlatformPublisher>> =
        std::collections::HashMap::new();
    publishers.insert(
        Platform::Kwai,
        if let (Some(config), true) = (kwai_publish_config, broker_client.is_some()) {
            Arc::new(KwaiUploader::new(config)) as Arc<dyn PlatformPublisher>
        } else {
            Arc::new(StubPublisher::new(
                Platform::Kwai,
                "KWAI_APP_ID is not configured or the Auth Broker is unavailable",
            )) as Arc<dyn PlatformPublisher>
        },
    );
    publishers.insert(
        Platform::YouTube,
        if youtube_configured {
            Arc::new(YouTubeUploader::new()) as Arc<dyn PlatformPublisher>
        } else {
            Arc::new(StubPublisher::new(
                Platform::YouTube,
                "YOUTUBE_CLIENT_ID is not configured",
            )) as Arc<dyn PlatformPublisher>
        },
    );
    publishers.insert(
        Platform::TikTok,
        if tiktok_config.is_some() && broker_client.is_some() {
            Arc::new(TikTokUploader::new()) as Arc<dyn PlatformPublisher>
        } else {
            Arc::new(StubPublisher::new(
                Platform::TikTok,
                "TIKTOK_CLIENT_KEY is not configured or the Auth Broker is unavailable",
            )) as Arc<dyn PlatformPublisher>
        },
    );
    let publication_consent_repo = Arc::new(SqlitePublicationConsentRepository::new(pool.clone()));
    let publishing_engine_service = Arc::new(PublishingEngineService::new(
        publication_repo.clone(),
        publication_attempt_repo,
        upload_session_repo,
        video_repo.clone(),
        platform_account_repo.clone(),
        channel_repo.clone(),
        publication_consent_repo.clone(),
        hash_service.clone(),
        credential_service,
        publishers.clone(),
        activity_service.clone(),
        notification_service.clone(),
    ));
    let publishing_readiness_service = Arc::new(PublishingReadinessService::new(
        publication_repo.clone(),
        video_repo.clone(),
        platform_account_repo,
        publication_consent_repo,
        publishers,
    ));

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
        job_runner.clone().spawn_periodic_token_refresh(
            token_lifecycle_service.clone(),
            TOKEN_REFRESH_SWEEP_INTERVAL,
        );

        // Section 88/123: reconcile any claim abandoned by a process that
        // was killed mid-upload *before* the periodic scan below can
        // claim anything new.
        publishing_engine_service
            .recover_interrupted(workspace.id)
            .await;
        job_runner.clone().spawn_periodic_publish_scan(
            publishing_engine_service.clone(),
            workspace.id,
            PUBLISH_SCAN_INTERVAL,
            DEFAULT_MAX_CONCURRENT_UPLOADS,
        );
        job_runner.clone().spawn_periodic_processing_poll(
            publishing_engine_service.clone(),
            workspace.id,
            PROCESSING_POLL_INTERVAL,
        );

        // Queue reconciliation (section 91/113): catches queue/schedule
        // drift (e.g. an orphaned QueueItem left behind by a crash between
        // two writes) on every startup, the same "never silently discard"
        // philosophy as the media reconciliation above.
        match publication_service.reconcile_queue(workspace.id).await {
            Ok(summary) if summary.orphaned_queue_items_removed > 0 => {
                tracing::warn!(
                    removed = summary.orphaned_queue_items_removed,
                    "startup queue reconciliation removed orphaned queue items"
                );
            }
            Ok(_) => {}
            Err(err) => tracing::error!(%err, "startup queue reconciliation failed"),
        }
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
        publication_service,
        scheduler_service,
        schedule_slot_service,
        platform_account_service,
        platform_auth_service,
        token_lifecycle_service,
        publishing_engine_service,
        publishing_readiness_service,
        job_runner,
        video_repo,
        paths,
    })
}
