use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::platform::Platform;
use super::publication::{Publication, PublicationStatus};
use super::video_status::VideoPriority;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueSort {
    /// Manual position for unscheduled items, `scheduled_at` for scheduled
    /// ones — the natural operator ordering (section 15).
    QueueOrder,
    PriorityDesc,
    NewestFirst,
    OldestFirst,
}

/// Composable filter/sort/pagination input for the Queue's List view
/// (section 34-36), mirroring `VideoListQuery`'s shape so the query stays
/// entirely database-side.
#[derive(Debug, Clone)]
pub struct PublicationListQuery {
    pub workspace_id: Uuid,
    pub search: Option<String>,
    pub channel_id: Option<Uuid>,
    pub platform_account_id: Option<Uuid>,
    pub platform: Option<Platform>,
    pub priority: Option<VideoPriority>,
    pub statuses: Option<Vec<PublicationStatus>>,
    pub requires_attention: bool,
    pub sort: QueueSort,
    pub page: i64,
    pub page_size: i64,
}

impl PublicationListQuery {
    pub fn new(workspace_id: Uuid) -> Self {
        Self {
            workspace_id,
            search: None,
            channel_id: None,
            platform_account_id: None,
            platform: None,
            priority: None,
            statuses: None,
            requires_attention: false,
            sort: QueueSort::QueueOrder,
            page: 0,
            page_size: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicationPage {
    pub items: Vec<Publication>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

/// One publication placed on the calendar, with its UTC `scheduled_at`
/// already resolved into the workspace's local wall-clock date/time
/// (section 41-45) so the frontend never has to do timezone math itself.
#[derive(Debug, Clone, Serialize)]
pub struct CalendarPublication {
    pub publication: Publication,
    pub local_date: NaiveDate,
    pub local_time: String,
}

#[derive(Debug, Clone, Copy)]
pub struct UtcRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}
