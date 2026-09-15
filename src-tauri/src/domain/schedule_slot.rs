use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::{DomainError, DomainResult};
use super::platform::Platform;

/// A recurring publishing time-slot for a channel (e.g. "every Tuesday at
/// 18:00"), consumed by the scheduler to auto-place queued publications
/// (section 14-17).
///
/// Design decision (section 17): `platform` is `Option<Platform>` rather
/// than a required column plus a separate "applies to all platforms" flag
/// or table. `None` means the slot is a *channel-default* slot that applies
/// to any platform that does not have its own platform-specific slots on
/// that weekday; `Some(p)` means the slot only applies to publications on
/// platform `p`. This reuses the single `schedule_slots` table for both
/// cases instead of inventing a second schema concept (section 7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSlot {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub platform: Option<Platform>,
    /// ISO-8601 weekday, 0 = Monday .. 6 = Sunday.
    pub day_of_week: i32,
    /// Local wall-clock time (workspace timezone) in `HH:MM` (24h) format.
    pub time_of_day: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScheduleSlot {
    pub fn new(
        channel_id: Uuid,
        platform: Option<Platform>,
        day_of_week: i32,
        time_of_day: impl Into<String>,
    ) -> DomainResult<Self> {
        let time_of_day = time_of_day.into();
        Self::validate(day_of_week, &time_of_day)?;
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            channel_id,
            platform,
            day_of_week,
            time_of_day,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn validate(day_of_week: i32, time_of_day: &str) -> DomainResult<()> {
        if !(0..=6).contains(&day_of_week) {
            return Err(DomainError::InvalidValue {
                field: "day_of_week",
                reason: format!("must be 0..=6, got {day_of_week}"),
            });
        }
        Self::parse_time_of_day(time_of_day)?;
        Ok(())
    }

    /// Parses `HH:MM` into `(hour, minute)`, rejecting anything malformed or
    /// out of range rather than silently defaulting to midnight.
    pub fn parse_time_of_day(time_of_day: &str) -> DomainResult<(u32, u32)> {
        let invalid = || DomainError::InvalidValue {
            field: "time_of_day",
            reason: format!("expected HH:MM (24h), got {time_of_day:?}"),
        };
        let (h, m) = time_of_day.split_once(':').ok_or_else(invalid)?;
        let hour: u32 = h.parse().map_err(|_| invalid())?;
        let minute: u32 = m.parse().map_err(|_| invalid())?;
        if hour > 23 || minute > 59 {
            return Err(invalid());
        }
        Ok((hour, minute))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_channel_default_slot() {
        let slot = ScheduleSlot::new(Uuid::new_v4(), None, 1, "18:00").unwrap();
        assert!(slot.platform.is_none());
        assert!(slot.is_active);
    }

    #[test]
    fn accepts_a_platform_specific_slot() {
        let slot = ScheduleSlot::new(Uuid::new_v4(), Some(Platform::TikTok), 6, "09:30").unwrap();
        assert_eq!(slot.platform, Some(Platform::TikTok));
    }

    #[test]
    fn rejects_an_out_of_range_weekday() {
        assert!(ScheduleSlot::new(Uuid::new_v4(), None, 7, "10:00").is_err());
        assert!(ScheduleSlot::new(Uuid::new_v4(), None, -1, "10:00").is_err());
    }

    #[test]
    fn rejects_malformed_time_strings() {
        assert!(ScheduleSlot::new(Uuid::new_v4(), None, 0, "25:00").is_err());
        assert!(ScheduleSlot::new(Uuid::new_v4(), None, 0, "10:60").is_err());
        assert!(ScheduleSlot::new(Uuid::new_v4(), None, 0, "not-a-time").is_err());
    }

    #[test]
    fn parses_valid_time_of_day() {
        assert_eq!(ScheduleSlot::parse_time_of_day("07:05").unwrap(), (7, 5));
    }
}
