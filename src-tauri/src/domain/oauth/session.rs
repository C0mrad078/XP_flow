use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use uuid::Uuid;

use crate::domain::platform::Platform;

use super::pkce::Pkce;

/// How long an in-progress authorization attempt stays valid (section 46 —
/// "OAuth flows must have finite timeouts"). Generous enough for a user to
/// actually complete a browser login, short enough that a stale session
/// can't be replayed hours later.
pub const AUTH_SESSION_TTL: Duration = Duration::minutes(10);

/// A cryptographically random, single-use anti-forgery token (section 12).
pub fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Live, in-progress authorization attempt. Deliberately **in-memory
/// only** (section 42: "prefer memory for short flows" — this session's
/// entire lifetime is bounded by `AUTH_SESSION_TTL` and by the desktop
/// process being open to begin with, so persisting it to disk would only
/// add a place for `code_verifier` to leak without buying any real crash
/// recovery — if the app dies mid-flow, "click Connect again" is a
/// perfectly fine recovery path for a sub-10-minute action). Never
/// serialized, never logged, never written to SQLite or Activity.
#[derive(Debug, Clone)]
pub struct AuthSession {
    pub id: Uuid,
    pub platform: Platform,
    pub workspace_id: Uuid,
    pub channel_id: Uuid,
    pub state: String,
    pub pkce: Pkce,
    pub redirect_uri: String,
    /// Set once the desktop has handed the flow to the Auth Broker
    /// (TikTok/Kwai) — the opaque session id the desktop polls for
    /// completion (section 18/23).
    pub broker_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AuthSession {
    pub fn new(
        platform: Platform,
        workspace_id: Uuid,
        channel_id: Uuid,
        pkce: Pkce,
        redirect_uri: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            platform,
            workspace_id,
            channel_id,
            state: generate_state(),
            pkce,
            redirect_uri,
            broker_session_id: None,
            created_at: now,
            expires_at: now + AUTH_SESSION_TTL,
        }
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// Constant-time-ish comparison isn't strictly required here (the
    /// state token is not the secret — the PKCE verifier is — but a
    /// straightforward equality check on a per-request, single-use token
    /// is the standard and sufficient mitigation), just correctness: the
    /// callback's `state` must match this session's exactly.
    pub fn matches_state(&self, candidate: &str) -> bool {
        self.state == candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::oauth::pkce::generate_pkce;

    fn sample_session() -> AuthSession {
        AuthSession::new(
            Platform::YouTube,
            Uuid::new_v4(),
            Uuid::new_v4(),
            generate_pkce(),
            "http://127.0.0.1:49183/oauth/youtube/callback".to_string(),
        )
    }

    #[test]
    fn state_tokens_are_unique_per_session() {
        let a = sample_session();
        let b = sample_session();
        assert_ne!(a.state, b.state);
    }

    #[test]
    fn session_is_not_expired_immediately_after_creation() {
        let session = sample_session();
        assert!(!session.is_expired(Utc::now()));
    }

    #[test]
    fn session_expires_after_its_ttl() {
        let session = sample_session();
        assert!(session.is_expired(session.created_at + AUTH_SESSION_TTL + Duration::seconds(1)));
    }

    #[test]
    fn state_matching_rejects_a_different_token() {
        let session = sample_session();
        assert!(session.matches_state(&session.state));
        assert!(!session.matches_state("not-the-real-state"));
    }
}
