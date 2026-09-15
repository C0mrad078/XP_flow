/// TikTok's non-secret client key (section 13/62) — safe to configure on
/// the desktop. `client_secret` never appears here; it lives only in the
/// Auth Broker's own environment (`TIKTOK_CLIENT_SECRET`, section 27).
#[derive(Debug, Clone)]
pub struct TikTokAuthConfig {
    pub client_key: String,
}

impl TikTokAuthConfig {
    pub fn resolve() -> Option<Self> {
        let client_key = std::env::var("TIKTOK_CLIENT_KEY").ok()?;
        if client_key.trim().is_empty() {
            return None;
        }
        Some(Self { client_key })
    }
}
