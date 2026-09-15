use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Broad grouping for [`ActivityEvent`]s, used for filtering in the Activity
/// screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCategory {
    System,
    Content,
    Publication,
    Platform,
    Warning,
    Error,
}

impl ActivityCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityCategory::System => "system",
            ActivityCategory::Content => "content",
            ActivityCategory::Publication => "publication",
            ActivityCategory::Platform => "platform",
            ActivityCategory::Warning => "warning",
            ActivityCategory::Error => "error",
        }
    }
}

impl std::str::FromStr for ActivityCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "system" => Ok(ActivityCategory::System),
            "content" => Ok(ActivityCategory::Content),
            "publication" => Ok(ActivityCategory::Publication),
            "platform" => Ok(ActivityCategory::Platform),
            "warning" => Ok(ActivityCategory::Warning),
            "error" => Ok(ActivityCategory::Error),
            other => Err(format!("unknown activity category: {other}")),
        }
    }
}

/// Severity of an [`ActivityEvent`], independent of its category (a
/// `Publication` event can be `Info` or `Error`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl ActivityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityLevel::Info => "info",
            ActivityLevel::Success => "success",
            ActivityLevel::Warning => "warning",
            ActivityLevel::Error => "error",
        }
    }
}

impl std::str::FromStr for ActivityLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(ActivityLevel::Info),
            "success" => Ok(ActivityLevel::Success),
            "warning" => Ok(ActivityLevel::Warning),
            "error" => Ok(ActivityLevel::Error),
            other => Err(format!("unknown activity level: {other}")),
        }
    }
}

/// A single, immutable, locally recorded application event. This is the
/// backing data for the Activity screen and, later, for diagnosing
/// publication failures across jobs/retries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub category: ActivityCategory,
    pub level: ActivityLevel,
    pub message: String,
    pub metadata_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ActivityEvent {
    pub fn new(
        category: ActivityCategory,
        level: ActivityLevel,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id: None,
            category,
            level,
            message: message.into(),
            metadata_json: None,
            created_at: Utc::now(),
        }
    }
}
