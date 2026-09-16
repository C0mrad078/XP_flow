/// Kwai's non-secret app id (section 58-61) — needed directly by the
/// desktop for the `open.kuaishou.com` upload/publish calls, which take
/// `app_id` as a plain request parameter rather than deriving identity
/// from the bearer token the way TikTok's API does (section 14/88: video
/// bytes and the publish call both go Desktop → Kwai directly, never
/// through the broker). `app_secret` never appears here; it lives only
/// in the Auth Broker's own environment (`KWAI_APP_SECRET`), the same
/// split as TikTok's `client_key`/`client_secret`.
#[derive(Debug, Clone)]
pub struct KwaiPublishConfig {
    pub app_id: String,
}

impl KwaiPublishConfig {
    pub fn resolve() -> Option<Self> {
        let app_id = std::env::var("KWAI_APP_ID").ok()?;
        if app_id.trim().is_empty() {
            return None;
        }
        Some(Self { app_id })
    }
}
