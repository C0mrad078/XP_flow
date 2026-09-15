use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A one-off override to a channel's recurring [`super::schedule_slot::ScheduleSlot`]
/// pattern for a specific calendar date (section 17's "exceptions").
///
/// Design decision: only "skip this date" (`Skip`) is implemented in Phase
/// 3. A full custom-schedule-for-one-date system (different slots just for
/// that day) is deferred — it is a materially bigger feature (its own slot
/// list, its own conflict rules) with no operator-facing urgency yet, and
/// the spec explicitly allows implementing only "practical basic exception
/// management". `Skip` alone already covers the common real case (holidays,
/// planned days off) cleanly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleExceptionKind {
    Skip,
}

impl ScheduleExceptionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleExceptionKind::Skip => "skip",
        }
    }
}

impl std::str::FromStr for ScheduleExceptionKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "skip" => Ok(ScheduleExceptionKind::Skip),
            other => Err(format!("unknown schedule exception kind: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleException {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub date: NaiveDate,
    pub kind: ScheduleExceptionKind,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ScheduleException {
    pub fn new_skip(channel_id: Uuid, date: NaiveDate, reason: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel_id,
            date,
            kind: ScheduleExceptionKind::Skip,
            reason,
            created_at: Utc::now(),
        }
    }
}
