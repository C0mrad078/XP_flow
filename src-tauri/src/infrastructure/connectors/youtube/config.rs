/// Non-secret (client_id) and Google-installed-app (client_secret — see
/// module docs) configuration for the YouTube connector (section 62): the
/// client id is safe to show in an Advanced/developer settings screen;
/// Google's own OAuth documentation for "Desktop app" client types
/// explicitly does not treat this client_secret as confidential the way a
/// web-server client's would be — PKCE (section 6) is YouTube's real
/// security boundary, which is exactly why YouTube, alone among the three
/// providers, never needs the Auth Broker (section 5).
///
/// XP FLOW ships with its own Google OAuth Desktop client id baked in
/// (`DEFAULT_CLIENT_ID`) so a normal user never opens Google Cloud
/// Console themselves — connecting YouTube is just "Continue with
/// Google." A `YOUTUBE_CLIENT_ID` environment override still exists for
/// anyone building/testing against their own Google Cloud project; it
/// takes precedence over the built-in id when set to a non-empty value.
/// `resolve()` therefore always succeeds — YouTube is never unavailable
/// for lack of configuration.
#[derive(Debug, Clone)]
pub struct YouTubeAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
}

/// XP FLOW's own Google OAuth Desktop Client id. Public/non-secret by
/// Google's own installed-app model — safe to embed in source, the
/// compiled binary, and the frontend bundle. No client_secret accompanies
/// it: the desktop flow is PKCE-only (section 2 of the OAuth
/// configuration task) and never sends a confidential value to Google.
const DEFAULT_CLIENT_ID: &str =
    "877785202982-1k56b1oin20n0n3m1b1t9jgrlmkb3juq.apps.googleusercontent.com";

impl YouTubeAuthConfig {
    pub fn resolve() -> Self {
        let client_id = std::env::var("YOUTUBE_CLIENT_ID")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CLIENT_ID.to_string());
        // Never required by the desktop PKCE flow (`api_client.rs` only
        // sends it if present at all) — this override exists solely for
        // a developer whose own test project happens to be a client type
        // that still issues one.
        let client_secret = std::env::var("YOUTUBE_CLIENT_SECRET")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self {
            client_id,
            client_secret,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // `resolve()` reads process-wide env vars; cargo runs tests in this
    // file concurrently by default, so every test here holds this lock
    // for its duration to avoid racing another test's set/remove.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn resolves_to_the_built_in_client_id_with_no_environment_configured() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("YOUTUBE_CLIENT_ID");
            std::env::remove_var("YOUTUBE_CLIENT_SECRET");
        }
        let config = YouTubeAuthConfig::resolve();
        assert_eq!(config.client_id, DEFAULT_CLIENT_ID);
        assert!(config.client_secret.is_none());
    }

    #[test]
    fn a_non_empty_environment_override_wins_over_the_built_in_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var(
                "YOUTUBE_CLIENT_ID",
                "dev-project-client-id.apps.googleusercontent.com",
            );
        }
        let config = YouTubeAuthConfig::resolve();
        assert_eq!(
            config.client_id,
            "dev-project-client-id.apps.googleusercontent.com"
        );
        unsafe {
            std::env::remove_var("YOUTUBE_CLIENT_ID");
        }
    }

    #[test]
    fn an_empty_environment_override_is_ignored_in_favor_of_the_built_in_id() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("YOUTUBE_CLIENT_ID", "   ");
        }
        let config = YouTubeAuthConfig::resolve();
        assert_eq!(config.client_id, DEFAULT_CLIENT_ID);
        unsafe {
            std::env::remove_var("YOUTUBE_CLIENT_ID");
        }
    }
}
