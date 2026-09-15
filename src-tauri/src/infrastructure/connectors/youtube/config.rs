/// Non-secret (client_id) and Google-installed-app (client_secret — see
/// module docs) configuration for the YouTube connector (section 62): the
/// client id is safe to show in an Advanced/developer settings screen;
/// Google's own OAuth documentation for "Desktop app" client types
/// explicitly does not treat this client_secret as confidential the way a
/// web-server client's would be — PKCE (section 6) is YouTube's real
/// security boundary, which is exactly why YouTube, alone among the three
/// providers, never needs the Auth Broker (section 5).
#[derive(Debug, Clone)]
pub struct YouTubeAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
}

impl YouTubeAuthConfig {
    pub fn resolve() -> Option<Self> {
        let client_id = std::env::var("YOUTUBE_CLIENT_ID").ok()?;
        if client_id.trim().is_empty() {
            return None;
        }
        let client_secret = std::env::var("YOUTUBE_CLIENT_SECRET")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Some(Self {
            client_id,
            client_secret,
        })
    }
}
