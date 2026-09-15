use chrono::{DateTime, Utc};

/// Raw OAuth credential material for a provider XP FLOW talks to
/// directly (YouTube only — section 87). Never serialized to SQLite;
/// exists only long enough to be written into `SecureStorage` and dropped.
/// `Debug` is intentionally not derived with field values — see
/// `infrastructure::auth::redact` for the shared redaction utility this
/// type's `fmt::Debug` impl defers to.
#[derive(Clone)]
pub struct LocalCredential {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

impl std::fmt::Debug for LocalCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalCredential")
            .field("access_token", &"[redacted]")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

/// What a successful authorization or a `get_profile`/`validate_connection`
/// call returns: the provider's own account identity plus whatever XP FLOW
/// needs to persist on the `PlatformAccount` row (section 9/29). Never
/// carries a raw token — those stay in `LocalCredential` (YouTube) or
/// behind the broker's opaque `provider_connection_id` (TikTok/Kwai).
#[derive(Debug, Clone)]
pub struct ConnectedIdentity {
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub username_or_handle: Option<String>,
    pub avatar_url: Option<String>,
    pub granted_scopes: Vec<String>,
    pub access_expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    /// `Some` only for brokered providers (TikTok/Kwai) — `None` for
    /// YouTube, which never talks to the Auth Broker.
    pub provider_connection_id: Option<String>,
    /// `Some` only for YouTube — `None` for brokered providers, whose
    /// tokens never leave the broker.
    pub local_credential: Option<LocalCredential>,
}

/// Result of a background/foreground token refresh (section 37) — the
/// subset of `ConnectedIdentity` that can change on refresh. Provider
/// identity itself never changes as a side effect of a refresh.
#[derive(Debug, Clone)]
pub struct RefreshedCredentials {
    pub access_expires_at: Option<DateTime<Utc>>,
    pub refresh_expires_at: Option<DateTime<Utc>>,
    pub local_credential: Option<LocalCredential>,
}
