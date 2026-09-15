use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Info => "info",
            NotificationType::Success => "success",
            NotificationType::Warning => "warning",
            NotificationType::Error => "error",
        }
    }
}

impl std::str::FromStr for NotificationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(NotificationType::Info),
            "success" => Ok(NotificationType::Success),
            "warning" => Ok(NotificationType::Warning),
            "error" => Ok(NotificationType::Error),
            other => Err(format!("unknown notification type: {other}")),
        }
    }
}

/// An in-app notification surfaced in the Notification Center. Distinct
/// from [`super::activity_event::ActivityEvent`]: activity events are a
/// full audit trail, notifications are the (dismissible, read/unread)
/// subset the user is expected to act on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub notification_type: NotificationType,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub created_at: DateTime<Utc>,
}

impl Notification {
    pub fn new(
        notification_type: NotificationType,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id: None,
            notification_type,
            title: title.into(),
            message: message.into(),
            read: false,
            created_at: Utc::now(),
        }
    }
}
