use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::platform::Platform;

/// A recurring publishing time-slot for a channel/platform pair (e.g.
/// "TikTok, every Tuesday at 18:00"). The future scheduler will consume
/// these to auto-place queued publications; Phase 1 only models and
/// persists them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSlot {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub platform: Platform,
    /// ISO-8601 weekday, 0 = Monday .. 6 = Sunday.
    pub day_of_week: i32,
    /// Local wall-clock time in `HH:MM` (24h) format.
    pub time_of_day: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
