pub mod kwai;
pub mod tiktok;

use chrono::{DateTime, Utc};

/// What a successful exchange/refresh returns, common to both brokered
/// providers even though their HTTP shapes differ.
#[derive(Debug, Clone)]
pub struct ProviderTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub access_expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    pub granted_scopes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderIdentity {
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub username_or_handle: Option<String>,
    pub avatar_url: Option<String>,
}
