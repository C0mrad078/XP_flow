use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::platform::Platform;

/// Connection status of a `PlatformAccount`. No real OAuth flow exists yet
/// (see [`crate::domain::ports::platform_connector::PlatformConnector`]);
/// this models the states the UI needs to render channel health today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    NotConnected,
    Connected,
    AuthExpired,
    Error,
}

impl ConnectionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionStatus::NotConnected => "not_connected",
            ConnectionStatus::Connected => "connected",
            ConnectionStatus::AuthExpired => "auth_expired",
            ConnectionStatus::Error => "error",
        }
    }
}

impl std::str::FromStr for ConnectionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "not_connected" => Ok(ConnectionStatus::NotConnected),
            "connected" => Ok(ConnectionStatus::Connected),
            "auth_expired" => Ok(ConnectionStatus::AuthExpired),
            "error" => Ok(ConnectionStatus::Error),
            other => Err(format!("unknown connection status: {other}")),
        }
    }
}

/// A single platform's connected (or not-yet-connected) account for a
/// [`super::channel::Channel`]. Credentials themselves never live here or in
/// SQLite — they belong behind `SecureStorage`, referenced only by this
/// account's `id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAccount {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub platform: Platform,
    pub display_name: Option<String>,
    pub status: ConnectionStatus,
    pub external_account_id: Option<String>,
    pub connected_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PlatformAccount {
    pub fn new(channel_id: Uuid, platform: Platform) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            channel_id,
            platform,
            display_name: None,
            status: ConnectionStatus::NotConnected,
            external_account_id: None,
            connected_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}
