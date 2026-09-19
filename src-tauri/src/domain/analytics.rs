use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::platform::Platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsAvailability {
    Supported,
    Unsupported,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsCapabilities {
    pub platform: Platform,
    pub publication_views: bool,
    pub publication_likes: bool,
    pub publication_comments: bool,
    pub publication_shares: bool,
    pub channel_followers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationMetricSnapshot {
    pub id: Uuid,
    pub publication_id: Uuid,
    pub provider: Platform,
    pub captured_at: DateTime<Utc>,
    pub views: Option<i64>,
    pub likes: Option<i64>,
    pub comments: Option<i64>,
    pub shares: Option<i64>,
    pub availability: AnalyticsAvailability,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMetricSnapshot {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub provider: Platform,
    pub captured_at: DateTime<Utc>,
    pub followers: Option<i64>,
    pub total_views: Option<i64>,
    pub availability: AnalyticsAvailability,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsSyncState {
    pub platform_account_id: Uuid,
    pub provider: Platform,
    pub last_attempted_at: Option<DateTime<Utc>>,
    pub last_successful_at: Option<DateTime<Utc>>,
    pub next_allowed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}
