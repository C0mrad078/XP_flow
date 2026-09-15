//! SQLite implementations of the `domain::ports::repositories` traits.
//! This is the only place in the codebase allowed to write SQL.

mod sqlite_activity_repository;
mod sqlite_channel_repository;
mod sqlite_notification_repository;
mod sqlite_publication_repository;
mod sqlite_settings_repository;
mod sqlite_video_repository;
mod sqlite_workspace_repository;

pub use sqlite_activity_repository::SqliteActivityRepository;
pub use sqlite_channel_repository::SqliteChannelRepository;
pub use sqlite_notification_repository::SqliteNotificationRepository;
pub use sqlite_publication_repository::SqlitePublicationRepository;
pub use sqlite_settings_repository::SqliteSettingsRepository;
pub use sqlite_video_repository::SqliteVideoRepository;
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
