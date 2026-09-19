//! SQLite implementations of the `domain::ports::repositories` (and
//! `jobs::JobRepository`) traits. This is the only place in the codebase
//! allowed to write SQL.

mod sqlite_activity_repository;
mod sqlite_analytics_repository;
mod sqlite_channel_repository;
mod sqlite_duplicate_match_repository;
mod sqlite_hashtag_set_repository;
mod sqlite_job_repository;
mod sqlite_metadata_template_repository;
mod sqlite_notification_repository;
mod sqlite_platform_account_repository;
mod sqlite_provider_rate_state_repository;
mod sqlite_publication_attempt_repository;
mod sqlite_publication_consent_repository;
mod sqlite_publication_repository;
mod sqlite_queue_item_repository;
mod sqlite_schedule_exception_repository;
mod sqlite_schedule_slot_repository;
mod sqlite_settings_repository;
mod sqlite_upload_session_repository;
mod sqlite_video_repository;
mod sqlite_video_source_repository;
mod sqlite_workspace_repository;

pub use sqlite_activity_repository::SqliteActivityRepository;
pub use sqlite_analytics_repository::SqliteAnalyticsRepository;
pub use sqlite_channel_repository::SqliteChannelRepository;
pub use sqlite_duplicate_match_repository::SqliteDuplicateMatchRepository;
pub use sqlite_hashtag_set_repository::SqliteHashtagSetRepository;
pub use sqlite_job_repository::SqliteJobRepository;
pub use sqlite_metadata_template_repository::SqliteMetadataTemplateRepository;
pub use sqlite_notification_repository::SqliteNotificationRepository;
pub use sqlite_platform_account_repository::SqlitePlatformAccountRepository;
pub use sqlite_provider_rate_state_repository::SqliteProviderRateStateRepository;
pub use sqlite_publication_attempt_repository::SqlitePublicationAttemptRepository;
pub use sqlite_publication_consent_repository::SqlitePublicationConsentRepository;
pub use sqlite_publication_repository::SqlitePublicationRepository;
pub use sqlite_queue_item_repository::SqliteQueueItemRepository;
pub use sqlite_schedule_exception_repository::SqliteScheduleExceptionRepository;
pub use sqlite_schedule_slot_repository::SqliteScheduleSlotRepository;
pub use sqlite_settings_repository::SqliteSettingsRepository;
pub use sqlite_upload_session_repository::SqliteUploadSessionRepository;
pub use sqlite_video_repository::SqliteVideoRepository;
pub use sqlite_video_source_repository::SqliteVideoSourceRepository;
pub use sqlite_workspace_repository::SqliteWorkspaceRepository;

use chrono::{DateTime, Utc};

/// Shared helper: SQLite stores timestamps as RFC 3339 UTC strings; this
/// centralizes the (infallible-in-practice, defaulting-to-now-on-corruption)
/// parse used by every repository.
pub(crate) fn parse_dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
