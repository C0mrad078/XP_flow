use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Whether a channel currently participates in scheduling (section 23).
/// A paused channel keeps all of its history, schedule slots and queued
/// publications untouched — it is simply skipped by the auto-scheduler and
/// excluded from "next available slot" search until resumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelStatus {
    #[default]
    Active,
    Paused,
}

impl ChannelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelStatus::Active => "active",
            ChannelStatus::Paused => "paused",
        }
    }
}

impl std::str::FromStr for ChannelStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "active" => ChannelStatus::Active,
            "paused" => ChannelStatus::Paused,
            other => return Err(format!("unknown channel status: {other}")),
        })
    }
}

/// A `Channel` groups a niche/brand of content together across platforms.
/// One channel can eventually hold one `PlatformAccount` per platform
/// (YouTube, TikTok, Kwai).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub niche: Option<String>,
    pub description: Option<String>,
    pub status: ChannelStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Channel {
    pub fn new(workspace_id: Uuid, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            name: name.into(),
            niche: None,
            description: None,
            status: ChannelStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == ChannelStatus::Active
    }
}
