use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::capability::Capability;
use super::platform::Platform;

/// Typed lifecycle for a `PlatformAccount` (section 31). Deliberately not
/// an arbitrary string — every state a caller can be in is enumerated so
/// an invalid one is a compile error, not a typo three files away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformAccountStatus {
    /// No authorization attempt has ever completed for this account row
    /// (a channel with no connection yet, or one that was reset).
    NotConfigured,
    /// An authorization attempt is currently in progress.
    Connecting,
    Connected,
    /// A token refresh is currently in progress.
    Refreshing,
    /// Connected, but missing a permission XP FLOW needs for a feature
    /// the user is trying to use (distinct from not being connected at
    /// all — section 16/33).
    PermissionMissing,
    /// The provider's refresh credential is no longer usable — the user
    /// must go through the authorization flow again.
    ReauthRequired,
    /// The provider (or the user, from the provider's own settings)
    /// revoked this connection.
    Revoked,
    Error,
}

impl PlatformAccountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformAccountStatus::NotConfigured => "not_configured",
            PlatformAccountStatus::Connecting => "connecting",
            PlatformAccountStatus::Connected => "connected",
            PlatformAccountStatus::Refreshing => "refreshing",
            PlatformAccountStatus::PermissionMissing => "permission_missing",
            PlatformAccountStatus::ReauthRequired => "reauth_required",
            PlatformAccountStatus::Revoked => "revoked",
            PlatformAccountStatus::Error => "error",
        }
    }
}

impl std::str::FromStr for PlatformAccountStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "not_configured" => PlatformAccountStatus::NotConfigured,
            "connecting" => PlatformAccountStatus::Connecting,
            "connected" => PlatformAccountStatus::Connected,
            "refreshing" => PlatformAccountStatus::Refreshing,
            "permission_missing" => PlatformAccountStatus::PermissionMissing,
            "reauth_required" => PlatformAccountStatus::ReauthRequired,
            "revoked" => PlatformAccountStatus::Revoked,
            "error" => PlatformAccountStatus::Error,
            other => return Err(format!("unknown platform account status: {other}")),
        })
    }
}

/// A single platform's connected (or not-yet-connected) account for a
/// [`super::channel::Channel`]. No OAuth secret ever lives here or in
/// SQLite (section 29): YouTube's tokens live behind the OS keychain
/// (`SecureStorage`, keyed by this row's `id`); TikTok/Kwai's live behind
/// the Auth Broker, referenced only by the opaque `provider_connection_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAccount {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub channel_id: Uuid,
    pub platform: Platform,

    /// The provider's own stable identifier for this account (e.g. a
    /// YouTube channel id) — authoritative identity, never the display
    /// name (section 9).
    pub provider_account_id: Option<String>,
    /// Opaque Auth Broker connection reference for brokered providers
    /// (TikTok/Kwai). `None` for YouTube, which never talks to the broker.
    pub provider_connection_id: Option<String>,

    pub display_name: Option<String>,
    pub username_or_handle: Option<String>,
    pub avatar_url: Option<String>,

    pub status: PlatformAccountStatus,
    pub granted_scopes: Vec<String>,
    /// Cached projection of `granted_scopes` through
    /// `capability::map_scopes_to_capabilities`, recomputed every time
    /// scopes change — persisted so a capability check never has to
    /// recompute the mapping (section 29/34).
    pub capabilities: Vec<Capability>,

    /// Whether "Add to Queue" should default to this account when a
    /// channel has more than one account on the same platform (kept from
    /// Phase 3, section 49).
    pub default_target: bool,

    pub access_expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    pub connected_at: Option<DateTime<Utc>>,
    pub last_validated_at: Option<DateTime<Utc>>,
    pub last_refreshed_at: Option<DateTime<Utc>>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PlatformAccount {
    pub fn new(workspace_id: Uuid, channel_id: Uuid, platform: Platform) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            channel_id,
            platform,
            provider_account_id: None,
            provider_connection_id: None,
            display_name: None,
            username_or_handle: None,
            avatar_url: None,
            status: PlatformAccountStatus::NotConfigured,
            granted_scopes: Vec::new(),
            capabilities: Vec::new(),
            default_target: false,
            access_expires_at: None,
            refresh_expires_at: None,
            connected_at: None,
            last_validated_at: None,
            last_refreshed_at: None,
            last_error_code: None,
            last_error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }
}
